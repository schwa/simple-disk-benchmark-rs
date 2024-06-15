use anyhow::{Ok, Result};
use enum_display_derive::Display;
use indicatif::{ProgressBar, ProgressStyle};
use rand::{Rng, RngCore};
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
    pub modes: Vec<ReadWrite>, // TODO: Make ref?
    pub path: PathBuf,         // TODO: Make ref?
    pub file_size: usize,
    pub block_size: usize,
    pub cycles: usize,
    pub no_create: bool,
    pub no_delete: bool,
    pub dry_run: bool,
    pub no_progress: bool,
    pub no_disable_cache: bool,
    pub random_seek: bool,
    pub no_close_file: bool,
    pub no_random_buffer: bool,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct SessionResult {
    pub args: String,
    #[serde(with = "time::serde::iso8601")]
    pub created: time::OffsetDateTime,
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
        let file = self.prepare_file(
            &self.options.path,
            self.options.file_size,
            self.options.no_create,
            self.options.no_random_buffer,
        )?;
        drop(file);

        let runs_results: Vec<RunResult> = self
            .options
            .modes
            .iter()
            .map(|mode| {
                let run_options = RunOptions {
                    session_options: &self.options,
                    mode,
                };
                let run = Run {
                    options: &run_options,
                };

                run.main().expect("TODO")
            })
            .collect();
        let result = SessionResult {
            args: std::env::args().collect::<Vec<String>>()[1..].join(" "),
            volume: Volume::volume_for_path(&self.options.path).ok(),
            created: time::OffsetDateTime::now_local()?,
            options: self.options.clone(),

            runs: runs_results,
        };

        if !self.options.no_delete {
            if !self.options.no_create {
                log::debug!(
                    target: "Session",
                    "Deleting test file {}.",
                    self.options.path.display()
                );
                std::fs::remove_file(&self.options.path)?;
            } else {
                log::debug!(
                    target: "Session",
                    "Not deleting test file {} due to --no-delete option.",
                    self.options.path.display()
                );
            }
        }

        Ok(result)
    }

    pub fn prepare_file(
        &self,
        path: &PathBuf,
        file_size: usize,
        no_create: bool,
        no_random_buffer: bool,
    ) -> Result<File> {
        log::debug!(
            target: "Session",
            "Preparing test file {}, size: {}.",
            path.display(),
            file_size
        );

        if path.exists() {
            if no_create {
                log::debug!(
                    target: "Session",
                    "File {} already exists, not removing due to --no-create option.",
                    path.display()
                );
            } else {
                log::debug!(
                    target: "Session",
                    "Deleting existing file {}.",
                    path.display()
                );
                std::fs::remove_file(path)?;
            }
        }
        log::trace!(
            target: "Session",
            "Creating file {}.",
            path.display()
        );
        let mut file = File::create_for_benchmarking(path, self.options.no_disable_cache)?;
        log::debug!(
            target: "Session",
            "Writing {} bytes to {}",
            DataSize::from(file_size),
            path.display()
        );

        let (elapsed, result) = measure(|| {
            log::trace!(
                target: "Session",
                "Creating buffer.",
            );
            let mut buffer = vec![0; file_size];

            if !no_random_buffer {
                log::trace!(
                    target: "Session",
                    "Filing random buffer.",
                );
                if cfg!(debug_assertions) {
                    log::warn!("This can take a long time in debug builds. Make sure you're running in a release build.");
                }
                let mut rng = rand::thread_rng();
                rng.fill_bytes(&mut buffer);
            } else {
                log::trace!(
                    target: "Session",
                    "Filing buffer with pattern.",
                );
                // fill buffer with 0xDEADBEEF pattern
                let mut i = 0;
                while i < buffer.len() {
                    let bytes = [0xDE, 0xAD, 0xBE, 0xEF];
                    let bytes_to_copy = std::cmp::min(bytes.len(), buffer.len() - i);
                    buffer[i..i + bytes_to_copy].copy_from_slice(&bytes[..bytes_to_copy]);
                    i += bytes_to_copy;
                }
            }

            log::trace!(
                target: "Session",
                "Writing buffer.",
            );
            let bytes_written = file.write(&buffer)?;
            anyhow::ensure!(
                bytes_written == file_size,
                "Failed to write all bytes to file.",
            );
            file.sync_all()?;
            Ok(())
        });

        log::debug!(
            target: "Session",
            "Wrote {} in {:.3}s ({}/s)",
            DataSize::from(file_size),
            elapsed,
            DataSize::from(file_size as f64 / elapsed)
        );
        result?;

        Ok(file)
    }
}

impl<'a> Run<'a> {
    pub fn main(&self) -> Result<RunResult> {
        log::debug!(target: "Session::Run", "Starting run.");
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

        let mut file = None;
        if session_options.no_close_file {
            log::debug!(target: "Session::Run","Opening file _once_ for this run due to --no-close-file option.");
            file = Some(File::open_for_benchmarking(
                &session_options.path,
                session_options.no_disable_cache,
            )?)
        }

        let mut results = Vec::with_capacity(session_options.cycles);

        for cycle_index in 0..session_options.cycles {
            let cycle_options = CycleOptions {
                cycle: cycle_index,
                run_options: self.options,
                progress: &progress,
            };
            let cycle = Cycle {
                options: &cycle_options,
            };

            let cycle_result = cycle.main(&file, &mut buffer);
            results.push(cycle_result?);
        }

        let result = RunResult::new(self.options.mode.to_owned(), results);
        log::debug!(target: "Session::Run","Ending run.");
        Ok(result)
    }
}

impl RunResult {
    fn new(mode: ReadWrite, cycle_results: Vec<CycleResult>) -> Self {
        let statistics = RunStatistics::new(&cycle_results);
        RunResult {
            mode,
            cycle_results,
            statistics,
        }
    }
}

impl<'a> Cycle<'a> {
    fn main(&self, file: &'a Option<File>, buffer: &'a mut [u8]) -> Result<CycleResult> {
        let run_options = &self.options.run_options;
        let session_options = &run_options.session_options;
        log::debug!(target: "Session::Run::Cycle", "Starting cycle {}/{}.", self.options.cycle + 1, session_options.cycles);

        assert!(session_options.file_size > session_options.block_size);

        let my_file: Option<File> = match file {
            Some(_) => None,
            None => Some(File::open_for_benchmarking(
                &session_options.path,
                session_options.no_disable_cache,
            )?),
        };

        let mut file: &File = match file {
            Some(file) => file,
            None => my_file.as_ref().unwrap(),
        };

        if let Some(progress) = self.options.progress {
            progress.inc(0);
        }

        let ops = session_options.file_size / session_options.block_size;
        log::debug!(target: "Session::Run::Cycle", "Performing {} {} operations of {} bytes each.", ops, run_options.mode, DataSize::new(session_options.block_size, Unit::B).to_human_string());

        if session_options.dry_run {
            log::debug!(target: "Session::Run::Cycle", "Dry run, skipping read/write.");
            return Ok(CycleResult {
                cycle: self.options.cycle,
                bytes: session_options.file_size,
                elapsed: 1.0,
            });
        }
        let (elapsed, _) = measure(|| -> Result<()> {
            if session_options.random_seek {
                let random_seek_location = rand::thread_rng()
                    .gen_range(0..session_options.file_size - session_options.block_size);
                file.seek(std::io::SeekFrom::Start(random_seek_location as u64))?;
            }

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
                        let bytes_written = file.write(buffer)?;
                        anyhow::ensure!(
                            bytes_written == buffer.len(),
                            "Failed to write all bytes to file.",
                        );

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
        log::debug!(target: "Session::Run::Cycle", "Ending cycle.");
        Ok(result)
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
    fn new(cycle_results: &[CycleResult]) -> Self {
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
