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

pub trait DiskBenchmark {
    fn open_for_benchmarking(
        path: &PathBuf,
        no_create: bool,
        no_disable_cache: bool,
    ) -> Result<File>;
    fn set_nocache(&self) -> Result<()>;
}

#[cfg(target_os = "windows")]
impl DiskBenchmark for File {
    fn open_for_benchmarking(
        path: &PathBuf,
        no_create: bool,
        no_disable_cache: bool,
    ) -> Result<File> {
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
    fn open_for_benchmarking(
        path: &PathBuf,
        no_create: bool,
        no_disable_cache: bool,
    ) -> Result<File> {
        log::info!("Opening using posix::open");
        unsafe {
            let mut oflags = libc::O_RDWR;
            if !no_create {
                oflags |= libc::O_CREAT;
            }
            if !no_disable_cache {
                oflags |= libc::O_DIRECT;
            }

            let fd = libc::open(
                path.as_os_str().as_bytes().as_ptr() as *const u8,
                oflags,
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
    fn open_for_benchmarking(path: &PathBuf, no_create: bool, _: bool) -> Result<File> {
        log::info!("Opening using posix::open");
        unsafe {
            let mut oflags = libc::O_RDWR;
            if !no_create {
                oflags |= libc::O_CREAT;
            }

            let fd = libc::open(
                path.as_os_str().as_bytes().as_ptr() as *const i8,
                oflags,
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
