#![allow(unused_imports)]
#![allow(dead_code)]

use anyhow::{anyhow, Ok, Result};
use bytesize::ByteSize;
use clap::Parser;
use indicatif::{ProgressBar, ProgressStyle};
use rand::RngCore;
use simple_disk_benchmark::*;
use std::collections::HashMap;
use std::collections::HashSet;
use std::default;
use std::fs::File;
use std::io::{Read, Seek, Write};
use std::os::fd::AsRawFd;
use std::path::PathBuf;
use std::vec;

mod colored_markup;
use colored_markup::*;

/// From <https://www.geschke-online.de/sdb/sdb.1.html>
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(
        short = 'f',
        long = "file",
        value_name = "FILE",
        default_value = "testfile.dat"
    )]
    path: PathBuf,

    #[arg(short, long = "blocksize", value_parser = parse_data_size, default_value = "128MB")]
    block_size: DataSize,

    #[arg(short = 'D', long, default_value_t = false)]
    direct: bool,

    #[arg(short = 'F', long, default_value_t = false)]
    use_fsync: bool,

    #[arg(short = 's', long = "size", value_name = "FILESIZE", value_parser = parse_data_size, default_value = "1GB")]
    file_size: DataSize,

    #[arg(short, long, default_value_t = 10)]
    cycles: i32,
}

fn main() {
    let args = Args::parse();

    simple_logger::SimpleLogger::new().env().init().unwrap();
    log::debug!("{:?}", args);

    let file_size = args.file_size.to_bytes();
    let block_size = args.block_size.to_bytes();
    let progress = ProgressBar::new(file_size as u64 * args.cycles as u64);
    progress.set_style(
        ProgressStyle::with_template(
            "{prefix:.green} {spinner} {elapsed_precise} {eta_precise} {bar:40.green/white} {bytes}/{total_bytes} {bytes_per_sec} {msg}",
        )
        .unwrap()
        .progress_chars("#-."),
    );
    progress.set_prefix("Reading");

    let mut file = prepare_file(&args.path, file_size).unwrap();
    let mut buffer: Vec<u8> = vec![0; block_size];
    let measurements = process_cycles(
        ReadWrite::Write,
        &mut file,
        args.cycles,
        &mut buffer,
        &progress,
    )
    .unwrap();
    drop(file);
    std::fs::remove_file(&args.path).unwrap();

    let timings = measurements
        .iter()
        .map(|m| m.per_sec())
        .collect::<Vec<f64>>();
    let mean = statistical::mean(&timings);
    let median = statistical::median(&timings);
    let standard_deviation = statistical::standard_deviation(&timings, Some(mean));
    let min = min(&timings);
    let max = max(&timings);

    let template = Template::stylesheet(
        "g { foreground: green }
        r { foreground: magenta }",
    )
    .unwrap();

    //log::debug!("{:?}", template);

    println!(
        "{}",
        cmarkup!(
            template,
            "Mean: <g>{}/s</g>, Medium: <g>{}/s</g>, Standard Deviation: <r>{}/s</r>",
            ByteSize(mean as u64),
            ByteSize(median as u64),
            ByteSize(standard_deviation as u64)
        )
    );
    println!(
        "{}",
        cmarkup!(
            template,
            "Min: <g>{}/s</g>, Max: <r>{}/s</r>",
            ByteSize(min as u64),
            ByteSize(max as u64)
        )
    );
}

fn prepare_file(path: &PathBuf, file_size: usize) -> Result<File> {
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
    //    let mut file = File::new(path);?.read(true)?.write(true)?.create(true)?;
    //create a file for read and write and create
    let mut file = File::options()
        .create(true)
        .read(true)
        .write(true)
        .open(&path)?;

    let fd = file.as_raw_fd();
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

enum ReadWrite {
    Read,
    Write,
}

fn process_cycles(
    mode: ReadWrite,
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
    let file_size = file.metadata().unwrap().len();
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
