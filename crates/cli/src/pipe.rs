//! Windows client side of the app's named pipe (feature 012, research R3). Before sending the
//! terminal's token, the client checks that the process serving the pipe runs as the same user.

use std::ffi::OsString;
use std::fs::{File, OpenOptions};
use std::io;
use std::os::windows::ffi::{OsStrExt, OsStringExt};
use std::os::windows::io::AsRawHandle;

use windows_sys::Win32::Foundation::{CloseHandle, ERROR_PIPE_BUSY, HANDLE, HLOCAL, LocalFree};
use windows_sys::Win32::Security::Authorization::ConvertSidToStringSidW;
use windows_sys::Win32::Security::{GetTokenInformation, TOKEN_QUERY, TOKEN_USER, TokenUser};
use windows_sys::Win32::System::Pipes::{GetNamedPipeServerProcessId, WaitNamedPipeW};
use windows_sys::Win32::System::Threading::{
    GetCurrentProcess, OpenProcess, OpenProcessToken, PROCESS_QUERY_LIMITED_INFORMATION,
};

const BUSY_WAIT_MS: u32 = 5_000;

struct Owned(HANDLE);

impl Drop for Owned {
    fn drop(&mut self) {
        if !self.0.is_null() {
            // SAFETY: owned handle, closed once.
            unsafe { CloseHandle(self.0) };
        }
    }
}

fn wide(s: &str) -> Vec<u16> {
    std::ffi::OsStr::new(s)
        .encode_wide()
        .chain(std::iter::once(0))
        .collect()
}

fn token_sid(process: HANDLE) -> io::Result<String> {
    let mut token: HANDLE = std::ptr::null_mut();
    // SAFETY: `process` is a valid process handle.
    if unsafe { OpenProcessToken(process, TOKEN_QUERY, &mut token) } == 0 {
        return Err(io::Error::last_os_error());
    }
    let token = Owned(token);
    let mut len = 0u32;
    // SAFETY: size query.
    unsafe { GetTokenInformation(token.0, TokenUser, std::ptr::null_mut(), 0, &mut len) };
    let mut buf = vec![0u64; (len as usize).div_ceil(8)];
    // SAFETY: `buf` holds `len` bytes.
    if unsafe { GetTokenInformation(token.0, TokenUser, buf.as_mut_ptr().cast(), len, &mut len) }
        == 0
    {
        return Err(io::Error::last_os_error());
    }
    // SAFETY: the buffer starts with a TOKEN_USER.
    let user = unsafe { &*(buf.as_ptr() as *const TOKEN_USER) };
    let mut text: *mut u16 = std::ptr::null_mut();
    // SAFETY: the SID is valid while `buf` lives.
    if unsafe { ConvertSidToStringSidW(user.User.Sid, &mut text) } == 0 {
        return Err(io::Error::last_os_error());
    }
    let mut n = 0;
    // SAFETY: NUL-terminated string from the call.
    while unsafe { *text.add(n) } != 0 {
        n += 1;
    }
    // SAFETY: `n` units were just read.
    let s = OsString::from_wide(unsafe { std::slice::from_raw_parts(text, n) });
    // SAFETY: allocated by ConvertSidToStringSidW.
    unsafe { LocalFree(text as HLOCAL) };
    Ok(s.to_string_lossy().into_owned())
}

fn same_user(conn: &File) -> io::Result<bool> {
    let mut pid = 0u32;
    // SAFETY: `conn` is a connected pipe client.
    if unsafe { GetNamedPipeServerProcessId(conn.as_raw_handle() as HANDLE, &mut pid) } == 0 {
        return Err(io::Error::last_os_error());
    }
    // SAFETY: null on failure; closed by Owned.
    let server = Owned(unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid) });
    if server.0.is_null() {
        return Err(io::Error::last_os_error());
    }
    // SAFETY: the pseudo handle of this process is always valid.
    let me = token_sid(unsafe { GetCurrentProcess() })?;
    Ok(token_sid(server.0)? == me)
}

/// Opens the pipe, waiting up to 5 s for a free instance, and refuses a server owned by
/// another user.
pub fn connect(name: &str) -> Result<File, String> {
    let mut opened = None;
    for attempt in 0..2 {
        match OpenOptions::new().read(true).write(true).open(name) {
            Err(e) if attempt == 0 && e.raw_os_error() == Some(ERROR_PIPE_BUSY as i32) => {
                let w = wide(name);
                // SAFETY: NUL-terminated name.
                unsafe { WaitNamedPipeW(w.as_ptr(), BUSY_WAIT_MS) };
            }
            other => {
                opened = Some(other);
                break;
            }
        }
    }
    let conn = opened
        .unwrap_or_else(|| Err(io::Error::from_raw_os_error(ERROR_PIPE_BUSY as i32)))
        .map_err(|e| format!("Sonata is not reachable ({e}). Is the app running?"))?;
    match same_user(&conn) {
        Ok(true) => Ok(conn),
        Ok(false) => Err("The Sonata channel is owned by another user.".into()),
        Err(e) => Err(format!(
            "Could not check who serves the Sonata channel ({e})."
        )),
    }
}
