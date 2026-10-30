//! Windows bodies of [`super`] (feature 012, research R4, R5, R6, R12, R14).

use std::ffi::{OsStr, OsString};
use std::fs::{self, File, OpenOptions};
use std::io;
use std::os::windows::ffi::{OsStrExt, OsStringExt};
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use windows_sys::Win32::Foundation::{
    CloseHandle, GetLastError, HANDLE, HLOCAL, LocalFree, STILL_ACTIVE,
};
use windows_sys::Win32::Security::Authorization::{
    ConvertSidToStringSidW, ConvertStringSecurityDescriptorToSecurityDescriptorW, SDDL_REVISION_1,
};
use windows_sys::Win32::Security::{
    ACCESS_ALLOWED_ACE, ACE_HEADER, ACL, ACL_SIZE_INFORMATION, AclSizeInformation,
    DACL_SECURITY_INFORMATION, GetAce, GetAclInformation, GetFileSecurityW,
    GetSecurityDescriptorDacl, GetTokenInformation, PROTECTED_DACL_SECURITY_INFORMATION,
    PSECURITY_DESCRIPTOR, SetFileSecurityW, TOKEN_QUERY, TOKEN_USER, TokenUser,
};
use windows_sys::Win32::System::Threading::{
    GetCurrentProcess, GetExitCodeProcess, OpenProcess, OpenProcessToken,
    PROCESS_QUERY_LIMITED_INFORMATION,
};

use super::{Kind, Launch};

// ---- The per-OS API that `linux.rs` and `macos.rs` also provide (feature 011, research R1) ----

pub const DEFAULT_SHELL: &str = "powershell.exe";
pub const MACHINE_NAME: &str = "PC";
pub const GIT_INSTALL_HINT: &str =
    "Git is not installed. Install Git for Windows from https://git-scm.com/download/win";
/// Git for Windows is found on `PATH` or in its install folders (see `lanes::git::discover`).
pub const GIT_CANDIDATES: &[&str] = &[];
pub const IOS_SIMULATORS: bool = false;
pub const BREW_PREFIXES: &[&str] = &[];

pub fn git_shim() -> Option<(&'static Path, bool)> {
    None
}

/// Android Studio's default SDK folder, `%LOCALAPPDATA%\Android\Sdk`.
pub fn android_sdk_defaults(home: &Path) -> Vec<PathBuf> {
    let local = std::env::var_os("LOCALAPPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(|| home.join("AppData").join("Local"));
    vec![local.join("Android").join("Sdk")]
}

/// The emulator writes discovery files under the temp folder.
pub fn emulator_discovery_dirs(
    home: &Path,
    vars: &std::collections::HashMap<String, String>,
) -> Vec<PathBuf> {
    let mut dirs: Vec<PathBuf> = ["TEMP", "TMP"]
        .iter()
        .filter_map(|k| vars.get(*k).filter(|v| !v.is_empty()))
        .map(|t| Path::new(t).join("avd").join("running"))
        .collect();
    let fallback = home
        .join("AppData")
        .join("Local")
        .join("Temp")
        .join("avd")
        .join("running");
    if !dirs.contains(&fallback) {
        dirs.push(fallback);
    }
    dirs.dedup();
    dirs
}

/// The default browser through the URL protocol handler (no shell, so no quoting issues).
pub fn open_url_command(url: &str) -> Launch {
    Launch {
        program: system32("rundll32.exe"),
        args: vec!["url.dll,FileProtocolHandler".into(), url.into()],
    }
}

/// File Explorer with the file selected.
pub fn reveal_command(path: &Path) -> Launch {
    let mut arg = std::ffi::OsString::from("/select,");
    arg.push(path.as_os_str());
    Launch {
        program: windows_dir().join("explorer.exe"),
        args: vec![arg],
    }
}

fn windows_dir() -> PathBuf {
    std::env::var_os("SystemRoot")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(r"C:\Windows"))
}

fn system32(program: &str) -> PathBuf {
    windows_dir().join("System32").join(program)
}

// ---- Shared helpers (feature 012) ----

/// The well-known SID of the local system account.
const SYSTEM_SID: &str = "S-1-5-18";
/// `ACCESS_ALLOWED_ACE_TYPE`.
const ALLOWED_ACE: u8 = 0;
/// Paths this long or longer get the `\\?\` prefix before a Win32 call that does not add it.
const LONG_PATH: usize = 248;

/// A NUL-terminated UTF-16 copy of `s`.
pub fn wide(s: impl AsRef<OsStr>) -> Vec<u16> {
    s.as_ref().encode_wide().chain(std::iter::once(0)).collect()
}

/// A NUL-terminated UTF-16 path, with the `\\?\` prefix when the path is long.
fn wide_path(path: &Path) -> Vec<u16> {
    let s = path.as_os_str();
    if s.len() < LONG_PATH || !path.is_absolute() || s.to_string_lossy().starts_with(r"\\?\") {
        return wide(s);
    }
    let text = s.to_string_lossy();
    let prefixed = match text.strip_prefix(r"\\") {
        Some(unc) => format!(r"\\?\UNC\{unc}"),
        None => format!(r"\\?\{text}"),
    };
    wide(prefixed)
}

/// Frees memory returned by `Convert…` calls.
struct LocalBox(*mut core::ffi::c_void);

impl Drop for LocalBox {
    fn drop(&mut self) {
        if !self.0.is_null() {
            // SAFETY: the pointer came from a Win32 call that allocates with LocalAlloc.
            unsafe { LocalFree(self.0 as HLOCAL) };
        }
    }
}

struct OwnedHandle(HANDLE);

impl Drop for OwnedHandle {
    fn drop(&mut self) {
        if !self.0.is_null() {
            // SAFETY: the handle is owned and closed once.
            unsafe { CloseHandle(self.0) };
        }
    }
}

/// Reads a NUL-terminated UTF-16 string.
///
/// # Safety
/// `p` must point to a NUL-terminated UTF-16 string.
unsafe fn from_wide_ptr(p: *const u16) -> String {
    let mut len = 0;
    // SAFETY: the caller guarantees a terminating NUL.
    while unsafe { *p.add(len) } != 0 {
        len += 1;
    }
    // SAFETY: `len` elements were just read.
    let slice = unsafe { std::slice::from_raw_parts(p, len) };
    OsString::from_wide(slice).to_string_lossy().into_owned()
}

/// The string form of a SID.
///
/// # Safety
/// `sid` must point to a valid SID.
unsafe fn sid_string(sid: *mut core::ffi::c_void) -> io::Result<String> {
    let mut out: *mut u16 = std::ptr::null_mut();
    // SAFETY: `sid` is valid; `out` receives a LocalAlloc'd string.
    if unsafe { ConvertSidToStringSidW(sid, &mut out) } == 0 {
        return Err(io::Error::last_os_error());
    }
    let _free = LocalBox(out.cast());
    // SAFETY: the call returned a NUL-terminated string.
    Ok(unsafe { from_wide_ptr(out) })
}

fn current_user_sid() -> io::Result<String> {
    // SAFETY: the pseudo handle of the current process is always valid.
    token_user_sid(unsafe { GetCurrentProcess() })
}

/// The user SID of another process (for example the server end of a pipe).
pub fn process_user_sid(pid: u32) -> io::Result<String> {
    // SAFETY: OpenProcess returns null on failure; the handle is closed by OwnedHandle.
    let process = OwnedHandle(unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid) });
    if process.0.is_null() {
        return Err(io::Error::last_os_error());
    }
    token_user_sid(process.0)
}

fn token_user_sid(process: HANDLE) -> io::Result<String> {
    let mut token: HANDLE = std::ptr::null_mut();
    // SAFETY: `process` is a valid process handle.
    if unsafe { OpenProcessToken(process, TOKEN_QUERY, &mut token) } == 0 {
        return Err(io::Error::last_os_error());
    }
    let token = OwnedHandle(token);
    let mut len = 0u32;
    // SAFETY: a size query with a null buffer.
    unsafe { GetTokenInformation(token.0, TokenUser, std::ptr::null_mut(), 0, &mut len) };
    // u64 storage keeps the TOKEN_USER aligned.
    let mut buf = vec![0u64; (len as usize).div_ceil(8)];
    // SAFETY: `buf` holds at least `len` bytes.
    if unsafe { GetTokenInformation(token.0, TokenUser, buf.as_mut_ptr().cast(), len, &mut len) }
        == 0
    {
        return Err(io::Error::last_os_error());
    }
    // SAFETY: the buffer now starts with a TOKEN_USER whose SID points inside it.
    let user = unsafe { &*(buf.as_ptr() as *const TOKEN_USER) };
    // SAFETY: the SID is valid while `buf` lives.
    unsafe { sid_string(user.User.Sid) }
}

/// The current user's SID as text (`S-1-5-21-…`).
pub fn user_sid() -> io::Result<&'static str> {
    static SID: OnceLock<Result<String, String>> = OnceLock::new();
    SID.get_or_init(|| current_user_sid().map_err(|e| e.to_string()))
        .as_deref()
        .map_err(|e| io::Error::other(e.clone()))
}

fn sddl(kind: Kind) -> io::Result<String> {
    let user = user_sid()?;
    Ok(match kind {
        Kind::Dir => format!("D:P(A;OICI;FA;;;{user})(A;OICI;FA;;;{SYSTEM_SID})"),
        Kind::File => format!("D:P(A;;FA;;;{user})(A;;FA;;;{SYSTEM_SID})"),
    })
}

pub fn make_private(path: &Path, kind: Kind) -> io::Result<()> {
    let text = wide(sddl(kind)?);
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
    let _free = LocalBox(sd);
    let name = wide_path(path);
    // SAFETY: `name` is NUL-terminated and `sd` is a valid descriptor.
    if unsafe {
        SetFileSecurityW(
            name.as_ptr(),
            DACL_SECURITY_INFORMATION | PROTECTED_DACL_SECURITY_INFORMATION,
            sd,
        )
    } == 0
    {
        return Err(io::Error::last_os_error());
    }
    Ok(())
}

/// The string SIDs of every allow entry in `path`'s access list; `None` for a null list (which
/// grants everyone full access).
pub fn allowed_sids(path: &Path) -> io::Result<Option<Vec<String>>> {
    let name = wide_path(path);
    let mut len = 0u32;
    // SAFETY: a size query with a null buffer.
    unsafe {
        GetFileSecurityW(
            name.as_ptr(),
            DACL_SECURITY_INFORMATION,
            std::ptr::null_mut(),
            0,
            &mut len,
        )
    };
    if len == 0 {
        return Err(io::Error::last_os_error());
    }
    let mut buf = vec![0u64; (len as usize).div_ceil(8)];
    // SAFETY: `buf` holds at least `len` bytes.
    if unsafe {
        GetFileSecurityW(
            name.as_ptr(),
            DACL_SECURITY_INFORMATION,
            buf.as_mut_ptr().cast(),
            len,
            &mut len,
        )
    } == 0
    {
        return Err(io::Error::last_os_error());
    }
    let sd: PSECURITY_DESCRIPTOR = buf.as_mut_ptr().cast();
    let (mut present, mut defaulted) = (0, 0);
    let mut acl: *mut ACL = std::ptr::null_mut();
    // SAFETY: `sd` is the descriptor just read.
    if unsafe { GetSecurityDescriptorDacl(sd, &mut present, &mut acl, &mut defaulted) } == 0 {
        return Err(io::Error::last_os_error());
    }
    if present == 0 || acl.is_null() {
        return Ok(None);
    }
    // SAFETY: `acl` points into `buf`, which lives until the call returns.
    unsafe { acl_sids(acl) }.map(Some)
}

/// The string SIDs of every allow entry in an open handle's access list (for example a pipe).
pub fn handle_allowed_sids(
    handle: &impl std::os::windows::io::AsRawHandle,
) -> io::Result<Option<Vec<String>>> {
    use windows_sys::Win32::Security::Authorization::{GetSecurityInfo, SE_KERNEL_OBJECT};
    let mut acl: *mut ACL = std::ptr::null_mut();
    let mut sd: PSECURITY_DESCRIPTOR = std::ptr::null_mut();
    // SAFETY: `handle` is valid; the descriptor is LocalAlloc'd and freed by LocalBox.
    let err = unsafe {
        GetSecurityInfo(
            handle.as_raw_handle() as HANDLE,
            SE_KERNEL_OBJECT,
            DACL_SECURITY_INFORMATION,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            &mut acl,
            std::ptr::null_mut(),
            &mut sd,
        )
    };
    let _free = LocalBox(sd);
    if err != 0 {
        return Err(io::Error::from_raw_os_error(err as i32));
    }
    if acl.is_null() {
        return Ok(None);
    }
    // SAFETY: `acl` points into `sd`, freed after this call.
    unsafe { acl_sids(acl) }.map(Some)
}

/// # Safety
/// `acl` must point to a valid access list.
unsafe fn acl_sids(acl: *mut ACL) -> io::Result<Vec<String>> {
    let mut info = ACL_SIZE_INFORMATION::default();
    // SAFETY: the caller guarantees a valid list.
    if unsafe {
        GetAclInformation(
            acl,
            (&mut info as *mut ACL_SIZE_INFORMATION).cast(),
            size_of::<ACL_SIZE_INFORMATION>() as u32,
            AclSizeInformation,
        )
    } == 0
    {
        return Err(io::Error::last_os_error());
    }
    let mut sids = Vec::new();
    for i in 0..info.AceCount {
        let mut ace: *mut core::ffi::c_void = std::ptr::null_mut();
        // SAFETY: `i` is below the entry count.
        if unsafe { GetAce(acl, i, &mut ace) } == 0 {
            return Err(io::Error::last_os_error());
        }
        // SAFETY: every entry starts with an ACE_HEADER.
        let header = unsafe { &*(ace as *const ACE_HEADER) };
        if header.AceType != ALLOWED_ACE {
            continue;
        }
        let allowed = ace as *mut ACCESS_ALLOWED_ACE;
        // SAFETY: an allow entry holds its SID at `SidStart`.
        let sid = unsafe { std::ptr::addr_of_mut!((*allowed).SidStart) };
        // SAFETY: the SID lives inside `buf`.
        sids.push(unsafe { sid_string(sid.cast())? });
    }
    Ok(sids)
}

pub fn is_private(path: &Path, _kind: Kind) -> io::Result<bool> {
    let user = user_sid()?;
    Ok(allowed_sids(path)?.is_some_and(|sids| {
        sids.iter()
            .all(|s| s.eq_ignore_ascii_case(user) || s == SYSTEM_SID)
    }))
}

pub fn create_dir_all(path: &Path) -> io::Result<()> {
    fs::create_dir_all(path)
}

pub fn open_private_append(path: &Path) -> io::Result<File> {
    let existed = path.exists();
    let file = OpenOptions::new().create(true).append(true).open(path)?;
    if !existed {
        make_private(path, Kind::File)?;
    }
    Ok(file)
}

fn path_extensions() -> Vec<String> {
    std::env::var("PATHEXT")
        .ok()
        .filter(|v| !v.trim().is_empty())
        .unwrap_or_else(|| ".COM;.EXE;.BAT;.CMD".into())
        .split(';')
        .map(|e| e.trim().to_ascii_lowercase())
        .filter(|e| e.starts_with('.') && e.len() > 1)
        .collect()
}

pub fn is_executable(path: &Path) -> bool {
    let Some(ext) = path.extension().and_then(|e| e.to_str()) else {
        return false;
    };
    let ext = format!(".{}", ext.to_ascii_lowercase());
    path.is_file() && path_extensions().contains(&ext)
}

pub fn executable_names(name: &str) -> Vec<String> {
    if Path::new(name).extension().is_some_and(|e| {
        path_extensions().contains(&format!(".{}", e.to_string_lossy().to_ascii_lowercase()))
    }) {
        return vec![name.to_string()];
    }
    path_extensions()
        .into_iter()
        .map(|ext| format!("{name}{ext}"))
        .collect()
}

pub fn process_alive(pid: u32) -> bool {
    // SAFETY: OpenProcess returns null on failure; the handle is closed by OwnedHandle.
    let handle = OwnedHandle(unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid) });
    if handle.0.is_null() {
        // A process we may not query (another user's) still exists.
        // SAFETY: GetLastError has no preconditions.
        return unsafe { GetLastError() } == windows_sys::Win32::Foundation::ERROR_ACCESS_DENIED;
    }
    let mut code = 0u32;
    // SAFETY: the handle is valid.
    unsafe { GetExitCodeProcess(handle.0, &mut code) != 0 && code == STILL_ACTIVE as u32 }
}

pub fn simplify(path: &Path) -> PathBuf {
    let text = path.as_os_str().to_string_lossy();
    if let Some(rest) = text.strip_prefix(r"\\?\UNC\") {
        if rest.len() + 2 < 260 {
            return PathBuf::from(format!(r"\\{rest}"));
        }
    } else if let Some(rest) = text.strip_prefix(r"\\?\") {
        let b = rest.as_bytes();
        let drive = b.len() >= 3 && b[0].is_ascii_alphabetic() && b[1] == b':' && b[2] == b'\\';
        if drive && rest.len() < 260 {
            return PathBuf::from(rest);
        }
    }
    path.to_path_buf()
}

/// Direct children of `pid`: `(pid, executable name)` (feature 012, research R10).
pub fn child_processes(pid: u32) -> Vec<(u32, String)> {
    use windows_sys::Win32::Foundation::INVALID_HANDLE_VALUE;
    use windows_sys::Win32::System::Diagnostics::ToolHelp::{
        CreateToolhelp32Snapshot, PROCESSENTRY32W, Process32FirstW, Process32NextW,
        TH32CS_SNAPPROCESS,
    };
    // SAFETY: a process snapshot has no preconditions; the handle is closed by OwnedHandle.
    let snap = unsafe { CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0) };
    if snap == INVALID_HANDLE_VALUE {
        return Vec::new();
    }
    let snap = OwnedHandle(snap);
    let mut entry = PROCESSENTRY32W {
        dwSize: size_of::<PROCESSENTRY32W>() as u32,
        ..Default::default()
    };
    let mut out = Vec::new();
    // SAFETY: `entry.dwSize` is set as the API requires.
    let mut ok = unsafe { Process32FirstW(snap.0, &mut entry) } != 0;
    while ok {
        if entry.th32ParentProcessID == pid && entry.th32ProcessID != pid {
            let len = entry
                .szExeFile
                .iter()
                .position(|&c| c == 0)
                .unwrap_or(entry.szExeFile.len());
            out.push((
                entry.th32ProcessID,
                OsString::from_wide(&entry.szExeFile[..len])
                    .to_string_lossy()
                    .into_owned(),
            ));
        }
        // SAFETY: same snapshot and entry.
        ok = unsafe { Process32NextW(snap.0, &mut entry) } != 0;
    }
    out
}
