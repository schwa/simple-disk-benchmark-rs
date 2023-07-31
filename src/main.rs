use anyhow::{ensure, Ok};
use clap::Parser;
use clap_verbosity_flag::*;
use enum_display_derive::Display;
use minijinja::{context, Environment};
use serde::Serialize;
use serde_json;
use std::collections::HashSet;
use std::fmt::Display;
use std::fs::File;
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
    file_size: DataSize<usize>,

    /// Size of the blocks to read/write.
    #[arg(short, long = "blocksize", value_parser = parse_data_size, default_value = "128MB")]
    block_size: DataSize<usize>,

    /// Number of test cycles to run.
    #[arg(short, long, default_value_t = 10)]
    cycles: i32,

    /// Types of test to run: read, write or all.
    #[arg(short, long, default_value = "all")]
    mode: Vec<Mode>,

    /// Do not create the test file, the file must already exist.
    #[arg(long, default_value_t = false)]
    no_create: bool,

    /// Do not delete the test file after the test.
    #[arg(long, default_value_t = false)]
    no_delete: bool,

    /// Do not display progress bar.
    #[arg(long, default_value_t = false)]
    no_progress: bool,

    /// Do not disable the file system cache.
    #[arg(long, default_value_t = false)]
    no_disable_cache: bool,

    /// Set the log level.
    #[clap(flatten)]
    verbose: Verbosity<WarnLevel>,

    /// Do not actually perform benchmarks to the disk (file is still created and/or deleted).
    #[arg(short, long, default_value_t = false)]
    dry_run: bool,

    /// Export the timing summary statistics and timings of individual runs as JSON to the given FILE. The output time unit is always seconds.
    #[arg(long, value_name = "FILE")]
    export_json: Option<PathBuf>,
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

    let file_size: usize = args.file_size.into();
    let block_size: usize = args.block_size.into();
    ensure!(
        file_size > block_size,
        "File size ({}) is smaller than block size ({}).",
        args.file_size,
        args.block_size
    );
    ensure!(file_size > 0, "File size must be greater than zero.");
    ensure!(block_size > 0, "Block size must be greater than zero.");

    // if file size is not divisible by block size, reduce file size and log a warning
    if file_size % block_size != 0 {
        let new_file_size = file_size - (file_size % block_size);
        log::warn!(
            "File size ({}) is not divisible by block size ({}). Reducing file size to {}.",
            args.file_size,
            args.block_size,
            DataSize::from(new_file_size),
        );
    }

    let modes: HashSet<&Mode> = HashSet::from_iter(args.mode.iter());
    let modes = if modes.contains(&Mode::All) {
        vec![Mode::Read, Mode::Write]
    } else {
        args.mode.clone()
    };
    log::debug!("Modes: {:?}", modes);
    let modes = modes
        .iter()
        .map(|m| match m {
            Mode::Read => ReadWrite::Read,
            Mode::Write => ReadWrite::Write,
            Mode::All => unreachable!(),
        })
        .collect::<Vec<ReadWrite>>();

    let info = os_info::get();
    info.version();

    let template = "File: <info>{{file}}</info>
OS: <info>{{os.os_type}} {{os_version}} ({{os.architecture}})</info>
Cycles: <num>{{ cycles }}</num>
Block Size: <size>{{ block_size }}</size>
File Size: <size>{{ file_size }}</size>";
    let context = context! {
        file => args.path.to_string_lossy(),
        os => info,
        os_version => info.version().to_string(),
        cycles => args.cycles,
        block_size => args.block_size.to_human_string(),
        file_size => args.file_size.to_human_string(),
    };
    render(&template, &context)?;

    // TODO: It's rather silly copying all this from Args.
    let options = SessionOptions {
        modes: modes,
        path: args.path,
        file_size: args.file_size.into(),
        block_size: args.block_size.into(),
        cycles: args.cycles as usize,
        no_create: args.no_create,
        no_delete: args.no_delete,
        dry_run: args.dry_run,
        no_progress: args.no_progress,
        no_disable_cache: args.no_disable_cache,
    };
    let session = Session { options };
    let session_result = session.main().expect("Session failed.");

    for run_result in session_result.runs.iter() {
        run_result.display_result();
    }

    if let Some(path) = args.export_json {
        let report = Report::new(&session.options, &session_result);
        let file = File::create(path)?;
        serde_json::to_writer_pretty(file, &report)?;
    }

    Ok(())
}

trait RunDisplay {
    fn display_result(&self);
}

impl RunDisplay for RunResult {
    fn display_result(&self) {
        let template = "Mode: <mode>{{mode}}</mode>
Mean: <speed>{{mean}}</speed>/sec, Median: <speed>{{median}}</speed>/sec, Standard Deviation Ø: <speed>{{standard_deviation}}</speed>/sec
Min: <speed>{{min}}</speed>/sec, Max: <speed>{{max}}</speed>/sec";
        let context = context! {
            mode => self.mode.to_string(),
            mean => DataSize::from(self.statistics.mean).to_human_string(),
            median => DataSize::from(self.statistics.median).to_human_string(),
            standard_deviation => DataSize::from(self.statistics.standard_deviation).to_human_string(),
            min => DataSize::from(self.statistics.min).to_human_string(),
            max => DataSize::from(self.statistics.max).to_human_string(),
        };
        render(&template, &context).unwrap();
    }
}

fn render(template: &str, context: &minijinja::value::Value) -> anyhow::Result<()> {
    let style_sheet = StyleSheet::parse(
        "
        info { foreground: yellow }
        mode { foreground: red }
        speed { foreground: cyan }
        size { foreground: green }
        num { foreground: yellow }
        ",
    )
    .expect("Failed to parse stylesheet.");

    let mut env = Environment::new();
    env.add_template("template", template).unwrap();
    let tmpl = env.get_template("template").unwrap();
    let render = tmpl.render(context).unwrap();
    println!("{}", style_sheet.render(&render)?);

    Ok(())
}

#[derive(Serialize)]
struct Report<'a> {
    args: String,
    created: chrono::DateTime<chrono::Local>,
    options: &'a SessionOptions,
    runs: &'a Vec<RunResult>,
}

impl<'a> Report<'a> {
    fn new(session: &'a SessionOptions, result: &'a SessionResult) -> Self {
        Self {
            args: std::env::args().collect::<Vec<String>>()[1..].join(" "),
            created: chrono::Local::now(),
            options: session,
            runs: &result.runs,
        }
    }
}
