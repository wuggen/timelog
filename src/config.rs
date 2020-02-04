use crate::timelog::TimeLog;
use crate::commands::Command;

use dirs;
use serde_json;
use structopt::StructOpt;

use std::env;
use std::ffi::OsString;
use std::fs::File;
use std::io;
use std::path::PathBuf;

use std::error::Error;
use std::fmt::{self, Display, Formatter};

use ConfigError::*;

/// Log time.
///
/// The log file to read/write is selected as follows:
/// - The value of the `--file` argument, if given.
/// - If the `--file` argument is absent and the `TIMELOG_LOGFILE` environment variable is set,
///   timelog will use its value.
/// - As a final attempt, timelog will attempt to use `${HOME}/.timelog`.
#[derive(Debug, Clone, StructOpt)]
pub struct Options {
    /// The logfile to read or write.
    #[structopt(long = "file", short = "f")]
    pub logfile: Option<PathBuf>,

    #[structopt(subcommand)]
    pub command: Command,
}

pub fn logfile_path(options: &Options) -> Result<PathBuf, ConfigError> {
    options
        .logfile
        .clone()
        .or_else(|| env::var_os("TIMELOG_LOGFILE").map(<PathBuf as From<OsString>>::from))
        .or_else(|| {
            let home_dir = dirs::home_dir()?;
            Some(home_dir.join(PathBuf::from(".timelog")))
        })
        .ok_or(CannotFindLogFile)
}

pub fn current_timelog(options: &Options) -> Result<TimeLog, ConfigError> {
    let path = logfile_path(options)?;
    match File::open(path) {
        Ok(file) => Ok(serde_json::from_reader(file)?),
        Err(err) => match err.kind() {
            io::ErrorKind::NotFound => Ok(TimeLog::new()),
            _ => Err(err.into()),
        },
    }
}

pub fn write_timelog(options: &Options, timelog: &TimeLog) -> Result<(), ConfigError> {
    let path = logfile_path(options)?;
    let file = File::create(path)?;
    Ok(serde_json::to_writer(file, timelog)?)
}

#[derive(Debug)]
pub enum ConfigError {
    SerdeJson(serde_json::Error),
    CannotFindLogFile,
    CannotOpenLogFile(io::Error),
}

impl Display for ConfigError {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            SerdeJson(err) => write!(f, "error parsing log: {}", err),
            CannotFindLogFile => write!(f, "cannot find log file"),
            CannotOpenLogFile(err) => write!(f, "cannot open log file: {}", err),
        }
    }
}

impl Error for ConfigError {}

impl From<serde_json::Error> for ConfigError {
    fn from(err: serde_json::Error) -> ConfigError {
        SerdeJson(err)
    }
}

impl From<io::Error> for ConfigError {
    fn from(err: io::Error) -> ConfigError {
        CannotOpenLogFile(err)
    }
}