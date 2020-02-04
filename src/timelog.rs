use crate::filter::{self, Filter};
use crate::interval::{Interval, TaggedInterval};
use crate::tags::Tags;

use std::error::Error;
use std::fmt::{self, Display, Formatter};

use TimeLogError::*;

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct TimeLog {
    tags: Tags,
    intervals: Vec<TaggedInterval>,
}

impl TimeLog {
    pub fn new() -> TimeLog {
        TimeLog {
            tags: Tags::new(),
            intervals: Vec::new(),
        }
    }

    pub fn filter<'a>(&'a self, filter: Filter<'a>) -> impl Iterator<Item = Interval> + 'a {
        self.intervals
            .iter()
            .copied()
            .filter(filter.filter)
            .map(|int| *int.interval())
    }

    pub fn filter_mut<'a>(
        &'a mut self,
        filter: Filter<'a>,
    ) -> impl Iterator<Item = &mut Interval> + 'a {
        self.intervals
            .iter_mut()
            .filter(move |int| filter.apply(int))
            .map(|int| int.interval_mut())
    }

    pub fn open(&mut self, tag: &str) -> Result<(), TimeLogError> {
        let tag = self.tags.get_id_or_insert(tag);

        if self
            .filter(filter::has_tag(tag) & filter::is_open())
            .next()
            .is_some()
        {
            Err(TagAlreadyOpen)
        } else {
            self.intervals.push(TaggedInterval::open_now(tag));
            Ok(())
        }
    }
}

#[derive(Debug, Clone, Copy, PartialOrd, Ord, PartialEq, Eq, Hash)]
pub enum TimeLogError {
    TagAlreadyOpen,
    TagNotOpen,
}

impl Display for TimeLogError {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            TagAlreadyOpen => write!(f, "attempt to open a tag that is already open"),

            TagNotOpen => write!(f, "attempt to close a tag that is not open"),
        }
    }
}

impl Error for TimeLogError {}
