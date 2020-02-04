use crate::tags::TagId;

use chrono::{DateTime, Duration, Timelike, Utc};

use std::ops::Add;
use std::time::Duration as StdDuration;

#[derive(Debug, Clone, Copy, PartialOrd, Ord, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Interval {
    start: DateTime<Utc>,
    duration: Option<StdDuration>,
}

impl Interval {
    pub fn open(start: DateTime<Utc>) -> Interval {
        Interval {
            start,
            duration: None,
        }
    }

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

    pub fn open_now() -> Interval {
        Interval::open(Utc::now())
    }

    pub fn close_now(&self) -> Option<Interval> {
        self.close(Utc::now())
    }

    pub fn closed(start: DateTime<Utc>, duration: StdDuration) -> Interval {
        Interval {
            start,
            duration: Some(duration),
        }
    }

    pub fn start_now() -> Interval {
        let start = Utc::now();
        let duration = None;

        Interval { start, duration }
    }

    pub fn round_to_quarter_hours(self) -> Interval {
        let Interval { start, duration } = self;

        let end_time =
            duration.map(|d| QuarterHour::ceil(&(start + Duration::from_std(d).unwrap())));
        let start = QuarterHour::floor(&start);
        let duration = end_time.map(|t| (t - start).to_std().unwrap());

        Interval { start, duration }
    }

    pub fn is_closed(&self) -> bool {
        self.duration.is_some()
    }

    pub fn start(&self) -> DateTime<Utc> {
        self.start
    }

    pub fn end(&self) -> Option<DateTime<Utc>> {
        self.duration
            .map(|d| self.start + Duration::from_std(d).unwrap())
    }

    pub fn duration(&self) -> Option<Duration> {
        self.duration.map(|d| Duration::from_std(d).unwrap())
    }
}

#[derive(Debug, Clone, Copy, PartialOrd, Ord, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TaggedInterval {
    tag: TagId,
    interval: Interval,
}

impl TaggedInterval {
    pub fn new(tag: TagId, interval: Interval) -> TaggedInterval {
        TaggedInterval { tag, interval }
    }

    pub fn tag(&self) -> TagId {
        self.tag
    }

    pub fn interval(&self) -> &Interval {
        &self.interval
    }

    pub fn interval_mut(&mut self) -> &mut Interval {
        &mut self.interval
    }

    pub fn open(tag: TagId, start: DateTime<Utc>) -> TaggedInterval {
        let interval = Interval::open(start);
        TaggedInterval { tag, interval }
    }

    pub fn close(&self, end: DateTime<Utc>) -> Option<TaggedInterval> {
        let interval = self.interval.close(end)?;
        Some(TaggedInterval { interval, ..*self })
    }

    pub fn open_now(tag: TagId) -> TaggedInterval {
        let interval = Interval::open_now();
        TaggedInterval { tag, interval }
    }

    pub fn close_now(&self) -> Option<TaggedInterval> {
        let interval = self.interval.close_now()?;
        Some(TaggedInterval { interval, ..*self })
    }

    pub fn is_closed(&self) -> bool {
        self.interval.is_closed()
    }

    pub fn start(&self) -> DateTime<Utc> {
        self.interval.start()
    }

    pub fn end(&self) -> Option<DateTime<Utc>> {
        self.interval.end()
    }

    pub fn duration(&self) -> Option<Duration> {
        self.interval.duration()
    }
}

pub fn tag(tag: TagId, interval: Interval) -> TaggedInterval {
    TaggedInterval::new(tag, interval)
}

#[derive(Debug, Clone, Copy, PartialOrd, Ord, PartialEq, Eq, Hash)]
enum QuarterHour {
    TopOfHour,
    QuarterPast,
    HalfPast,
    QuarterTill,
}

use QuarterHour::*;

impl QuarterHour {
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

    fn minute(self) -> u32 {
        match self {
            TopOfHour => 0,
            QuarterPast => 15,
            HalfPast => 30,
            QuarterTill => 45,
        }
    }

    fn floor<T>(time: &T) -> T
    where
        T: Timelike,
    {
        let qh = QuarterHour::of(time);
        time.with_minute(qh.minute())
            .unwrap()
            .with_second(0)
            .unwrap()
    }

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
    }
}
