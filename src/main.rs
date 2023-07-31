use anyhow::ensure;
use bytesize::ByteSize;
use clap::Parser;
use clap_verbosity_flag::*;
use enum_display_derive::Display;
use std::collections::HashSet;
use std::fmt::Display;
use std::path::PathBuf;
use std::vec;
//
mod support;
use support::*;

mod colored_markup;
use colored_markup::*;

mod disk_benchmark;
use disk_benchmark::*;

// Based partly on: From <https://www.geschke-online.de/sdb/sdb.1.html>

/// A simple tool for benchmarking disk performance.
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// File to use for benchmarking. If this file exists it will be deleted.
    #[arg(value_name = "FILE", default_value = "testfile.dat")]
    path: PathBuf,

    /// Size of the file to use for benchmarking.
    #[arg(short = 's', long = "size", value_name = "FILESIZE", value_parser = parse_data_size, default_value = "1GB")]
    file_size: DataSize,

    /// Size of the blocks to read/write.
    #[arg(short, long = "blocksize", value_parser = parse_data_size, default_value = "128MB")]
    block_size: DataSize,

    /// Number of test cycles to run.
    #[arg(short, long, default_value_t = 10)]
    cycles: i32,

    /// Types of test to run: read, write or all.
    #[arg(short, long, default_value = "all")]
    mode: Vec<Mode>,

    /// Do not delete the test file after the test.
    #[arg(short, long, default_value_t = false)]
    no_delete: bool,

    /// Set the log level.
    #[clap(flatten)]
    verbose: Verbosity<WarnLevel>,

    /// Do not actually perform benchmarks to the disk (file is still created and/or deleted)
    #[arg(short, long, default_value_t = false)]
    dry_run: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Display, clap::ValueEnum)]
enum Mode {
    All,
    Read,
    Write,
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    simple_logger::SimpleLogger::new()
        // .with_module_level("ignore::walk", LevelFilter::Warn)
        .with_level(args.verbose.log_level_filter())
        .env()
        .init()
        .expect("Failed to initialize logger.");
    log::debug!("{:?}", args);

    let file_size = args.file_size.to_bytes() as usize;
    let block_size = args.block_size.to_bytes() as usize;
    ensure!(
        file_size > block_size,
        "File size ({}) is smaller than block size ({}).",
        ByteSize(file_size as u64),
        ByteSize(block_size as u64)
    );
    ensure!(file_size > 0, "File size must be greater than zero.");
    ensure!(block_size > 0, "Block size must be greater than zero.");

    // if file size is not divisible by block size, reduce file size and log a warning
    if file_size % block_size != 0 {
        let new_file_size = file_size - (file_size % block_size);
        log::warn!(
            "File size ({}) is not divisible by block size ({}). Reducing file size to {}.",
            ByteSize(file_size as u64),
            ByteSize(block_size as u64),
            ByteSize(new_file_size as u64)
        );
    }

    let modes: HashSet<&Mode> = HashSet::from_iter(args.mode.iter());
    let modes = if modes.contains(&Mode::All) {
        vec![Mode::Read, Mode::Write]
    } else {
        args.mode.clone()
    };
    let modes = modes
        .iter()
        .map(|m| match m {
            Mode::Read => ReadWrite::Read,
            Mode::Write => ReadWrite::Write,
            Mode::All => unreachable!(),
        })
        .collect::<Vec<ReadWrite>>();

    let style_sheet = StyleSheet::parse(
        "
        x { foreground: red; styles: bold }
        g { foreground: green }
        r { foreground: cyan }
        ",
    )
    .expect("Failed to parse stylesheet.");

    println!(
        "{}",
        cmarkup!(style_sheet, "Cycles <r>{}</r>, ", args.cycles)
    );
    println!(
        "{}",
        cmarkup!(style_sheet, "Block Size <r>{}</r>, ", args.block_size)
    );
    println!(
        "{}",
        cmarkup!(style_sheet, "File Size <r>{}</r>, ", args.file_size)
    );
    println!();

    let options = SessionOptions {
        modes: modes,
        path: args.path,
        file_size: args.file_size.to_bytes() as usize,
        block_size: args.block_size.to_bytes() as usize,
        cycles: args.cycles as usize,
        no_delete: args.no_delete,
        dry_run: args.dry_run,
    };
    let session = Session { options };
    let result = session.main().expect("Session failed.");

    for run in result.runs.iter() {
        run.display_result(&style_sheet);
    }

    Ok(())
}

trait RunDisplay {
    fn display_result(&self, style_sheet: &StyleSheet);
}

impl RunDisplay for RunResult {
    fn display_result(&self, style_sheet: &StyleSheet) {
        let timings = self
            .cycle_results
            .iter()
            .map(|r| r.bytes as f64 / r.elapsed)
            .collect::<Vec<f64>>();
        log::info!("Timings: {:?}", timings);
        let mean = statistical::mean(&timings);
        let median = statistical::median(&timings);
        let standard_deviation = statistical::standard_deviation(&timings, Some(mean));
        let min = min(&timings);
        let max = max(&timings);

        println!();
        println!("{}", cmarkup!(style_sheet, "Mode: <x>{}</x>", self.mode));
        println!(
            "{}",
            cmarkup!(
                style_sheet,
                "Mean: <g>{}/s</g>, Medium: <g>{}/s</g>, Standard Deviation Ø: <r>{}/s</r>",
                ByteSize(mean as u64),
                ByteSize(median as u64),
                ByteSize(standard_deviation as u64)
            )
        );
        println!(
            "{}",
            cmarkup!(
                style_sheet,
                "Min: <g>{}/s</g>, Max: <r>{}/s</r>",
                ByteSize(min as u64),
                ByteSize(max as u64)
            )
        );
    }
}
