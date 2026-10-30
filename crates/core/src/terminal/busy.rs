//! A terminal is busy when a program other than its shell owns the foreground (research R6).
//! Windows has no foreground process group; there a terminal is busy while its shell has a
//! child process other than a console host (feature 012, research R10).

use portable_pty::MasterPty;

/// Console helpers that run next to a shell without being a program the user started.
#[cfg(windows)]
const CONSOLE_HOSTS: [&str; 2] = ["conhost.exe", "openconsole.exe"];

#[cfg(windows)]
pub fn is_busy(_master: &dyn MasterPty, shell_pid: Option<u32>) -> bool {
    shell_pid.is_some_and(|pid| {
        crate::platform::child_processes(pid)
            .iter()
            .any(|(_, name)| !CONSOLE_HOSTS.contains(&name.to_ascii_lowercase().as_str()))
    })
}

#[cfg(unix)]
pub fn is_busy(master: &dyn MasterPty, shell_pid: Option<u32>) -> bool {
    let Some(shell) = shell_pid.and_then(|p| libc::pid_t::try_from(p).ok()) else {
        return false;
    };
    let leader = master.process_group_leader().or_else(|| {
        // SAFETY: tcgetpgrp only reads the terminal's foreground process group.
        master
            .as_raw_fd()
            .map(|fd| unsafe { libc::tcgetpgrp(fd) })
            .filter(|pg| *pg > 0)
    });
    leader.is_some_and(|pg| pg != shell)
}
