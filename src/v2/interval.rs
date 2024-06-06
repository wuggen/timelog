use std::fmt::{self, Debug, Display, Formatter};

use chrono::{NaiveTime, Timelike};

/// A timezone-naive time that restricts the minutes to quarter-hour increments.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RestrictedTime(u32, QuarterHour);

impl Debug for RestrictedTime {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        Debug::fmt(&NaiveTime::from(self), f)
    }
}

impl Display for RestrictedTime {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        Display::fmt(&NaiveTime::from(self), f)
    }
}

impl RestrictedTime {
    /// Create a new `RestrictedTime` with the given hours and quarter hour.
    pub fn new(hour: u32, quarter_hour: QuarterHour) -> Self {
        Self(hour, quarter_hour)
    }

    /// Get the quarter-hour floor of the given time.
    ///
    /// This will return a `RestrictedTime` representing the quarter-hour increment most recently
    /// preceding the given time. If the given time is exactly a quarter-hour increment, the
    /// returned `RestrictedTime` will represent the same time.
    pub fn floor<T: Timelike>(time: T) -> Self {
        let hour = time.hour();
        let quarter_hour = QuarterHour::of(&time);
        Self(hour, quarter_hour)
    }

    /// Get the quarter-hour ceiling of the given time.
    ///
    /// This will return a `RestrictedTime` representing the quarter-hour increment most closely
    /// following the given time. If the given time is exactly a quarter-hour increment, the
    /// returned `RestrictedTime` will represent the same time.
    pub fn ceil<T: Timelike>(time: T) -> Self {
        todo!()
    }

    /// Get the hour of this `RestrictedTime`.
    pub fn hour(&self) -> u32 {
        self.0
    }

    /// Get the quarter hour of this `RestrictedTime`.
    pub fn quarter_hour(&self) -> QuarterHour {
        self.1
    }

    /// Get the minute of this `RestrictedTime`.
    ///
    /// This will be 0, 15, 30, or 45.
    pub fn minute(&self) -> u32 {
        self.1.minute()
    }
}

impl<T: Timelike> From<T> for RestrictedTime {
    fn from(value: T) -> Self {
        let hour = value.hour();
        let quarter_hour = QuarterHour::of(&value);
        Self(hour, quarter_hour)
    }
}

impl From<&RestrictedTime> for NaiveTime {
    fn from(value: &RestrictedTime) -> Self {
        let hour = value.hour();
        let minute = value.minute();
        NaiveTime::from_hms(hour, minute, 0)
    }
}

impl From<RestrictedTime> for NaiveTime {
    fn from(value: RestrictedTime) -> Self {
        Self::from(&value)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum QuarterHour {
    Q0,
    Q15,
    Q30,
    Q45,
}

impl QuarterHour {
    pub fn of<T>(time: &T) -> Self
    where
        T: Timelike,
    {
        let minute = time.minute();
        if minute < 15 {
            Self::Q0
        } else if minute < 30 {
            Self::Q15
        } else if minute < 45 {
            Self::Q30
        } else {
            Self::Q45
        }
    }

    pub fn increment(self) -> Self {
        match self {
            Self::Q0 => Self::Q15,
            Self::Q15 => Self::Q30,
            Self::Q30 => Self::Q45,
            Self::Q45 => Self::Q0,
        }
    }

    pub fn minute(self) -> u32 {
        match self {
            QuarterHour::Q0 => 0,
            QuarterHour::Q15 => 15,
            QuarterHour::Q30 => 30,
            QuarterHour::Q45 => 45,
        }
    }
}
