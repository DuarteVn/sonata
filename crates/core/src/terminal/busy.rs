//! A terminal is busy when a program other than its shell owns the foreground (research R6).

use portable_pty::MasterPty;

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
