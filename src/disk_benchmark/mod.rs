use anyhow::{Ok, Result};
use enum_display_derive::Display;
use indicatif::{ProgressBar, ProgressStyle};
use rand::RngCore;
use serde::{Deserialize, Serialize};
use std::{
    fmt::Display,
    fs::File,
    io::{Read, Seek, Write},
    path::PathBuf,
    vec,
};

mod support;
use support::*;

use crate::support::*;
use crate::volume::*;

// MARK: -

#[derive(Display, PartialEq, Debug, Clone, Deserialize, Serialize)]
pub enum ReadWrite {
    Read,
    Write,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct SessionOptions {
    pub modes: Vec<ReadWrite>,
    pub path: PathBuf,
    pub file_size: usize,
    pub block_size: usize,
    pub cycles: usize,
    pub no_create: bool,
    pub no_delete: bool,
    pub dry_run: bool,
    pub no_progress: bool,
    pub no_disable_cache: bool,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct SessionResult {
    pub args: String,
    pub created: chrono::DateTime<chrono::Local>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub volume: Option<Volume>,
    pub options: SessionOptions,
    pub runs: Vec<RunResult>,
}

#[derive(Debug)]
pub struct Session {
    pub options: SessionOptions,
}

// MARK: -

#[derive(Debug)]
pub struct RunOptions<'a> {
    pub session_options: &'a SessionOptions,
    pub mode: &'a ReadWrite,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct RunResult {
    pub mode: ReadWrite,
    pub cycle_results: Vec<CycleResult>,
    pub statistics: RunStatistics,
}

#[derive(Debug)]
pub struct Run<'a> {
    pub options: &'a RunOptions<'a>,
}

// MARK: -

#[derive(Debug)]
pub struct CycleOptions<'a> {
    pub cycle: usize,
    pub run_options: &'a RunOptions<'a>,
    pub progress: &'a Option<ProgressBar>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct CycleResult {
    pub cycle: usize,
    pub bytes: usize,
    pub elapsed: f64,
}

#[derive(Debug)]
pub struct Cycle<'a> {
    pub options: &'a CycleOptions<'a>,
}

// MARK: -

impl Session {
    pub fn main(&self) -> Result<SessionResult> {
        let mut file = self.prepare_file(
            &self.options.path,
            self.options.file_size,
            self.options.no_create,
            self.options.no_disable_cache,
        )?;

        let runs_results: Vec<RunResult> = self
            .options
            .modes
            .iter()
            .map(|mode| {
                let run_options = RunOptions {
                    session_options: &self.options,
                    mode: mode,
                };
                let run = Run {
                    options: &run_options,
                };
                let result = run.main(&mut file).expect("TODO");
                return result;
            })
            .collect();
        let result = SessionResult {
            args: std::env::args().collect::<Vec<String>>()[1..].join(" "),
            volume: Volume::volume_for_path(&self.options.path).ok(),
            created: chrono::Local::now(),
            options: self.options.clone(),

            runs: runs_results,
        };

        if !self.options.no_delete {
            if !self.options.no_create {
                log::info!("Deleting test file {}.", self.options.path.display());
                std::fs::remove_file(&self.options.path)?;
            } else {
                log::info!(
                    "Not deleting test file {} due to --no-delete option.",
                    self.options.path.display()
                );
            }
        }

        return Ok(result);
    }

    pub fn prepare_file(
        &self,
        path: &PathBuf,
        file_size: usize,
        no_create: bool,
        no_disable_cache: bool,
    ) -> Result<File> {
        log::info!(
            "Preparing test file {}, size: {}.",
            path.display(),
            file_size
        );

        if path.exists() {
            if no_create {
                log::info!(
                    "File {} already exists, not removing due to --no-create option.",
                    path.display()
                );
            } else {
                log::info!("Deleting existing file {}.", path.display());
                std::fs::remove_file(path)?;
            }
        }

        let mut file = File::open_for_benchmarking(&path, no_create, no_disable_cache)?;
        if !self.options.no_disable_cache {
            file.set_nocache()?;
        }
        log::info!(
            "Writing {} bytes to {}",
            DataSize::from(file_size),
            path.display()
        );

        let (elapsed, result) = measure(|| {
            let mut buffer = vec![0; file_size];
            let mut rng = rand::thread_rng();
            rng.fill_bytes(&mut buffer);
            file.write(&buffer)?;
            file.sync_all()?;
            return Ok(());
        });
        log::info!(
            "Wrote {} in {:.3}s ({}/s)",
            DataSize::from(file_size),
            elapsed,
            DataSize::from(file_size as f64 / elapsed)
        );
        result?;

        return Ok(file);
    }
}

impl<'a> Run<'a> {
    pub fn main(&self, file: &'a mut File) -> Result<RunResult> {
        let session_options = &self.options.session_options;

        let mut progress: Option<ProgressBar> = None;
        if !session_options.no_progress {
            let p = ProgressBar::new((session_options.file_size * session_options.cycles) as u64);
            p.set_style(ProgressStyle::with_template("{prefix:5.green} {spinner} {elapsed_precise} / {eta_precise} {bar:50.green/white} {bytes:9} {msg}")
            .expect("Failed to create progress style.")
            .progress_chars("#-"),
            );
            p.set_prefix(format!("{}", self.options.mode));
            progress = Some(p);
        }

        let mut buffer = vec![0; session_options.block_size];

        if self.options.mode == &ReadWrite::Write {
            let mut rng = rand::thread_rng();
            rng.fill_bytes(&mut buffer);
        }

        let results = (0..session_options.cycles)
            .map(|cycle_index| {
                log::trace!("Cycle {} of {}", cycle_index + 1, session_options.cycles);
                let cycle_options = CycleOptions {
                    cycle: cycle_index,
                    run_options: &self.options,
                    progress: &progress,
                };
                let cycle = Cycle {
                    options: &cycle_options,
                };
                cycle.main(file, &mut buffer)
            })
            .collect::<Result<Vec<CycleResult>>>()?;
        let result = RunResult::new(self.options.mode.to_owned(), results);
        Ok(result)
    }
}

impl RunResult {
    fn new(mode: ReadWrite, cycle_results: Vec<CycleResult>) -> Self {
        let statistics = RunStatistics::new(&cycle_results);
        RunResult {
            mode: mode,
            cycle_results: cycle_results,
            statistics: statistics,
        }
    }
}

impl<'a> Cycle<'a> {
    fn main(&self, file: &'a mut File, buffer: &'a mut Vec<u8>) -> Result<CycleResult> {
        let run_options = &self.options.run_options;
        let session_options = &run_options.session_options;

        assert!(session_options.file_size > session_options.block_size);

        log::trace!(
            "read: cycles={} / block_size={}",
            session_options.cycles,
            DataSize::from(session_options.block_size)
        );
        if let Some(progress) = self.options.progress {
            progress.inc(0);
        }

        file.seek(std::io::SeekFrom::Start(0))?;
        let ops = session_options.file_size / session_options.block_size;

        if session_options.dry_run {
            log::info!("Dry run, skipping read/write.");
            return Ok(CycleResult {
                cycle: self.options.cycle,
                bytes: session_options.file_size,
                elapsed: 1.0,
            });
        }
        let (elapsed, _) = measure(|| -> Result<()> {
            match run_options.mode {
                ReadWrite::Read => {
                    for _ in 0..ops {
                        let count = file.read(buffer)?;
                        if count != buffer.len() {
                            return Err(anyhow::anyhow!(
                                "Read {} bytes, expected {}.",
                                count,
                                buffer.len()
                            ));
                        }
                        if let Some(progress) = self.options.progress {
                            progress.inc(session_options.block_size as u64);
                        }
                    }
                }
                ReadWrite::Write => {
                    for _ in 0..ops {
                        file.write(buffer)?;
                        if let Some(progress) = self.options.progress {
                            progress.inc(session_options.block_size as u64);
                        }
                    }
                }
            }
            Ok(())
        });

        let result = CycleResult {
            cycle: self.options.cycle,
            bytes: session_options.file_size,
            elapsed,
        };
        return Ok(result);
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct RunStatistics {
    pub mean: f64,
    pub median: f64,
    pub standard_deviation: f64,
    pub min: f64,
    pub max: f64,
}

impl RunStatistics {
    fn new(cycle_results: &Vec<CycleResult>) -> Self {
        let timings = cycle_results
            .iter()
            .map(|r| r.bytes as f64 / r.elapsed)
            .collect::<Vec<f64>>();
        let mean = statistical::mean(&timings);
        let median = statistical::median(&timings);
        let standard_deviation = statistical::standard_deviation(&timings, Some(mean));
        let min = min(&timings);
        let max = max(&timings);

        RunStatistics {
            mean,
            median,
            standard_deviation,
            min,
            max,
        }
    }
}
