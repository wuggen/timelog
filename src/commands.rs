use crate::interval;
use crate::timelog::{TimeLog, TimeLogError};

use chrono::{Local, TimeZone};
use structopt::StructOpt;

use std::error::Error;
use std::fmt::{self, Display, Formatter};

#[derive(Debug, Clone, StructOpt)]
pub enum Command {
    /// Open a new interval for the given tag, or the tag 'default'.
    Open { tag: Option<String> },

    /// Close the currently open interval for the given tag, or the tag 'default'.
    Close { tag: Option<String> },

    /// List all logged intervals for all tags.
    List,
}

impl Command {
    pub fn execute(&self, timelog: &mut TimeLog) -> Result<(), CommandError> {
        match self {
            Command::Open { tag } => open(
                &tag.as_ref().cloned().unwrap_or_else(|| "default".into()),
                timelog,
            ),
            Command::Close { tag } => close(
                &tag.as_ref().cloned().unwrap_or_else(|| "default".into()),
                timelog,
            ),
            Command::List => {
                list(timelog);
                Ok(())
            }
        }
    }
}

fn open(tag: &str, timelog: &mut TimeLog) -> Result<(), CommandError> {
    match timelog.open(tag) {
        Ok(int) => {
            let start = Local.from_utc_datetime(&int.start().naive_utc());
            println!(
                "Opened new interval for tag '{}' at {}",
                tag,
                start.format(interval::FMT_STR)
            );
            Ok(())
        }
        Err(err) => {
            println!("Error opening new interval for tag '{}': {}", tag, err);
            Err(err.into())
        }
    }
}

fn close(tag: &str, timelog: &mut TimeLog) -> Result<(), CommandError> {
    match timelog.close(tag) {
        Ok(int) => {
            println!("Closed interval for tag '{}': {}", tag, int.interval());
            Ok(())
        }
        Err(err) => {
            println!("Error closing interval for tag '{}': {}", tag, err);
            Err(err.into())
        }
    }
}

fn list(timelog: &TimeLog) {
    for int in timelog.iter() {
        let tag = timelog.tag_name(int.tag()).unwrap();
        println!("{}: {}", tag, int.interval());
    }
}

#[derive(Debug, Clone)]
pub enum CommandError {
    TimeLogError(TimeLogError),
}

impl Display for CommandError {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            CommandError::TimeLogError(err) => Display::fmt(err, f),
        }
    }
}

impl Error for CommandError {}

impl From<TimeLogError> for CommandError {
    fn from(err: TimeLogError) -> CommandError {
        CommandError::TimeLogError(err)
    }
}
