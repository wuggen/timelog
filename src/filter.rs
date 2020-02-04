use crate::interval::TaggedInterval;
use crate::tags::TagId;

use chrono::{DateTime, Duration, Utc};

use std::ops::{BitAnd, BitOr, Not};

pub struct Filter<'a> {
    pub filter: Box<dyn Fn(&TaggedInterval) -> bool + 'a>,
}

impl<'a> Filter<'a> {
    pub fn new<F>(filter: F) -> Filter<'a>
    where
        F: Fn(&TaggedInterval) -> bool + 'a,
    {
        Filter {
            filter: Box::new(filter),
        }
    }

    pub fn apply(&self, int: &TaggedInterval) -> bool {
        (*self.filter)(int)
    }

    pub fn or(self, other: Filter<'a>) -> Filter<'a> {
        Filter::new(move |int| self.apply(int) || other.apply(int))
    }

    pub fn and(self, other: Filter<'a>) -> Filter<'a> {
        Filter::new(move |int| self.apply(int) && other.apply(int))
    }
}

impl<'a> Not for Filter<'a> {
    type Output = Self;

    fn not(self) -> Filter<'a> {
        Filter::new(move |int| !self.apply(int))
    }
}

impl<'a> BitAnd for Filter<'a> {
    type Output = Self;

    fn bitand(self, rhs: Filter<'a>) -> Filter<'a> {
        self.and(rhs)
    }
}

impl<'a> BitOr for Filter<'a> {
    type Output = Self;

    fn bitor(self, rhs: Filter<'a>) -> Filter<'a> {
        self.or(rhs)
    }
}

pub fn has_tag<'a>(tag: TagId) -> Filter<'a> {
    Filter::new(move |int| int.tag() == tag)
}

pub fn is_closed<'a>() -> Filter<'a> {
    Filter::new(|int| int.is_closed())
}

pub fn is_open<'a>() -> Filter<'a> {
    !is_closed()
}

pub fn started_before<'a>(time: DateTime<Utc>) -> Filter<'a> {
    Filter::new(move |int| int.start() < time)
}

pub fn ended_before<'a>(time: DateTime<Utc>) -> Filter<'a> {
    Filter::new(move |int| {
        if let Some(end) = int.end() {
            end < time
        } else {
            false
        }
    })
}

pub fn started_after<'a>(time: DateTime<Utc>) -> Filter<'a> {
    !started_before(time)
}

pub fn ended_after<'a>(time: DateTime<Utc>) -> Filter<'a> {
    !ended_before(time)
}

pub fn with_duration_at_least<'a>(duration: Duration) -> Filter<'a> {
    Filter::new(move |int| {
        if let Some(dur) = int.duration() {
            duration <= dur
        } else {
            false
        }
    })
}

pub fn with_duration_at_most<'a>(duration: Duration) -> Filter<'a> {
    Filter::new(move |int| {
        if let Some(dur) = int.duration() {
            dur <= duration
        } else {
            false
        }
    })
}
