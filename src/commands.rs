use crate::timelog::{TimeLog, TimeLogError};

use chrono::{Local, TimeZone};
use structopt::StructOpt;

use std::error::Error;
use std::fmt::{self, Display, Formatter};

#[derive(Debug, Clone, StructOpt)]
pub enum Command {
    Open { tag: Option<String> },

    Close { tag: Option<String> },

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
            println!("Opened new interval for tag '{}' at {}", tag, start);
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
            let start = Local.from_utc_datetime(&int.start().naive_utc());
            let end = Local.from_utc_datetime(&int.end().unwrap().naive_utc());
            println!("Closed interval for tag '{}': {} -- {}", tag, start, end);
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
        let start = Local
            .from_utc_datetime(&int.start().naive_utc())
            .to_string();
        let end = int
            .end()
            .map(|d| Local.from_utc_datetime(&d.naive_utc()).to_string())
            .unwrap_or_else(|| "OPEN".into());

        println!("{}: {} -- {}", tag, start, end);
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
