use anyhow::{ensure, Ok, Result};
use clap::Parser;
use clap_verbosity_flag::{Verbosity, WarnLevel};
use enum_display_derive::Display;
use fern::colors::{Color, ColoredLevelConfig};
use minijinja::{context, Environment};
use std::time::SystemTime;
use std::{collections::HashSet, fmt::Display, fs::File, path::PathBuf, vec};

mod colored_markup;
mod disk_benchmark;
mod support;
mod volume;

use colored_markup::*;
use disk_benchmark::*;
use support::*;

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

    /// Seek to a random position in the file before each read/write.
    #[arg(short, long)]
    random_seek: bool,

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

    /// Do not close the file after each cycle.
    #[arg(long, default_value_t = false)]
    no_close_file: bool,

    /// Fill the buffer with fixed byte pattern on creation instead of random.
    #[arg(long, default_value_t = true)]
    no_random_buffer: bool,

    /// Do not display a bar chart of the run timings.
    #[arg(short = 'X', long)]
    no_chart: bool,

    /// Export the timing summary statistics and timings of individual runs as JSON to the given FILE. The output time unit is always seconds.
    #[arg(short('j'), long, value_name = "FILE")]
    export_json: Option<PathBuf>,

    /// Export the log to the given FILE.
    #[arg(long, value_name = "FILE")]
    export_log: Option<PathBuf>,

    /// Do not actually perform benchmarks to the disk (file is still created and/or deleted).
    #[arg(short, long, default_value_t = false)]
    dry_run: bool,

    /// Set the log level.
    #[clap(flatten)]
    verbose: Verbosity<WarnLevel>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Display, clap::ValueEnum)]
enum Mode {
    All,
    Read,
    Write,
}

fn main() -> Result<()> {
    let args = Args::parse();

    let start_time = SystemTime::now();

    setup_logger(
        args.verbose.log_level_filter(),
        &args.export_log,
        start_time,
    )?;

    log::debug!("{:?}", args);

    let file_size: usize = args.file_size.into();
    let block_size: usize = args.block_size.into();
    ensure!(
        file_size > block_size,
        "File size ({}) is smaller than block size ({}).",
        args.file_size,
        args.block_size
    );
    ensure!(
        args.cycles >= 2,
        "Number of cycles must be at least two. (`--cycles 2`)"
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
    render(template, &context)?;

    // TODO: It's rather silly copying all this from Args.
    let options = SessionOptions {
        modes,
        path: args.path,
        file_size: args.file_size.into(),
        block_size: args.block_size.into(),
        cycles: args.cycles as usize,
        no_create: args.no_create,
        no_delete: args.no_delete,
        dry_run: args.dry_run,
        no_progress: args.no_progress,
        no_disable_cache: args.no_disable_cache,
        random_seek: args.random_seek,
        no_close_file: args.no_close_file,
        no_random_buffer: args.no_random_buffer,
    };
    let session = Session { options };
    let session_result = session.main().expect("Session failed.");

    for run_result in session_result.runs.iter() {
        run_result.display_result();
    }

    if !args.no_chart {
        let data: Vec<Vec<f64>> = session_result
            .runs
            .iter()
            .map(|r| r.cycle_results.iter().map(|c| c.elapsed).collect())
            .collect();
        let res = rasciigraph::plot_many(
            data,
            rasciigraph::Config::default()
                .with_height(10)
                .with_width(80),
        );
        print!("Timing:\n{}", res);
    }

    if let Some(path) = args.export_json {
        if path.exists() {
            log::warn!("File {} already exists, appending.", path.display());
            let mut file = File::open(&path)?;
            let mut reports: Vec<SessionResult> = serde_json::from_reader(&mut file)?;
            reports.push(session_result);
            let file = File::create(path)?;
            serde_json::to_writer_pretty(file, &reports)?;
        } else {
            let reports = vec![session_result];
            let file = File::create(path)?;
            serde_json::to_writer_pretty(file, &reports)?;
        }
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
        render(template, &context).unwrap();
    }
}

fn render(template: &str, context: &minijinja::value::Value) -> Result<()> {
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

fn setup_logger(
    level_filter: log::LevelFilter,
    log_path: &Option<PathBuf>,
    start_time: SystemTime,
) -> Result<()> {
    let colors = ColoredLevelConfig::new()
        .info(Color::Green)
        .debug(Color::Magenta);
    let mut base_logger = fern::Dispatch::new();
    let console_logger = fern::Dispatch::new()
        .level(level_filter)
        .format(move |out, message, record| {
            let duration = SystemTime::now().duration_since(start_time).unwrap();
            let duration_string = format!("{:10.3}", duration.as_secs_f64());
            out.finish(format_args!(
                "{} {:8.8} {:24.24} | {}",
                duration_string,
                colors.color(record.level()),
                record.target(),
                message
            ))
        })
        .chain(std::io::stdout());
    base_logger = base_logger.chain(console_logger);

    if let Some(log_path) = log_path {
        let file_logger = fern::Dispatch::new()
            .format(move |out, message, record| {
                out.finish(format_args!(
                    "[{} {} {}] {}",
                    humantime::format_rfc3339_seconds(SystemTime::now()),
                    record.level(),
                    record.target(),
                    message
                ))
            })
            .chain(fern::log_file(log_path)?);
        base_logger = base_logger.chain(file_logger);
    }

    base_logger.apply()?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use assert_cmd::Command;

    #[test]
    fn test_cli() {
        let mut cmd = Command::cargo_bin(env!("CARGO_PKG_NAME")).unwrap();
        let output = cmd
            .args([
                "--size",
                "1MB",
                "--blocksize",
                "64KB",
                "--no-chart",
                "--no-progress",
            ])
            .unwrap();
        println!("{:?}", output);
    }
}
