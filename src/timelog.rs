use crate::filter::{self, Filter};
use crate::interval::{self, Interval, TaggedInterval};
use crate::tags::{TagId, Tags};

use chrono::Utc;

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

    pub fn tag_id(&self, tag: &str) -> Option<TagId> {
        self.tags.get_id(tag)
    }

    pub fn iter(&self) -> impl Iterator<Item = &TaggedInterval> {
        self.intervals.iter()
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut TaggedInterval> {
        self.intervals.iter_mut()
    }

    pub fn remove(&mut self, filter: &Filter) {
        self.retain(&(!filter.clone()));
    }

    pub fn retain(&mut self, filter: &Filter) {
        self.intervals = self.iter().cloned().filter(filter.closure()).collect();
    }

    pub fn gc_tag_names(&mut self) {
        let mut new_log = TimeLog::new();
        for int in self.intervals.iter() {
            let tag = self.tags.get_name(int.tag()).unwrap();

            new_log.insert_unchecked(tag, *int.interval());
        }

        self.tags = new_log.tags;
        self.intervals = new_log.intervals;
    }

    fn insert_unchecked(&mut self, tag: &str, int: Interval) -> TaggedInterval {
        let tag = self.tags.get_id_or_insert(tag);
        let int = TaggedInterval::new(tag, int);
        self.intervals.push(int);
        *self.intervals.last().unwrap()
    }

    pub fn open(&mut self, tag: &str) -> Result<TaggedInterval, TimeLogError> {
        let tag = self.tags.get_id_or_insert(tag);
        let now_floor = interval::floor_time(&Utc::now());
        let filter = filter::has_tag(tag) & (filter::is_open() | filter::ended_after(now_floor));

        if let Some(int) = self.iter_mut().find(filter.closure_mut()) {
            return if !int.is_closed() {
                Err(TagAlreadyOpen)
            } else {
                *int = TaggedInterval::open(int.tag(), int.start());
                Ok(*int)
            };
        }

        let new_int = TaggedInterval::open(tag, now_floor);
        self.intervals.push(new_int);
        Ok(*self.intervals.last().unwrap())
    }

    pub fn close(&mut self, tag: &str) -> Result<TaggedInterval, TimeLogError> {
        let tag = self.tags.get_id(tag).ok_or(TagNotOpen)?;
        let filter = filter::has_tag(tag) & filter::is_open();

        if let Some(int) = self.iter_mut().filter(filter.closure_mut()).next() {
            *int = int.close_now().unwrap();
            *int = int.round_to_quarter_hours();
            Ok(*int)
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
