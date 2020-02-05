use crate::filter::{self, Filter};
use crate::interval;
use crate::timelog::{TimeLog, TimeLogError};

use chrono::offset::Offset;
use chrono::{DateTime, Duration, Local, NaiveDate, NaiveDateTime, NaiveTime, TimeZone, Utc};
use structopt::StructOpt;

use std::io::{self, Write};

use std::error::Error;
use std::fmt::{self, Display, Formatter};

#[derive(Debug, Clone, Copy, PartialOrd, Ord, PartialEq, Eq, Hash)]
pub enum ChangeStatus {
    Changed,
    Unchanged,
}

impl ChangeStatus {
    pub fn is_changed(self) -> bool {
        self == ChangeStatus::Changed
    }
}

#[derive(Debug, Clone, StructOpt)]
pub enum Command {
    /// Open a new interval for the given tag, or the tag 'default'.
    Open { tag: Option<String> },

    /// Close the currently open interval for the given tag, or the tag 'default'.
    Close { tag: Option<String> },

    /// List logged intervals.
    List {
        #[structopt(flatten)]
        info: TagsInRange,
    },

    /// Purge logged intervals.
    Purge {
        #[structopt(flatten)]
        info: TagsInRange,
    },

    /// Aggregate the durations of logged intervals.
    Aggregate {
        #[structopt(flatten)]
        info: TagsInRange,
    },

    /// Report open intervals.
    Status {
        /// Tags for which to see open intervals. If none are specified, see open intervals for all
        /// tags.
        tags: Vec<String>,
    },
}

#[derive(Debug, Clone, StructOpt)]
pub struct TagsInRange {
    /// Select only intervals that started before this time.
    #[structopt(short, long, parse(try_from_str = datetime_from_str))]
    before: Option<DateTime<Utc>>,

    /// Select only intervals that ended after this time (or are currently open).
    #[structopt(short, long, parse(try_from_str = datetime_from_str))]
    after: Option<DateTime<Utc>>,

    /// Select only open intervals. Mutually exclusive with --closed.
    #[structopt(short, long)]
    open: bool,

    /// Select only closed intervals. Mutually exclusive with --open.
    #[structopt(short, long)]
    closed: bool,

    /// Select only intervals with these tags. If none are given, select intervals with any tag.
    tags: Vec<String>,
}

impl TagsInRange {
    pub fn filter(&self, timelog: &TimeLog) -> Result<Filter, CommandError> {
        let tags_filter = if self.tags.is_empty() {
            Filter::True
        } else {
            filter::or_all(
                self.tags
                    .iter()
                    .map(|name| timelog.tag_id(name))
                    .filter(Option::is_some)
                    .map(Option::unwrap)
                    .map(filter::has_tag),
            )
        };

        let before_filter = if let Some(datetime) = self.before {
            filter::started_before(datetime)
        } else {
            Filter::True
        };

        let after_filter = if let Some(datetime) = self.after {
            filter::ended_after(datetime) | filter::is_open()
        } else {
            Filter::True
        };

        let open_closed_filter = {
            match (self.open, self.closed) {
                (true, true) => Err(CommandError::InconsistentFilter),
                (true, false) => Ok(filter::is_open()),
                (false, true) => Ok(filter::is_closed()),
                (false, false) => Ok(Filter::True),
            }
        }?;

        Ok(tags_filter & before_filter & after_filter & open_closed_filter)
    }
}

impl Command {
    pub fn execute(&self, timelog: &mut TimeLog) -> Result<ChangeStatus, CommandError> {
        match self {
            Command::Open { tag } => open(
                &tag.as_ref().cloned().unwrap_or_else(|| "default".into()),
                timelog,
            ),
            Command::Close { tag } => close(
                &tag.as_ref().cloned().unwrap_or_else(|| "default".into()),
                timelog,
            ),
            Command::List { info } => list(info, timelog),
            Command::Purge { info } => purge(info, timelog),
            Command::Aggregate { info } => aggregate(info, timelog),
            Command::Status { tags } => status(tags.as_ref(), timelog),
        }
    }
}

fn open(tag: &str, timelog: &mut TimeLog) -> Result<ChangeStatus, CommandError> {
    match timelog.open(tag) {
        Ok(int) => {
            let start = Local.from_utc_datetime(&int.start().naive_utc());
            println!(
                "Opened new interval for tag '{}' at {}",
                tag,
                start.format(interval::FMT_STR)
            );
            Ok(ChangeStatus::Changed)
        }
        Err(err) => {
            eprintln!("Error opening new interval for tag '{}': {}", tag, err);
            Err(err.into())
        }
    }
}

fn close(tag: &str, timelog: &mut TimeLog) -> Result<ChangeStatus, CommandError> {
    match timelog.close(tag) {
        Ok(int) => {
            println!("Closed interval for tag '{}': {}", tag, int.interval());
            Ok(ChangeStatus::Changed)
        }
        Err(err) => {
            eprintln!("Error closing interval for tag '{}': {}", tag, err);
            Err(err.into())
        }
    }
}

fn list(info: &TagsInRange, timelog: &TimeLog) -> Result<ChangeStatus, CommandError> {
    let filter = info.filter(timelog)?;
    list_filter(&filter, timelog);
    Ok(ChangeStatus::Unchanged)
}

fn list_filter(filter: &Filter, timelog: &TimeLog) {
    for int in timelog.iter().filter(filter.closure_ref()) {
        let tag = timelog.tag_name(int.tag()).unwrap();
        println!("{}: {}", tag, int.interval());
    }
}

fn purge(info: &TagsInRange, timelog: &mut TimeLog) -> Result<ChangeStatus, CommandError> {
    let filter = info.filter(timelog)?;

    if timelog.iter().any(filter.closure()) {
        if filter == Filter::True {
            println!("Purging ALL INTERVALS!");
        } else {
            println!("Purging the following intervals:");
            list_filter(&filter, timelog);
        }

        if user_confirmation(false) {
            println!("Purging.");
            timelog.remove(&filter);
            timelog.gc_tag_names();
            Ok(ChangeStatus::Changed)
        } else {
            println!("Purge cancelled.");
            Ok(ChangeStatus::Unchanged)
        }
    } else {
        println!("No intervals match filter criteria; purge cancelled.");
        Ok(ChangeStatus::Unchanged)
    }
}

fn aggregate(info: &TagsInRange, timelog: &TimeLog) -> Result<ChangeStatus, CommandError> {
    let filter = info.filter(timelog)?;

    println!("Aggregating the following intervals:");
    list_filter(&filter, timelog);

    let total = timelog
        .iter()
        .filter(filter.closure_ref())
        .fold(Duration::seconds(0), |d, int| d + int.duration());

    println!(
        "Total duration: {}:{:02}",
        total.num_hours(),
        total.num_minutes() % 60
    );

    Ok(ChangeStatus::Unchanged)
}

fn status(tags: &[String], timelog: &TimeLog) -> Result<ChangeStatus, CommandError> {
    let filter = if tags.is_empty() {
        filter::is_open()
    } else {
        let tags_filter = filter::or_all(
            tags.iter()
                .map(|name| timelog.tag_id(&name))
                .filter(|t| t.is_some())
                .map(|t| filter::has_tag(t.unwrap())),
        );

        filter::is_open() & tags_filter
    };

    if timelog.iter().any(filter.closure()) {
        println!("Currently open intervals:");
        list_filter(&filter, timelog);
    } else {
        println!("No currently open intervals matching these filter criteria.");
    }

    Ok(ChangeStatus::Unchanged)
}

fn user_confirmation(default: bool) -> bool {
    let options = if default { "(Y/n)" } else { "(y/N)" };

    let mut line = String::new();
    let mut result = default;

    loop {
        print!("Okay? {} ", options);
        io::stdout().flush().unwrap();
        io::stdin().read_line(&mut line).unwrap();

        let line_chars: Vec<_> = line.chars().collect();

        if line.len() < 2 {
            break;
        } else if line_chars.len() == 2 {
            match line_chars[0] {
                'y' | 'Y' => {
                    result = true;
                    break;
                }

                'n' | 'N' => {
                    result = false;
                    break;
                }

                _ => {
                    line.clear();
                    continue;
                }
            }
        } else {
            line.clear();
            continue;
        }
    }

    result
}

#[derive(Debug, Clone)]
pub enum CommandError {
    TimeLogError(TimeLogError),
    TimeParseError,
    InconsistentFilter,
}

impl Display for CommandError {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            CommandError::TimeLogError(err) => Display::fmt(err, f),
            CommandError::TimeParseError => write!(f, "error parsing time specification"),
            CommandError::InconsistentFilter => write!(f, "inconsistent filters specified"),
        }
    }
}

impl Error for CommandError {}

impl From<TimeLogError> for CommandError {
    fn from(err: TimeLogError) -> CommandError {
        CommandError::TimeLogError(err)
    }
}

fn datetime_from_str(s: &str) -> Result<DateTime<Utc>, CommandError> {
    const TIME_FMTS: &[&str] = &[
        "%-H:%M",   // H:MM
        "%-I:%M%P", // H:MM(am|pm)
        "%-I:%M%p", // H:MM(AM|PM)
    ];

    const DATE_FMTS: &[&str] = &[
        "%Y-%-m-%-d", // YYYY-M-D
        "%b%-d,%Y",   // MMMD,YYYY
    ];

    let now = Local::now();
    let s: String = s.chars().filter(|c| !c.is_whitespace()).collect();

    for fmt in TIME_FMTS {
        if let Ok(time) = NaiveTime::parse_from_str(&s, fmt) {
            let datetime = NaiveDateTime::new(now.naive_local().date(), time);
            let res = Utc.from_utc_datetime(&(datetime - now.offset().fix()));
            return Ok(res);
        }
    }

    for fmt in DATE_FMTS {
        if let Ok(date) = NaiveDate::parse_from_str(&s, fmt) {
            let datetime = NaiveDateTime::new(date, NaiveTime::from_hms(0, 0, 0));
            return Ok(Utc.from_local_datetime(&datetime).unwrap());
        }
    }

    for time_fmt in TIME_FMTS {
        for date_fmt in DATE_FMTS {
            let mut fmt = String::from(*date_fmt);
            fmt.push(',');
            fmt.push_str(*time_fmt);
            if let Ok(datetime) = NaiveDateTime::parse_from_str(&s, &fmt) {
                return Ok(Utc
                    .from_local_datetime(&(datetime - now.offset().fix()))
                    .unwrap());
            }
        }
    }

    Err(CommandError::TimeParseError)
}
