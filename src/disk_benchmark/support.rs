use anyhow::{Ok, Result};
use std::fs::File;
use std::os::fd::{AsRawFd, FromRawFd};
use std::os::unix::ffi::OsStrExt;
use std::path::PathBuf;

pub fn measure<F, R>(f: F) -> (f64, R)
where
    F: FnOnce() -> R,
{
    let start = std::time::Instant::now();
    let result = f();
    let elapsed = start.elapsed().as_secs_f64();
    return (elapsed, result);
}

#[derive(Debug)]
pub struct Measurement<V> {
    pub value: V,
    pub elapsed: f64,
}

impl Measurement<usize> {
    pub fn measure<F, R>(value: usize, f: F) -> (Measurement<usize>, R)
    where
        F: FnOnce() -> R,
    {
        let start = std::time::Instant::now();
        let result = f();
        let elapsed = start.elapsed().as_secs_f64();
        let measurement = Measurement {
            value: value,
            elapsed: elapsed,
        };
        return (measurement, result);
    }

    pub fn per_sec(&self) -> f64 {
        return self.value as f64 / self.elapsed;
    }
}

pub trait DiskBenchmark {
    fn open_for_benchmarking(path: &PathBuf) -> Result<File>;
    fn set_nocache(&self) -> Result<()>;
}

#[cfg(target_os = "windows")]
impl DiskBenchmark for File {
    fn open_for_benchmarking(path: &PathBuf) -> Result<File> {
        File::options()
            .create(true)
            .read(true)
            .write(true)
            .open(&path)
            .map_err(|e| e.into())
    }

    fn set_nocache(&self) -> Result<()> {
        Ok(())
    }
}

#[cfg(target_os = "linux")]
impl DiskBenchmark for File {
    fn open_for_benchmarking(path: &PathBuf) -> Result<File> {
        log::info!("Opening using posix::open");
        unsafe {
            let fd = libc::open(
                path.as_os_str().as_bytes().as_ptr() as *const u8,
                libc::O_CREAT | libc::O_RDWR | libc::O_DIRECT,
                0o644,
            );
            if fd == -1 {
                return Err(std::io::Error::last_os_error().into());
            }
            Ok(File::from_raw_fd(fd))
        }
    }

    fn set_nocache(&self) -> Result<()> {
        Ok(())
    }
}

#[cfg(target_os = "macos")]
impl DiskBenchmark for File {
    fn open_for_benchmarking(path: &PathBuf) -> Result<File> {
        log::info!("Opening using posix::open");
        unsafe {
            let fd = libc::open(
                path.as_os_str().as_bytes().as_ptr() as *const i8,
                libc::O_CREAT | libc::O_RDWR,
                0o644,
            );
            if fd == -1 {
                return Err(std::io::Error::last_os_error().into());
            }
            Ok(File::from_raw_fd(fd))
        }
    }

    fn set_nocache(&self) -> Result<()> {
        let fd = self.as_raw_fd();
        unsafe {
            log::info!("Setting F_NOCACHE on fd={}", fd);
            let r = libc::fcntl(fd, libc::F_NOCACHE, 1);
            if r == -1 {
                return Err(std::io::Error::last_os_error().into());
            }
            log::info!("Setting F_GLOBAL_NOCACHE on fd={}", fd);
            let r = libc::fcntl(fd, libc::F_GLOBAL_NOCACHE, 1);
            if r == -1 {
                return Err(std::io::Error::last_os_error().into());
            }
        }
        Ok(())
    }
}
