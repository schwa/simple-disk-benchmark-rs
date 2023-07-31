use anyhow::{Ok, Result};
use bytesize::ByteSize;
use enum_display_derive::Display;
use indicatif::{ProgressBar, ProgressStyle};
use rand::RngCore;
use std::{
    fmt::Display,
    fs::File,
    io::{Read, Seek, Write},
    path::PathBuf,
    vec,
};

mod support;
use support::*;

// MARK: -

#[derive(Display, PartialEq, Debug, Clone)]
pub enum ReadWrite {
    Read,
    Write,
}

#[derive(Debug)]
pub struct SessionOptions {
    pub modes: Vec<ReadWrite>,
    pub path: PathBuf,
    pub file_size: usize,
    pub block_size: usize,
    pub cycles: usize,
    pub no_delete: bool,
    pub dry_run: bool,
}

#[derive(Debug)]
pub struct SessionResult {
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

#[derive(Debug)]
pub struct RunResult {
    pub mode: ReadWrite,
    pub cycle_results: Vec<CycleResult>,
}

#[derive(Debug)]
pub struct Run<'a> {
    pub options: &'a RunOptions<'a>,
}

// MARK: -

#[derive(Debug)]
pub struct CycleOptions<'a> {
    pub run_options: &'a RunOptions<'a>,
    pub progress: &'a ProgressBar,
}

#[derive(Debug)]
pub struct CycleResult {
    pub bytes: usize,
    pub elapsed: f64,
}

pub struct Cycle<'a> {
    pub options: &'a CycleOptions<'a>,
}

// MARL: -

impl Session {
    pub fn main(&self) -> Result<SessionResult> {
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
                let result = run.main().expect("TODO");
                return result;
            })
            .collect();
        let result = SessionResult { runs: runs_results };
        return Ok(result);
    }
}

impl<'a> Run<'a> {
    pub fn main(&self) -> Result<RunResult> {
        let session_options = &self.options.session_options;

        let progress =
            ProgressBar::new((session_options.file_size * session_options.cycles) as u64);
        progress.set_style(
        ProgressStyle::with_template(
            "{prefix:5.green} {spinner} {elapsed_precise} / {eta_precise} {bar:50.green/white} {bytes:9} {msg}",
        )
        .expect("Failed to create progress style.")
        .progress_chars("#-"),
    );
        progress.set_prefix(format!("{}", self.options.mode));

        let mut buffer = vec![0; session_options.block_size];

        if self.options.mode == &ReadWrite::Write {
            let mut rng = rand::thread_rng();
            rng.fill_bytes(&mut buffer);
        }

        let cycle_options = CycleOptions {
            run_options: &self.options,
            progress: &progress,
        };

        let mut file = self.prepare_file(&session_options.path, session_options.file_size)?;

        let results = (0..session_options.cycles).map(|cycle_index| {
            log::info!("Cycle {} of {}", cycle_index + 1, session_options.cycles);
            let cycle = Cycle {
                options: &cycle_options,
            };
            cycle.main(&mut file, &mut buffer)
        });

        if !session_options.no_delete {
            log::info!("Deleting test file {}.", session_options.path.display());
            std::fs::remove_file(&session_options.path)?;
        }

        let result = RunResult {
            mode: self.options.mode.to_owned(),
            cycle_results: results.collect::<Result<Vec<CycleResult>>>()?,
        };
        Ok(result)
    }

    pub fn prepare_file(&self, path: &PathBuf, file_size: usize) -> Result<File> {
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
}

impl<'a> Cycle<'a> {
    fn main(&self, file: &'a mut File, buffer: &'a mut Vec<u8>) -> Result<CycleResult> {
        let run_options = &self.options.run_options;
        let session_options = &run_options.session_options;

        assert!(session_options.file_size > session_options.block_size);

        log::info!(
            "read: cycles={} / block_size={}",
            session_options.cycles,
            ByteSize(session_options.block_size as u64)
        );
        self.options.progress.inc(0);

        file.seek(std::io::SeekFrom::Start(0))?;
        let ops = session_options.file_size / session_options.block_size;

        if session_options.dry_run {
            log::info!("Dry run, skipping read/write.");
            return Ok(CycleResult {
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
                        self.options.progress.inc(session_options.block_size as u64);
                    }
                }
                ReadWrite::Write => {
                    for _ in 0..ops {
                        file.write(buffer)?;
                        self.options.progress.inc(session_options.block_size as u64);
                    }
                }
            }
            Ok(())
        });

        let result = CycleResult {
            bytes: session_options.file_size,
            elapsed,
        };
        return Ok(result);
    }
}
