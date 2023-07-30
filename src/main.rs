use bytesize::ByteSize;
use clap::Parser;
use indicatif::{ProgressBar, ProgressStyle};
use simple_disk_benchmark::*;
use std::path::PathBuf;
use std::vec;

mod colored_markup;
use colored_markup::*;

mod disk_benchmark;
use disk_benchmark::*;

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
