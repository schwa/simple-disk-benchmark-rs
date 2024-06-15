// #[cfg(target_os = "linux")]
// use nix::fcntl::{splice, SpliceFFlags};
#[cfg(target_os = "linux")]
use std::fs::File;
#[cfg(target_os = "linux")]
use std::os::unix::io::AsRawFd;

#[cfg(target_os = "windows")]
use windows_sys::Win32::System::Threading::CREATE_SUSPENDED;

use std::process::ExitStatus;

/// Used to indicate the result of running a command
#[derive(Debug, Copy, Clone)]
pub struct TimerResult {
    pub time_real: f64,
    pub time_user: f64,
    pub time_system: f64,

    /// The exit status of the process
    pub status: ExitStatus,
}
