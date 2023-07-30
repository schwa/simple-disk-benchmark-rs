use anyhow::{Ok, Result};
use bytesize::ByteSize;
use enum_display_derive::Display;
use indicatif::ProgressBar;
use rand::RngCore;
use std::fmt::Display;
use std::fs::File;
use std::io::{Read, Seek, Write};
use std::os::fd::RawFd;
use std::os::fd::{AsRawFd, FromRawFd};
use std::os::unix::ffi::OsStrExt;
use std::path::PathBuf;
use std::vec;

pub fn prepare_file(path: &PathBuf, file_size: usize) -> Result<File> {
    log::info!(
        "Preparing test file {}, size: {}.",
        path.display(),
        file_size
    );
    let mut buffer = vec![0; file_size];
    let mut rng = rand::thread_rng();
    rng.fill_bytes(&mut buffer);

    if path.exists() {
        std::fs::remove_file(path)?;
    }

    let mut file = File::open_for_benchmarking(&path)?;
    file.set_nocache()?;
    log::info!(
        "Writing {} bytes to {}",
        ByteSize(file_size as u64),
        path.display()
    );

    let (elapsed, result) = measure(|| {
        file.write(&buffer)?;
        file.sync_all()?;
        return Ok(());
    });
    log::info!(
        "Wrote {} in {:.3}s ({}/s)",
        ByteSize(file_size as u64),
        elapsed,
        ByteSize((file_size as f64 / elapsed) as u64)
    );
    result?;

    return Ok(file);
}

#[derive(Display)]
pub enum ReadWrite {
    Read,
    Write,
}

pub fn process_cycles(
    mode: &ReadWrite,
    file: &mut File,
    cycles: i32,
    buffer: &mut [u8],
    progress: &ProgressBar,
) -> Result<Vec<Measurement<u64>>> {
    log::info!(
        "read: cycles={} / block_size={}",
        cycles,
        ByteSize(buffer.len() as u64)
    );
    let mut measurements = Vec::new();
    let file_size = file.metadata()?.len();
    progress.inc(0);
    for _ in 0..cycles {
        let (measurement, result) = Measurement::measure(file_size, || {
            file.seek(std::io::SeekFrom::Start(0))?;
            let ops = file_size / buffer.len() as u64;
            for _ in 0..ops {
                match mode {
                    ReadWrite::Read => {
                        let count = file.read(buffer)?;
                        if count == 0 {
                            panic!();
                        }
                    }
                    ReadWrite::Write => {
                        file.write(buffer)?;
                    }
                }
                progress.inc(buffer.len() as u64);
            }
            return Ok(());
        });
        result?;
        measurements.push(measurement);
    }
    return Ok(measurements);
}

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

impl Measurement<u64> {
    pub fn measure<F, R>(value: u64, f: F) -> (Measurement<u64>, R)
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

trait DiskBenchmark {
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
