//! Time interval types and definitions.

use crate::tags::TagId;

use chrono::{DateTime, Duration, Local, TimeZone, Timelike, Utc};
use serde::{Deserialize, Serialize};

use std::ops::Add;
use std::time::Duration as StdDuration;

use std::fmt::{self, Display, Formatter};

pub static FMT_STR: &str = "%a %F %I:%M%P";

/// A possibly-open time interval.
///
/// An interval is represented by a start time and, if it is closed, a duration.
#[derive(Debug, Clone, Copy, PartialOrd, Ord, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Interval {
    start: DateTime<Utc>,
    duration: Option<StdDuration>,
}

impl Interval {
    /// Open a new interval at the given time.
    pub fn open(start: DateTime<Utc>) -> Interval {
        Interval {
            start,
            duration: None,
        }
    }

    /// Close this interval at the given time.
    ///
    /// Returns `None` if the given end time is before this interval's start time.
    pub fn close(&self, end: DateTime<Utc>) -> Option<Interval> {
        let Interval { start, duration } = *self;

        if duration.is_none() {
            let duration = end.signed_duration_since(start);
            if duration < Duration::zero() {
                None
            } else {
                let duration = Some(duration.to_std().unwrap());
                Some(Interval { start, duration })
            }
        } else {
            None
        }
    }

    /// Open a new interval at the current time.
    pub fn open_now() -> Interval {
        Interval::open(Utc::now())
    }

    /// Close this interval at the current time.
    ///
    /// Returns `None` if the start time of this interval is in the future.
    pub fn close_now(&self) -> Option<Interval> {
        self.close(Utc::now())
    }

    /// Create an interval with the given start time and duration.
    pub fn closed(start: DateTime<Utc>, duration: StdDuration) -> Interval {
        Interval {
            start,
            duration: Some(duration),
        }
    }

    /// Round the start time back to the nearest quarter hour, and the end time forward to the
    /// nearest quarter hour.
    pub fn round_to_quarter_hours(self) -> Interval {
        let start = QuarterHour::floor(&self.start());
        let duration = self
            .end()
            .and_then(|end| (QuarterHour::ceil(&end) - start).to_std().ok());

        Interval { start, duration }
    }

    /// Is this interval closed?
    pub fn is_closed(&self) -> bool {
        self.duration.is_some()
    }

    /// Get the start time of this interval.
    pub fn start(&self) -> DateTime<Utc> {
        self.start
    }

    /// Get the end time of this interval, if it is closed.
    pub fn end(&self) -> Option<DateTime<Utc>> {
        self.duration
            .map(|d| self.start + Duration::from_std(d).unwrap())
    }

    /// Get the duration of this interval.
    ///
    /// If the interval is still open, this will return the duration elapsed between its start time
    /// and the current time. If the start time is in the future, this duration will be negative.
    pub fn duration(&self) -> Duration {
        self.duration
            .map(|d| Duration::from_std(d).unwrap())
            .unwrap_or_else(|| ceil_time(&Utc::now()).signed_duration_since(self.start))
    }
}

impl Display for Interval {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        let start = Local.from_utc_datetime(&self.start.naive_utc());

        fn fmt_duration(dur: Duration) -> String {
            format!("{}:{:02}", dur.num_hours(), dur.num_minutes() % 60)
        }

        match self.end() {
            Some(end) => {
                let end = Local.from_utc_datetime(&end.naive_utc());
                write!(
                    f,
                    "{} -- {} ({})",
                    start.format(FMT_STR),
                    end.format(FMT_STR),
                    fmt_duration(self.duration()),
                )
            }

            None => write!(
                f,
                "{} -- OPEN ({})",
                start.format(FMT_STR),
                fmt_duration(self.duration()),
            ),
        }
    }
}

/// A time interval with an associated tag.
#[derive(Debug, Clone, Copy, PartialOrd, Ord, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TaggedInterval {
    tag: TagId,
    interval: Interval,
}

impl TaggedInterval {
    /// Create a new tagged interval with the given tag ID and interval.
    pub fn new(tag: TagId, interval: Interval) -> TaggedInterval {
        TaggedInterval { tag, interval }
    }

    /// Get the tag ID of this tagged interval.
    pub fn tag(&self) -> TagId {
        self.tag
    }

    /// Get the interval of this tagged interval.
    pub fn interval(&self) -> &Interval {
        &self.interval
    }

    /// Get a mutable reference to the interval of this tagged interval.
    pub fn interval_mut(&mut self) -> &mut Interval {
        &mut self.interval
    }

    /// Open a new interval with the given tag at the given start time.
    pub fn open(tag: TagId, start: DateTime<Utc>) -> TaggedInterval {
        let interval = Interval::open(start);
        TaggedInterval { tag, interval }
    }

    /// Close this tagged interval at the given end time.
    ///
    /// Returns `None` if the given end time is before this interval's start time.
    pub fn close(&self, end: DateTime<Utc>) -> Option<TaggedInterval> {
        let interval = self.interval.close(end)?;
        Some(TaggedInterval { interval, ..*self })
    }

    /// Open a new interval with the given tag at the current time.
    pub fn open_now(tag: TagId) -> TaggedInterval {
        let interval = Interval::open_now();
        TaggedInterval { tag, interval }
    }

    /// Close this tagged interval at the current time.
    ///
    /// Returns `None` if this interval's start time is in the future.
    pub fn close_now(&self) -> Option<TaggedInterval> {
        let interval = self.interval.close_now()?;
        Some(TaggedInterval { interval, ..*self })
    }

    /// Is this tagged interval closed?
    pub fn is_closed(&self) -> bool {
        self.interval.is_closed()
    }

    /// Get the start time of this tagged interval.
    pub fn start(&self) -> DateTime<Utc> {
        self.interval.start()
    }

    /// Get the end time of this tagged interval, if it is closed.
    pub fn end(&self) -> Option<DateTime<Utc>> {
        self.interval.end()
    }

    /// Get the duration of this tagged interval.
    ///
    /// If the interval is not yet closed, this will return the duration elapsed between the
    /// interval's start time and the current time. If the start time is in the future, this
    /// duration will be negative.
    pub fn duration(&self) -> Duration {
        self.interval.duration()
    }

    /// Round the start time back to the nearest quarter hour, and the end time forward to the
    /// nearest quarter hour.
    pub fn round_to_quarter_hours(&self) -> TaggedInterval {
        let interval = self.interval.round_to_quarter_hours();
        TaggedInterval { interval, ..*self }
    }
}

/// Attach a tag to an interval.
pub fn tag(tag: TagId, interval: Interval) -> TaggedInterval {
    TaggedInterval::new(tag, interval)
}

/// Quarter hour increments. Utility type for rounding times to adjacent quarter hours.
#[derive(Debug, Clone, Copy, PartialOrd, Ord, PartialEq, Eq, Hash)]
enum QuarterHour {
    /// 0 to 14 minutes past the hour.
    TopOfHour,

    /// 15 to 29 minutes past the hour.
    QuarterPast,

    /// 30 to 44 minutes past the hour.
    HalfPast,

    /// 45 to 59 minutes past the hour.
    QuarterTill,
}

use QuarterHour::*;

impl QuarterHour {
    /// Get the quarter hour in which the given time resides.
    fn of<T>(time: &T) -> QuarterHour
    where
        T: Timelike,
    {
        let minute = time.minute();
        if minute < 15 {
            TopOfHour
        } else if minute < 30 {
            QuarterPast
        } else if minute < 45 {
            HalfPast
        } else {
            QuarterTill
        }
    }

    /// Get the starting minute of this quarter hour.
    fn minute(self) -> u32 {
        match self {
            TopOfHour => 0,
            QuarterPast => 15,
            HalfPast => 30,
            QuarterTill => 45,
        }
    }

    /// Round the given time to the quarter-hour increment most recently preceding it.
    fn floor<T>(time: &T) -> T
    where
        T: Timelike,
    {
        let qh = QuarterHour::of(time);
        time.with_minute(qh.minute())
            .unwrap()
            .with_second(0)
            .unwrap()
            .with_nanosecond(0)
            .unwrap()
    }

    /// Round the given time to the quarter-hour increment most closely following it.
    fn ceil<T>(time: &T) -> <T as Add<Duration>>::Output
    where
        T: Timelike + Add<Duration> + Clone,
        <T as Add<Duration>>::Output: Timelike,
    {
        let time = time.clone() + Duration::seconds(14 * 60 + 59);
        let qh = QuarterHour::of(&time);
        time.with_minute(qh.minute())
            .unwrap()
            .with_second(0)
            .unwrap()
            .with_nanosecond(0)
            .unwrap()
    }
}

/// Round the given time to the quarter-hour increment most recently preceding it.
pub fn floor_time<T>(time: &T) -> T
where
    T: Timelike,
{
    QuarterHour::floor(time)
}

/// Round the given time to the quarter-hour increment most closely following it.
pub fn ceil_time<T>(time: &T) -> <T as Add<Duration>>::Output
where
    T: Timelike + Add<Duration> + Clone,
    <T as Add<Duration>>::Output: Timelike,
{
    QuarterHour::ceil(time)
}
