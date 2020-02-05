use crate::interval::TaggedInterval;
use crate::tags::TagId;

use chrono::{DateTime, Duration, TimeZone, Utc};

use std::ops::{BitAnd, BitOr, Not};

/*
pub struct Filter<'a> {
    pub filter: Box<dyn Fn(&TaggedInterval) -> bool + 'a>,
}
*/

#[derive(Debug, Clone, PartialOrd, Ord, PartialEq, Eq)]
pub enum Filter {
    // Terminals
    HasTag(TagId),
    IsClosed,
    StartedBefore(DateTime<Utc>),
    EndedBefore(DateTime<Utc>),
    ShorterThan(Duration),

    // Connectives
    Not(Box<Filter>),
    And(Box<Filter>, Box<Filter>),
    Or(Box<Filter>, Box<Filter>),
}

impl Filter {
    pub fn eval(&self, int: &TaggedInterval) -> bool {
        match self {
            Filter::HasTag(tag) => int.tag() == *tag,
            Filter::IsClosed => int.duration().is_some(),
            Filter::StartedBefore(time) => int.start() < *time,
            Filter::EndedBefore(time) => int.end().map(|end| end < *time).unwrap_or(false),
            Filter::ShorterThan(dur) => int.duration().map(|duration| duration < *dur).unwrap_or(false),

            Filter::Not(f) => !f.eval(int),
            Filter::And(f1, f2) => f1.eval(int) && f2.eval(int),
            Filter::Or(f1, f2) => f1.eval(int) || f2.eval(int),
        }
    }

    pub fn closure(self) -> impl Fn(&TaggedInterval) -> bool + 'static {
        move |int| self.eval(int)
    }

    pub fn closure_ref(self) -> impl Fn(&&TaggedInterval) -> bool + 'static {
        move |int| self.eval(int)
    }

    pub fn closure_mut(self) -> impl FnMut(&&mut TaggedInterval) -> bool + 'static {
        move |int| self.eval(int)
    }

    pub fn or(self, other: Filter) -> Filter {
        Filter::Or(Box::new(self), Box::new(other))
    }

    pub fn and(self, other: Filter) -> Filter {
        Filter::And(Box::new(self), Box::new(other))
    }

    pub fn inverted(self) -> Filter {
        Filter::Not(Box::new(self))
    }
}

impl Not for Filter {
    type Output = Self;

    fn not(self) -> Filter {
        self.inverted()
    }
}

impl BitAnd for Filter {
    type Output = Self;

    fn bitand(self, rhs: Filter) -> Filter {
        self.and(rhs)
    }
}

impl BitOr for Filter {
    type Output = Self;

    fn bitor(self, rhs: Filter) -> Filter {
        self.or(rhs)
    }
}

pub fn has_tag(tag: TagId) -> Filter {
    Filter::HasTag(tag)
}

pub fn is_closed() -> Filter {
    Filter::IsClosed
}

pub fn is_open() -> Filter {
    !is_closed()
}

pub fn started_before<Tz>(time: DateTime<Tz>) -> Filter
where
    Tz: TimeZone,
{
    let time = Utc.from_utc_datetime(&time.naive_utc());
    Filter::StartedBefore(time)
}

pub fn ended_before<Tz>(time: DateTime<Tz>) -> Filter
where
    Tz: TimeZone,
{
    let time = Utc.from_utc_datetime(&time.naive_utc());
    Filter::EndedBefore(time)
}

pub fn started_after(time: DateTime<Utc>) -> Filter {
    !started_before(time)
}

pub fn ended_after(time: DateTime<Utc>) -> Filter {
    is_closed() & !ended_before(time)
}

pub fn shorter_than(duration: Duration) -> Filter {
    Filter::ShorterThan(duration)
}

pub fn with_duration_at_least(duration: Duration) -> Filter {
    !shorter_than(duration)
}

pub fn with_duration_at_most(duration: Duration) -> Filter {
    shorter_than(duration + Duration::nanoseconds(1))
}
