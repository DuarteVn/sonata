//! The Windows named pipe the `sonata` CLI talks to (feature 012, research R3): the Windows
//! counterpart of the `0600` Unix socket. Only the current user (and SYSTEM) may open it, remote
//! clients are refused, and the first instance claims the name so no other process can serve it.

use std::fs::{File, OpenOptions};
use std::io;
use std::os::windows::io::{AsRawHandle, FromRawHandle};
use std::path::{Path, PathBuf};

use windows_sys::Win32::Foundation::{
    CloseHandle, ERROR_PIPE_BUSY, ERROR_PIPE_CONNECTED, HANDLE, HLOCAL, INVALID_HANDLE_VALUE,
    LocalFree,
};
use windows_sys::Win32::Security::Authorization::{
    ConvertStringSecurityDescriptorToSecurityDescriptorW, SDDL_REVISION_1,
};
use windows_sys::Win32::Security::{PSECURITY_DESCRIPTOR, SECURITY_ATTRIBUTES};
use windows_sys::Win32::Storage::FileSystem::{
    FILE_FLAG_FIRST_PIPE_INSTANCE, FlushFileBuffers, PIPE_ACCESS_DUPLEX,
};
use windows_sys::Win32::System::Pipes::{
    ConnectNamedPipe, CreateNamedPipeW, GetNamedPipeServerProcessId, PIPE_READMODE_BYTE,
    PIPE_REJECT_REMOTE_CLIENTS, PIPE_TYPE_BYTE, PIPE_UNLIMITED_INSTANCES, PIPE_WAIT,
    WaitNamedPipeW,
};

use crate::platform::{user_sid, wide};

const BUFFER: u32 = 64 * 1024;
/// How long a client waits for a free instance.
const BUSY_WAIT_MS: u32 = 5_000;

/// `\\.\pipe\sonata-<16 hex>`: the hash covers the user and the data root, so two users or two
/// data roots never share a pipe.
pub fn pipe_name(root: &Path) -> io::Result<PathBuf> {
    use std::hash::{DefaultHasher, Hash, Hasher};
    let mut h = DefaultHasher::new();
    user_sid()?.hash(&mut h);
    root.hash(&mut h);
    Ok(PathBuf::from(format!(
        r"\\.\pipe\sonata-{:016x}",
        h.finish()
    )))
}

/// The SDDL of the pipe: generic all for the user and SYSTEM, nothing else, not inherited.
pub fn pipe_sddl() -> io::Result<String> {
    Ok(format!("D:P(A;;GA;;;{})(A;;GA;;;S-1-5-18)", user_sid()?))
}

struct Descriptor(PSECURITY_DESCRIPTOR);

impl Descriptor {
    fn new() -> io::Result<Self> {
        let text = wide(pipe_sddl()?);
        let mut sd: PSECURITY_DESCRIPTOR = std::ptr::null_mut();
        // SAFETY: `text` is NUL-terminated; `sd` receives a LocalAlloc'd descriptor.
        if unsafe {
            ConvertStringSecurityDescriptorToSecurityDescriptorW(
                text.as_ptr(),
                SDDL_REVISION_1,
                &mut sd,
                std::ptr::null_mut(),
            )
        } == 0
        {
            return Err(io::Error::last_os_error());
        }
        Ok(Self(sd))
    }
}

impl Drop for Descriptor {
    fn drop(&mut self) {
        // SAFETY: allocated by ConvertStringSecurityDescriptorToSecurityDescriptorW.
        unsafe { LocalFree(self.0 as HLOCAL) };
    }
}

// SAFETY: the descriptor is plain memory, only read by CreateNamedPipeW.
unsafe impl Send for Descriptor {}

/// A pipe name that accepts clients one at a time.
pub struct PipeListener {
    name: Vec<u16>,
    descriptor: Descriptor,
    /// The instance waiting for the next client.
    next: HANDLE,
}

// SAFETY: the handle is owned by the listener and used by one thread at a time.
unsafe impl Send for PipeListener {}

impl PipeListener {
    /// Claims `name`. Fails when another process already serves it.
    pub fn bind(name: &Path) -> io::Result<Self> {
        let descriptor = Descriptor::new()?;
        let name = wide(name);
        let next = create_instance(&name, &descriptor, true)?;
        Ok(Self {
            name,
            descriptor,
            next,
        })
    }

    /// Waits for a client and returns its connection.
    pub fn accept(&mut self) -> io::Result<File> {
        let handle = self.next;
        // SAFETY: `handle` is a pipe instance this listener created.
        let ok = unsafe { ConnectNamedPipe(handle, std::ptr::null_mut()) } != 0;
        let err = io::Error::last_os_error();
        if !ok && err.raw_os_error() != Some(ERROR_PIPE_CONNECTED as i32) {
            // A client that came and went leaves the instance unusable: replace it, so the next
            // call waits for a new client instead of failing again.
            let fresh = create_instance(&self.name, &self.descriptor, false)?;
            // SAFETY: the failed instance is owned and closed once.
            unsafe { CloseHandle(handle) };
            self.next = fresh;
            return Err(err);
        }
        self.next = create_instance(&self.name, &self.descriptor, false)?;
        // SAFETY: the connected instance is owned by the returned File from here on.
        Ok(unsafe { File::from_raw_handle(handle as _) })
    }
}

impl Drop for PipeListener {
    fn drop(&mut self) {
        // SAFETY: the waiting instance is owned and closed once.
        unsafe { CloseHandle(self.next) };
    }
}

fn create_instance(name: &[u16], descriptor: &Descriptor, first: bool) -> io::Result<HANDLE> {
    let attrs = SECURITY_ATTRIBUTES {
        nLength: size_of::<SECURITY_ATTRIBUTES>() as u32,
        lpSecurityDescriptor: descriptor.0,
        bInheritHandle: 0,
    };
    let mut open = PIPE_ACCESS_DUPLEX;
    if first {
        open |= FILE_FLAG_FIRST_PIPE_INSTANCE;
    }
    // SAFETY: `name` is NUL-terminated and `attrs` lives for the call.
    let handle = unsafe {
        CreateNamedPipeW(
            name.as_ptr(),
            open,
            PIPE_TYPE_BYTE | PIPE_READMODE_BYTE | PIPE_WAIT | PIPE_REJECT_REMOTE_CLIENTS,
            PIPE_UNLIMITED_INSTANCES,
            BUFFER,
            BUFFER,
            0,
            &attrs,
        )
    };
    if handle == INVALID_HANDLE_VALUE {
        return Err(io::Error::last_os_error());
    }
    Ok(handle)
}

/// Writes out what the client has not read yet, so closing the connection loses nothing.
pub fn flush(conn: &File) {
    // SAFETY: the handle is a connected pipe instance.
    unsafe { FlushFileBuffers(conn.as_raw_handle() as HANDLE) };
}

/// Opens `name` as a client, waiting up to 5 s when every instance is busy.
pub fn connect(name: &Path) -> io::Result<File> {
    for attempt in 0..2 {
        match OpenOptions::new().read(true).write(true).open(name) {
            Err(e) if attempt == 0 && e.raw_os_error() == Some(ERROR_PIPE_BUSY as i32) => {
                let w = wide(name);
                // SAFETY: `w` is NUL-terminated.
                unsafe { WaitNamedPipeW(w.as_ptr(), BUSY_WAIT_MS) };
            }
            other => return other,
        }
    }
    Err(io::Error::from_raw_os_error(ERROR_PIPE_BUSY as i32))
}

/// The process id of the server end of a client connection.
pub fn server_pid(conn: &File) -> io::Result<u32> {
    let mut pid = 0u32;
    // SAFETY: the handle is a connected pipe client.
    if unsafe { GetNamedPipeServerProcessId(conn.as_raw_handle() as HANDLE, &mut pid) } == 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(pid)
}
