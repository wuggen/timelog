use std::fmt::{self, Display, Formatter};
use std::ops::Add;
use std::time::Duration;

use chrono::{prelude::*, TimeDelta};
use serde::{Deserialize, Serialize};

/// A closed interval of time, defined by a start time and a duration.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct Interval {
    start: RestrictedDateTime,
    duration: RestrictedDuration,
}

impl Interval {
    /// Create a new interval with the given start time and duration.
    pub fn new(start: RestrictedDateTime, duration: RestrictedDuration) -> Self {
        Self { start, duration }
    }

    /// Get the start time of this interval.
    pub fn start_time(&self) -> RestrictedDateTime {
        self.start
    }

    /// Get the duration of this interval.
    pub fn duration(&self) -> RestrictedDuration {
        self.duration
    }

    /// Get the end time of this interval.
    pub fn end_time(&self) -> RestrictedDateTime {
        self.start + self.duration
    }
}

/// A duration of time restricted to 15-minute intervals.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct RestrictedDuration {
    hours: u32,
    quarter_hour: QuarterHour,
}

impl Display for RestrictedDuration {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        Display::fmt(&TimeDelta::from(*self), f)
    }
}

impl RestrictedDuration {
    /// Create a new `RestrictedDuration`.
    pub fn new(hours: u32, quarter_hour: QuarterHour) -> Self {
        Self {
            hours,
            quarter_hour,
        }
    }

    /// Create a new `RestrictedDuration` from a total number of minutes.
    pub fn from_minutes(minutes: u32) -> Self {
        let hours = minutes / 60;
        let quarter_hour = QuarterHour::from_minutes(minutes);
        Self {
            hours,
            quarter_hour,
        }
    }

    /// Get the whole hours of this duration.
    pub fn hours(&self) -> u32 {
        self.hours
    }

    /// Get the total number of minutes of this duration.
    pub fn minutes(&self) -> u32 {
        self.hours() * 60 + self.quarter_hour.minute()
    }

    /// Get the quarter hour of this duration.
    pub fn quarter_hour(&self) -> QuarterHour {
        self.quarter_hour
    }

    /// Increment this duration to the next quarter hour.
    pub fn increment(&self) -> Self {
        let quarter_hour = self.quarter_hour.increment();
        let hours = if quarter_hour == QuarterHour::Q0 {
            self.hours + 1
        } else {
            self.hours
        };

        Self {
            hours,
            quarter_hour,
        }
    }

    /// Get the quarter-hour floor of the given duration.
    pub fn floor_duration(duration: Duration) -> Self {
        Self::from(duration)
    }

    /// Get the quarter-hour ceiling of the given duration.
    pub fn ceil_duration(duration: Duration) -> Self {
        Self::from(duration + Duration::from_secs(14 * 60))
    }

    /// Get the quarter-hour floor of the given time delta.
    pub fn try_floor_delta(delta: TimeDelta) -> Result<Self, NegativeDeltaError> {
        Self::try_from(delta)
    }

    /// Get the quarter-hour ceiling of the given time delta.
    pub fn try_ceil_delta(delta: TimeDelta) -> Result<Self, NegativeDeltaError> {
        Self::try_from(delta + TimeDelta::try_minutes(14).unwrap())
    }
}

impl From<Duration> for RestrictedDuration {
    /// Returns the quarter-hour floor of the given duration.
    fn from(value: Duration) -> Self {
        let minutes = (value.as_secs() / 60) as u32;
        let hours = minutes / 60;
        let quarter_hour = QuarterHour::from_minutes(minutes);
        Self {
            hours,
            quarter_hour,
        }
    }
}

impl From<RestrictedDuration> for Duration {
    fn from(value: RestrictedDuration) -> Self {
        Duration::from_secs(
            (value.hours() as u64 * 3600) + (value.quarter_hour().minute() as u64 * 60),
        )
    }
}

/// Error returned when attempting to convert a negative `TimeDelta` to a `RestrictedDuration`.
///
/// Contains the original `TimeDelta`.
#[derive(Debug, Clone, Copy, thiserror::Error)]
#[error("cannot convert negative time delta to `RestrictedDuration`")]
pub struct NegativeDeltaError(pub TimeDelta);

impl TryFrom<TimeDelta> for RestrictedDuration {
    type Error = NegativeDeltaError;

    /// Returns the quarter-hour floor of the given delta.
    fn try_from(value: TimeDelta) -> Result<Self, Self::Error> {
        if value < TimeDelta::zero() {
            Err(NegativeDeltaError(value))
        } else {
            let hours = value.num_hours() as u32;
            let quarter_hour = QuarterHour::from_minutes(value.num_minutes() as u32);
            Ok(Self {
                hours,
                quarter_hour,
            })
        }
    }
}

impl From<RestrictedDuration> for TimeDelta {
    fn from(value: RestrictedDuration) -> Self {
        TimeDelta::new(
            (value.hours() as i64 * 3600) + (value.quarter_hour().minute() as i64 * 60),
            0,
        )
        .unwrap()
    }
}

/// A time of day restricted to quarter-hour increments.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct RestrictedTime {
    hour: u32,
    quarter_hour: QuarterHour,
}

impl Display for RestrictedTime {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        <NaiveTime as Display>::fmt(&(*self).into(), f)
    }
}

impl Timelike for RestrictedTime {
    fn hour(&self) -> u32 {
        self.hour
    }

    fn minute(&self) -> u32 {
        self.quarter_hour.minute()
    }

    /// Always 0; `RestrictedTime` only has quarter-hour resolution.
    fn second(&self) -> u32 {
        0
    }

    /// Always 0; `RestrictedTime` only has quarter-hour resolution.
    fn nanosecond(&self) -> u32 {
        0
    }

    fn with_hour(&self, hour: u32) -> Option<Self> {
        if hour < 24 {
            Some(Self { hour, ..*self })
        } else {
            None
        }
    }

    /// Sets the minute to the quarter-hour floor of the given minute.
    fn with_minute(&self, min: u32) -> Option<Self> {
        if min < 60 {
            let quarter_hour = QuarterHour::from_minutes(min);
            Some(Self {
                quarter_hour,
                ..*self
            })
        } else {
            None
        }
    }

    /// No-op; `RestrictedTime` only has quarter-hour resolution.
    fn with_second(&self, sec: u32) -> Option<Self> {
        if sec < 60 {
            Some(*self)
        } else {
            None
        }
    }

    /// No-op; `RestrictedTime` only has quarter-hour resolution.
    fn with_nanosecond(&self, nano: u32) -> Option<Self> {
        if nano < 1_000_000_000 {
            Some(*self)
        } else {
            None
        }
    }
}

impl RestrictedTime {
    /// Create a new `RestrictedTime`.
    ///
    /// Returns `None` if the given hour is 24 or greater.
    pub fn new(hour: u32, quarter_hour: QuarterHour) -> Option<Self> {
        if hour < 24 {
            Some(Self { hour, quarter_hour })
        } else {
            None
        }
    }

    /// Get the quarter hour of this `RestrictedTime`.
    pub fn quarter_hour(&self) -> QuarterHour {
        self.quarter_hour
    }

    /// Increment this `RestrictedTime` to the next quarter hour.
    ///
    /// This method wraps to zero if the resulting hour would be 24 or greater.
    pub fn increment(&self) -> Self {
        let quarter_hour = self.quarter_hour.increment();
        let hour = if quarter_hour == QuarterHour::Q0 {
            (self.hour + 1) % 24
        } else {
            self.hour
        };
        Self { hour, quarter_hour }
    }

    /// Get the quarter-hour floor of the given `NaiveTime`.
    pub fn floor_naive(time: NaiveTime) -> Self {
        Self::from(time)
    }

    /// Get the quarter-hour ceiling of the given `NaiveTime`.
    pub fn ceil_naive(time: NaiveTime) -> Self {
        Self::from(time + TimeDelta::new(14 * 60, 0).unwrap())
    }

    /// Get the quarter-hour floor of the current time, in UTC.
    pub fn now_floor() -> Self {
        let now = Utc::now().naive_utc().time();
        Self::floor_naive(now)
    }

    /// Get the quarter-hour ceiling of the current time, in UTC.
    pub fn now_ceil() -> Self {
        let now = Utc::now().naive_utc().time();
        Self::ceil_naive(now)
    }
}

impl Add<RestrictedDuration> for RestrictedTime {
    type Output = Self;

    fn add(self, rhs: RestrictedDuration) -> Self::Output {
        let self_minutes = self.hour * 60 + self.quarter_hour.minute();
        let rhs_minutes = rhs.minutes();
        let total_minutes = self_minutes + rhs_minutes;
        let hour = (total_minutes / 60) % 24;
        let quarter_hour = QuarterHour::from_minutes(total_minutes);
        Self { hour, quarter_hour }
    }
}

impl From<NaiveTime> for RestrictedTime {
    /// Returns the quarter-hour floor of the given time.
    fn from(value: NaiveTime) -> Self {
        let hour = value.hour();
        let minute = value.minute();
        let quarter_hour = QuarterHour::from_minutes(minute);
        Self { hour, quarter_hour }
    }
}

impl From<RestrictedTime> for NaiveTime {
    fn from(value: RestrictedTime) -> Self {
        NaiveTime::from_hms_opt(value.hour, value.quarter_hour.minute(), 0).unwrap()
    }
}

/// A date and time restricted to quarter-hour increments.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct RestrictedDateTime {
    date: NaiveDate,
    time: RestrictedTime,
}

impl RestrictedDateTime {
    /// Get the date of this `RestrictedDateTime`.
    pub fn date(&self) -> NaiveDate {
        self.date
    }

    /// Get the time of this `RestrictedDateTime`.
    pub fn time(&self) -> RestrictedTime {
        self.time
    }

    /// Get the quarter-hour floor of the given `NaiveDateTime`.
    pub fn floor_naive(datetime: NaiveDateTime) -> Self {
        Self::from(datetime)
    }

    /// Get the quarter-hour ceiling of the given `NaiveDateTime`.
    pub fn ceil_naive(datetime: NaiveDateTime) -> Self {
        Self::from(datetime + TimeDelta::new(14 * 60, 0).unwrap())
    }

    /// Get the quarter-hour floor of the current datetime, in UTC.
    pub fn now_floor() -> Self {
        let now = Utc::now().naive_utc();
        Self::floor_naive(now)
    }

    /// Get the quarter-hour ceiling of the current datetime, in UTC.
    pub fn now_ceil() -> Self {
        let now = Utc::now().naive_utc();
        Self::ceil_naive(now)
    }
}

impl Display for RestrictedDateTime {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        <NaiveDateTime as Display>::fmt(&(*self).into(), f)
    }
}

macro_rules! timelike_passthrough {
    ($field:ident) => {
        timelike_passthrough!($field | hour() -> u32);
        timelike_passthrough!($field | minute() -> u32);
        timelike_passthrough!($field | second() -> u32);
        timelike_passthrough!($field | nanosecond() -> u32);
        timelike_passthrough!($field | with_hour(u32));
        timelike_passthrough!($field | with_minute(u32));
        timelike_passthrough!($field | with_second(u32));
        timelike_passthrough!($field | with_nanosecond(u32));
    };

    ($field:ident | $method:ident () -> $ret:ty) => {
        fn $method (&self) -> $ret {
            self.$field.$method()
        }
    };

    ($field:ident | $method:ident ( $ty:ty ) ) => {
        fn $method (&self, val: $ty) -> Option<Self> {
            let $field = self.$field.$method(val)?;
            Some(Self { $field, ..*self })
        }
    };
}

impl Timelike for RestrictedDateTime {
    timelike_passthrough!(time);
}

macro_rules! datelike_passthrough {
    ($field:ident) => {
        datelike_passthrough!($field | year() -> i32);
        datelike_passthrough!($field | month() -> u32);
        datelike_passthrough!($field | month0() -> u32);
        datelike_passthrough!($field | day() -> u32);
        datelike_passthrough!($field | day0() -> u32);
        datelike_passthrough!($field | ordinal() -> u32);
        datelike_passthrough!($field | ordinal0() -> u32);
        datelike_passthrough!($field | weekday() -> Weekday);
        datelike_passthrough!($field | iso_week() -> chrono::IsoWeek);
        datelike_passthrough!($field | with_year(i32));
        datelike_passthrough!($field | with_month(u32));
        datelike_passthrough!($field | with_month0(u32));
        datelike_passthrough!($field | with_day(u32));
        datelike_passthrough!($field | with_day0(u32));
        datelike_passthrough!($field | with_ordinal(u32));
        datelike_passthrough!($field | with_ordinal0(u32));
    };

    ($field:ident | $method:ident () -> $ret:ty) => {
        fn $method (&self) -> $ret {
            self.$field.$method()
        }
    };

    ($field:ident | $method:ident ( $ty:ty )) => {
        fn $method (&self, val: $ty) -> Option<Self> {
            let $field = self.$field.$method(val)?;
            Some(Self { $field, ..*self })
        }
    };
}

impl Datelike for RestrictedDateTime {
    datelike_passthrough!(date);
}

impl From<NaiveDateTime> for RestrictedDateTime {
    /// Returns the quarter-hour floor of the given date-time.
    fn from(value: NaiveDateTime) -> Self {
        let date = value.date();
        let time = RestrictedTime::from(value.time());
        Self { date, time }
    }
}

impl From<RestrictedDateTime> for NaiveDateTime {
    fn from(value: RestrictedDateTime) -> Self {
        NaiveDateTime::new(value.date, NaiveTime::from(value.time))
    }
}

impl Add<RestrictedDuration> for RestrictedDateTime {
    type Output = Self;

    fn add(self, rhs: RestrictedDuration) -> Self::Output {
        let datetime = NaiveDateTime::from(self);
        let duration = TimeDelta::from(rhs);
        let res = datetime + duration;
        Self::from(res)
    }
}

/// A 15-minute division of an hour.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum QuarterHour {
    /// 0 to 14 minutes past the hour.
    Q0,
    /// 15 to 29 minutes past the hour.
    Q15,
    /// 30 to 44 minutes past the hour.
    Q30,
    /// 45 to 59 minutes past the hour.
    Q45,
}

impl QuarterHour {
    /// Get the `QuarterHour` in which the given number of minutes falls.
    ///
    /// This method operates modularly; that is, it will take the quarter hour of the given number
    /// of minutes modulo 60.
    pub fn from_minutes(minutes: u32) -> Self {
        Self::from_int(minutes / 15)
    }

    /// Get the `QuarterHour` in which the given time falls.
    ///
    /// ```
    /// # use chrono::NaiveTime;
    /// # use timelog::v2::interval::QuarterHour;
    /// assert_eq!(
    ///     QuarterHour::of(&NaiveTime::from_hms_opt(0, 12, 10).unwrap()),
    ///     QuarterHour::Q0,
    /// );
    /// assert_eq!(
    ///     QuarterHour::of(&NaiveTime::from_hms_opt(13, 29, 59).unwrap()),
    ///     QuarterHour::Q15,
    /// );
    /// assert_eq!(
    ///     QuarterHour::of(&NaiveTime::from_hms_opt(23, 34, 0).unwrap()),
    ///     QuarterHour::Q30,
    /// );
    /// assert_eq!(
    ///     QuarterHour::of(&NaiveTime::from_hms_opt(10, 45, 0).unwrap()),
    ///     QuarterHour::Q45,
    /// );
    /// ```
    pub fn of<T>(time: &T) -> Self
    where
        T: Timelike,
    {
        Self::from_minutes(time.minute())
    }

    /// Get the `QuarterHour` following this one.
    ///
    /// This method wraps, so that the quarter hour following `Q45` is `Q0`.
    ///
    /// ```
    /// # use timelog::v2::interval::QuarterHour;
    /// assert_eq!(QuarterHour::Q0.increment(), QuarterHour::Q15);
    /// assert_eq!(QuarterHour::Q15.increment(), QuarterHour::Q30);
    /// assert_eq!(QuarterHour::Q30.increment(), QuarterHour::Q45);
    /// assert_eq!(QuarterHour::Q45.increment(), QuarterHour::Q0);
    /// ```
    pub fn increment(self) -> Self {
        Self::from_int(self.as_int() + 1)
    }

    /// Get the first minute of this `QuarterHour`.
    /// ```
    /// # use timelog::v2::interval::QuarterHour;
    /// assert_eq!(QuarterHour::Q0.minute(), 0);
    /// assert_eq!(QuarterHour::Q15.minute(), 15);
    /// assert_eq!(QuarterHour::Q30.minute(), 30);
    /// assert_eq!(QuarterHour::Q45.minute(), 45);
    /// ```
    pub fn minute(self) -> u32 {
        self.as_int() * 15
    }

    /// Create a `QuarterHour` from its zero-indexed ordinal position within the hour.
    ///
    /// The argument is taken modulo 4; 0 goes to `Q0`, 1 goes to `Q15`, 2 goes to `Q30`, and 3
    /// goes to `Q45`.
    fn from_int(n: u32) -> Self {
        match n % 4 {
            0 => Self::Q0,
            1 => Self::Q15,
            2 => Self::Q30,
            3 => Self::Q45,
            _ => unreachable!(),
        }
    }

    /// Get the zero-indexed ordinal position of this `QuarterHour` within the hour.
    fn as_int(self) -> u32 {
        match self {
            QuarterHour::Q0 => 0,
            QuarterHour::Q15 => 1,
            QuarterHour::Q30 => 2,
            QuarterHour::Q45 => 3,
        }
    }
}

impl Add for QuarterHour {
    type Output = Self;

    /// Wrapping add
    fn add(self, rhs: Self) -> Self::Output {
        Self::from_int(self.as_int() + rhs.as_int())
    }
}
