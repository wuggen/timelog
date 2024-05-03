use crate::filter::{self, Filter};
use crate::interval;
use crate::timelog::{TimeLog, TimeLogError};

use chrono::offset::Offset;
use chrono::{DateTime, Duration, Local, NaiveDate, NaiveDateTime, NaiveTime, TimeZone, Utc};
use structopt::StructOpt;

use std::collections::BTreeSet;
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
    Open {
        tag: Option<String>,

        /// Whether to allow creation of a new tag without prompt.
        #[structopt(short, long)]
        create: bool,
    },

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

    /// List current tags.
    Tags,
}

impl Command {
    pub fn execute<W>(
        &self,
        timelog: &mut TimeLog,
        outputs: Outputs<W>,
    ) -> Result<ChangeStatus, CommandError>
    where
        W: Write,
    {
        let mut context = CommandContext {
            command: self,
            timelog,
            outputs,
        };

        context.execute()
    }
}

struct CommandContext<'c, 't, W> {
    command: &'c Command,
    timelog: &'t mut TimeLog,
    outputs: Outputs<W>,
}

impl<'c, 't, W> CommandContext<'c, 't, W>
where
    W: Write,
{
    fn execute(&mut self) -> Result<ChangeStatus, CommandError> {
        match self.command {
            Command::Open { tag, create } => self.open(
                &tag.as_ref().cloned().unwrap_or_else(|| "default".into()),
                *create,
            ),
            Command::Close { tag } => {
                self.close(&tag.as_ref().cloned().unwrap_or_else(|| "default".into()))
            }
            Command::List { info } => {
                info.log_debug();
                self.list(info)
            }
            Command::Purge { info } => {
                info.log_debug();
                self.purge(info)
            }
            Command::Aggregate { info } => {
                info.log_debug();
                self.aggregate(info)
            }
            Command::Status { tags } => self.status(tags.as_ref()),

            Command::Tags => self.tags(),
        }
    }

    fn open(&mut self, tag: &str, create: bool) -> Result<ChangeStatus, CommandError> {
        if self.timelog.tag_id(tag).is_none() && tag != "default" && !create {
            writeln!(self.outputs.error_mut(), "Creating new tag '{}'.", tag)?;
            if !self.user_confirmation(false)? {
                writeln!(self.outputs.error_mut(), "Cancelling open")?;
                return Ok(ChangeStatus::Unchanged);
            }
        }

        match self.timelog.open(tag) {
            Ok(int) => {
                let start = Local.from_utc_datetime(&int.start().naive_utc());
                writeln!(
                    self.outputs.error_mut(),
                    "Opened new interval for tag '{}' at {}",
                    tag,
                    start.format(interval::FMT_STR)
                )?;
                Ok(ChangeStatus::Changed)
            }
            Err(err) => Err(err.into()),
        }
    }

    fn close(&mut self, tag: &str) -> Result<ChangeStatus, CommandError> {
        match self.timelog.close(tag) {
            Ok(int) => {
                writeln!(
                    self.outputs.error_mut(),
                    "Closed interval for tag '{}': {}",
                    tag,
                    int.interval()
                )?;
                Ok(ChangeStatus::Changed)
            }
            Err(err) => Err(err.into()),
        }
    }

    fn list(&mut self, info: &TagsInRange) -> Result<ChangeStatus, CommandError> {
        let filter = info.filter(self.timelog)?;
        self.list_filter(&filter)?;
        Ok(ChangeStatus::Unchanged)
    }

    fn list_filter(&mut self, filter: &Filter) -> Result<(), CommandError> {
        let max_tagwidth = self
            .timelog
            .iter()
            .filter_map(|int| {
                if filter.eval(int) {
                    Some(self.timelog.tag_name(int.tag()).unwrap().len())
                } else {
                    None
                }
            })
            .max()
            .unwrap_or(0);

        for int in self.timelog.iter().filter(filter.build_ref()) {
            let tag = self.timelog.tag_name(int.tag()).unwrap();
            writeln!(
                self.outputs.output_mut(),
                "{:<width$} | {}",
                tag,
                int.interval(),
                width = max_tagwidth
            )?;
        }

        Ok(())
    }

    fn purge(&mut self, info: &TagsInRange) -> Result<ChangeStatus, CommandError> {
        let filter = info.filter(self.timelog)?;
        let filter_fn = filter.build();

        if self.timelog.iter().any(&filter_fn) {
            if filter.evals_true() {
                writeln!(self.outputs.error_mut(), "Purging ALL INTERVALS!")?;
            } else {
                writeln!(self.outputs.error_mut(), "Purging the following intervals:")?;
                self.list_filter(&filter)?;
            }

            if self.user_confirmation(false)? {
                writeln!(self.outputs.error_mut(), "Purging.")?;
                self.timelog.remove(&filter_fn);
                self.timelog.gc_tag_names();
                Ok(ChangeStatus::Changed)
            } else {
                writeln!(self.outputs.error_mut(), "Purge cancelled.")?;
                Ok(ChangeStatus::Unchanged)
            }
        } else {
            writeln!(
                self.outputs.error_mut(),
                "No intervals match filter criteria; purge cancelled."
            )?;
            Ok(ChangeStatus::Unchanged)
        }
    }

    fn aggregate(&mut self, info: &TagsInRange) -> Result<ChangeStatus, CommandError> {
        let filter = info.filter(self.timelog)?;

        writeln!(
            self.outputs.error_mut(),
            "Aggregating the following intervals:"
        )?;
        self.list_filter(&filter)?;

        let filter = filter.build_ref();

        let total = self
            .timelog
            .iter()
            .filter(filter)
            .fold(Duration::seconds(0), |d, int| d + int.duration());

        writeln!(
            self.outputs.output_mut(),
            "Total {}:{:02}",
            total.num_hours(),
            total.num_minutes() % 60
        )?;

        Ok(ChangeStatus::Unchanged)
    }

    fn status(&mut self, tags: &[String]) -> Result<ChangeStatus, CommandError> {
        let filter = if tags.is_empty() {
            filter::is_open()
        } else {
            let tags_filter = filter::or_all(
                tags.iter()
                    .map(|name| self.timelog.tag_id(name))
                    .filter(|t| t.is_some())
                    .map(|t| filter::has_tag(t.unwrap())),
            );

            filter::is_open() & tags_filter
        };

        if self.timelog.iter().any(filter.build()) {
            writeln!(self.outputs.error_mut(), "Currently open intervals:")?;
            self.list_filter(&filter)?;
        } else {
            writeln!(
                self.outputs.error_mut(),
                "No currently open intervals matching these filter criteria."
            )?;
        }

        Ok(ChangeStatus::Unchanged)
    }

    fn tags(&mut self) -> Result<ChangeStatus, CommandError> {
        let tagnames: BTreeSet<_> = self
            .timelog
            .iter()
            .map(|int| String::from(self.timelog.tag_name(int.tag()).unwrap()))
            .collect();

        for name in tagnames {
            writeln!(self.outputs.output_mut(), "{}", name)?;
        }

        Ok(ChangeStatus::Unchanged)
    }

    fn user_confirmation(&mut self, default: bool) -> Result<bool, CommandError> {
        let options = if default { "(Y/n)" } else { "(y/N)" };

        let mut line = String::new();
        let mut result = default;

        loop {
            write!(self.outputs.error_mut(), "Okay? {} ", options)?;
            self.outputs.error_mut().flush()?;
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
                let upper = line.to_uppercase();
                match upper.as_ref() {
                    "YES\n" => {
                        result = true;
                        break;
                    }

                    "NO\n" => {
                        result = false;
                        break;
                    }

                    _ => {
                        line.clear();
                        continue;
                    }
                }
            }
        }

        Ok(result)
    }
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
                    filter::ended_after_strict(todaytime)
                } else {
                    filter::ended_after_strict(aftertime)
                }
        } else if self.today {
            filter::is_open() | filter::ended_after_strict(todaytime)
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
        log::debug!("TagsInRange filter: {:?}", res);

        Ok(res)
    }

    fn log_debug(&self) {
        if let Some(before) = self.before {
            log::debug!("Before time: {}", before);
        }

        if let Some(after) = self.after {
            log::debug!("After time: {}", after);
        }
    }
}

#[derive(Debug, Clone)]
pub struct Outputs<W> {
    pub output: W,
    pub error: Option<W>,
}

impl<W> Outputs<W>
where
    W: Write,
{
    pub fn new(output: W, error: Option<W>) -> Outputs<W> {
        Outputs { output, error }
    }

    pub fn output(&self) -> &W {
        &self.output
    }

    pub fn output_mut(&mut self) -> &mut W {
        &mut self.output
    }

    pub fn error(&self) -> &W {
        self.error.as_ref().unwrap_or(&self.output)
    }

    pub fn error_mut(&mut self) -> &mut W {
        self.error.as_mut().unwrap_or(&mut self.output)
    }
}

pub type StdOutputs = Outputs<Box<dyn Write>>;

impl Default for StdOutputs {
    fn default() -> StdOutputs {
        Outputs {
            output: Box::new(io::stdout()),
            error: Some(Box::new(io::stderr())),
        }
    }
}

#[derive(Debug)]
pub enum CommandError {
    TimeLogError(TimeLogError),
    TimeParseError,
    InconsistentFilter,
    IoError(io::Error),
}

impl Display for CommandError {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            CommandError::TimeLogError(err) => Display::fmt(err, f),
            CommandError::TimeParseError => write!(f, "error parsing time specification"),
            CommandError::InconsistentFilter => write!(f, "inconsistent filters specified"),
            CommandError::IoError(err) => write!(f, "{}", err),
        }
    }
}

impl Error for CommandError {}

impl From<TimeLogError> for CommandError {
    fn from(err: TimeLogError) -> CommandError {
        CommandError::TimeLogError(err)
    }
}

impl From<io::Error> for CommandError {
    fn from(err: io::Error) -> CommandError {
        CommandError::IoError(err)
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
            fmt.push_str(time_fmt);
            if let Ok(datetime) = NaiveDateTime::parse_from_str(&s, &fmt) {
                return Ok(Utc
                    .from_local_datetime(&(datetime - now.offset().fix()))
                    .unwrap());
            }
        }
    }

    if let Some(c @ ('+' | '-')) = s.chars().next() {
        let s = &s[1..];
        let dur = duration_from_str(s)?;
        if c == '+' {
            Ok(Utc::now() + dur)
        } else {
            Ok(Utc::now() - dur)
        }
    } else {
        Err(CommandError::TimeParseError)
    }
}

fn duration_from_str(s: &str) -> Result<Duration, CommandError> {
    let tokens: Vec<_> = s.split(':').collect();

    let (hours, minutes, seconds) = if tokens.len() == 1 {
        (
            tokens[0]
                .parse::<u64>()
                .map_err(|_| CommandError::TimeParseError)?,
            0,
            0,
        )
    } else if tokens.len() == 2 {
        (
            tokens[0]
                .parse::<u64>()
                .map_err(|_| CommandError::TimeParseError)?,
            tokens[1]
                .parse::<u64>()
                .map_err(|_| CommandError::TimeParseError)?,
            0,
        )
    } else if tokens.len() == 3 {
        (
            tokens[0]
                .parse::<u64>()
                .map_err(|_| CommandError::TimeParseError)?,
            tokens[1]
                .parse::<u64>()
                .map_err(|_| CommandError::TimeParseError)?,
            tokens[2]
                .parse::<u64>()
                .map_err(|_| CommandError::TimeParseError)?,
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
