use crate::filter::{self, Filter};
use crate::interval::TaggedInterval;
use crate::tags::{Tags, TagId};

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

    pub fn tag_name(&self, tag: TagId) -> Option<&str> {
        self.tags.get_name(tag)
    }

    pub fn iter(&self) -> impl Iterator<Item = &TaggedInterval> {
        self.intervals.iter()
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut TaggedInterval> {
        self.intervals.iter_mut()
    }

    pub fn filter<'a>(&'a self, filter: Filter<'a>) -> impl Iterator<Item = &TaggedInterval> + 'a {
        self.intervals.iter().filter(move |int| filter.apply(int))
    }

    pub fn filter_mut<'a>(
        &'a mut self,
        filter: Filter<'a>,
    ) -> impl Iterator<Item = &mut TaggedInterval> + 'a {
        self.intervals
            .iter_mut()
            .filter(move |int| filter.apply(int))
    }

    pub fn open(&mut self, tag: &str) -> Result<&TaggedInterval, TimeLogError> {
        // lol clippy doesn't know this isn't actually an iterator
        #![allow(clippy::filter_next)]

        let tag = self.tags.get_id_or_insert(tag);

        if self
            .filter(filter::has_tag(tag) & filter::is_open())
            .next()
            .is_some()
        {
            Err(TagAlreadyOpen)
        } else {
            self.intervals.push(TaggedInterval::open_now(tag));
            Ok(self.intervals.last().unwrap())
        }
    }

    pub fn close(&mut self, tag: &str) -> Result<&TaggedInterval, TimeLogError> {
        let tag = self.tags.get_id(tag).ok_or(TagNotOpen)?;

        if let Some(int) = self
            .filter_mut(filter::has_tag(tag) & filter::is_open())
            .next()
        {
            *int = int.close_now().unwrap();
            *int = int.round_to_quarter_hours();
            Ok(&(*int))
        } else {
            Err(TagNotOpen)
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
