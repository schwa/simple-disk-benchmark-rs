use anyhow::Result;
use bytesize::ByteSize;
use clap::Parser;
use enum_display_derive::Display;
use indicatif::{ProgressBar, ProgressStyle};
use simple_disk_benchmark::*;
use std::collections::HashSet;
use std::fmt::Display;
use std::path::PathBuf;
use std::vec;

//

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

    /// TODO: Not implemented yet.
    #[arg(short = 'F', long, default_value_t = false)]
    use_fsync: bool,

    /// Types of test to run: read, write or all.
    #[arg(short, long, default_value = "all")]
    mode: Vec<Mode>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Display)]
enum Mode {
    All,
    Read,
    Write,
}

impl From<&str> for Mode {
    fn from(s: &str) -> Self {
        match s {
            "all" => Mode::All,
            "read" => Mode::Read,
            "write" => Mode::Write,
            _ => panic!(),
        }
    }
}

fn main() {
    let args = Args::parse();

    simple_logger::SimpleLogger::new()
        // .with_module_level("ignore::walk", LevelFilter::Warn)
        .with_level(log::LevelFilter::Warn)
        .env()
        .init()
        .expect("Failed to initialize logger.");
    log::debug!("{:?}", args);

    let modes: HashSet<&Mode> = HashSet::from_iter(args.mode.iter());
    let modes = if modes.contains(&Mode::All) {
        vec![Mode::Read, Mode::Write]
    } else {
        args.mode.clone()
    };

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

    let runs: Vec<Run> = modes
        .iter()
        .map(|mode| Run::run(mode, &args).expect("Run failed."))
        .collect();

    for run in runs.iter() {
        run.display_result(&style_sheet);
    }
}

struct Run {
    mode: ReadWrite,
    measurements: Vec<Measurement<u64>>,
}

impl Run {
    fn run(mode: &Mode, args: &Args) -> Result<Self> {
        let mode = match mode {
            Mode::Read => ReadWrite::Read,
            Mode::Write => ReadWrite::Write,
            _ => panic!(),
        };

        let file_size = args.file_size.to_bytes();
        let progress = ProgressBar::new(file_size as u64 * args.cycles as u64);
        progress.set_style(
            ProgressStyle::with_template(
                "{prefix:5.green} {spinner} {elapsed_precise} / {eta_precise} {bar:50.green/white} {bytes:9} {msg}",
            )
            .expect("Failed to create progress style.")
            .progress_chars("#-"),
        );
        progress.set_prefix(format!("{}", mode));

        let file_size = args.file_size.to_bytes();
        let block_size = args.block_size.to_bytes();

        let mut file = prepare_file(&args.path, file_size)?;
        let mut buffer: Vec<u8> = vec![0; block_size];
        let measurements = process_cycles(&mode, &mut file, args.cycles, &mut buffer, &progress)?;
        drop(file);
        std::fs::remove_file(&args.path)?;

        println!();

        Ok(Self { mode, measurements })
    }

    fn display_result(&self, style_sheet: &StyleSheet) {
        let timings = self
            .measurements
            .iter()
            .map(|m| m.per_sec())
            .collect::<Vec<f64>>();
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
                "Mean: <g>{}/s</g>, Medium: <g>{}/s</g>, Standard Deviation: <r>{}/s</r>",
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
