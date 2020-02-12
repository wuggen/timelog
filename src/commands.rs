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

    /// Select only intervals that ended after the most recent midnight (or are currently open).
    #[structopt(long)]
    today: bool,

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
            filter::filter_true()
        } else {
            filter::or_all(self.tags.iter().filter_map(|name| {
                let tag = timelog.tag_id(name)?;
                Some(filter::has_tag(tag))
            }))
        };

        let todaytime = Local::today().and_hms(0, 0, 0);
        let todaytime = (todaytime - todaytime.offset().fix()).with_timezone(&Utc);
        let tomorrowtime = todaytime + Duration::days(1);

        let before_filter = if let Some(beforetime) = self.before {
            if self.today && tomorrowtime < beforetime {
                filter::started_before(tomorrowtime)
            } else {
                filter::started_before(beforetime)
            }
        } else if self.today {
            filter::started_before(tomorrowtime)
        } else {
            filter::filter_true()
        };

        let after_filter = if let Some(aftertime) = self.after {
            filter::is_open()
                | if self.today && aftertime < todaytime {
                    filter::ended_after(todaytime)
                } else {
                    filter::ended_after(aftertime)
                }
        } else if self.today {
            filter::is_open() | filter::ended_after(todaytime)
        } else {
            filter::filter_true()
        };

        let open_closed_filter = {
            match (self.open, self.closed) {
                (true, true) => Err(CommandError::InconsistentFilter),
                (true, false) => Ok(filter::is_open()),
                (false, true) => Ok(filter::is_closed()),
                (false, false) => Ok(filter::filter_true()),
            }
        }?;

        let res = tags_filter & before_filter & after_filter & open_closed_filter;
        debug!("TagsInRange filter: {:?}", res);

        Ok(res)
    }

    fn log_debug(&self) {
        if let Some(before) = self.before {
            debug!("Before time: {}", before);
        }

        if let Some(after) = self.after {
            debug!("After time: {}", after);
        }
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
            Command::List { info } => {
                info.log_debug();
                list(info, timelog)
            }
            Command::Purge { info } => {
                info.log_debug();
                purge(info, timelog)
            }
            Command::Aggregate { info } => {
                info.log_debug();
                aggregate(info, timelog)
            }
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
    for int in timelog.iter().filter(filter.build_ref()) {
        let tag = timelog.tag_name(int.tag()).unwrap();
        println!("{}: {}", tag, int.interval());
    }
}

fn purge(info: &TagsInRange, timelog: &mut TimeLog) -> Result<ChangeStatus, CommandError> {
    let filter = info.filter(timelog)?;
    let filter_fn = filter.build();

    if timelog.iter().any(&filter_fn) {
        if filter.evals_true() {
            println!("Purging ALL INTERVALS!");
        } else {
            println!("Purging the following intervals:");
            list_filter(&filter, timelog);
        }

        if user_confirmation(false) {
            println!("Purging.");
            timelog.remove(&filter_fn);
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

    let filter = filter.build_ref();

    let total = timelog
        .iter()
        .filter(filter)
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

    if timelog.iter().any(filter.build()) {
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
            let datetime =
                NaiveDateTime::new(date, NaiveTime::from_hms(0, 0, 0)) - now.offset().fix();
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

    match s.chars().nth(0) {
        Some(c) if c == '+' || c == '-' => {
            let s = &s[1..];
            let dur = duration_from_str(s)?;
            if c == '+' {
                return Ok(Utc::now() + dur);
            } else {
                return Ok(Utc::now() - dur);
            }
        }
        _ => (),
    }

    Err(CommandError::TimeParseError)
}

fn duration_from_str(s: &str) -> Result<Duration, CommandError> {
    let tokens: Vec<_> = s.split(':').collect();

    let (hours, minutes, seconds) = if tokens.len() == 1 {
        (
            u64::from_str_radix(tokens[0], 10).map_err(|_| CommandError::TimeParseError)?,
            0,
            0,
        )
    } else if tokens.len() == 2 {
        (
            u64::from_str_radix(tokens[0], 10).map_err(|_| CommandError::TimeParseError)?,
            u64::from_str_radix(tokens[1], 10).map_err(|_| CommandError::TimeParseError)?,
            0,
        )
    } else if tokens.len() == 3 {
        (
            u64::from_str_radix(tokens[0], 10).map_err(|_| CommandError::TimeParseError)?,
            u64::from_str_radix(tokens[1], 10).map_err(|_| CommandError::TimeParseError)?,
            u64::from_str_radix(tokens[2], 10).map_err(|_| CommandError::TimeParseError)?,
        )
    } else {
        return Err(CommandError::TimeParseError);
    };

    if minutes >= 60 || seconds >= 60 {
        return Err(CommandError::TimeParseError);
    }

    Ok(Duration::seconds(
        seconds as i64 + 60 * minutes as i64 + 60 * 60 * hours as i64,
    ))
}
