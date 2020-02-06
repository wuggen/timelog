use crate::interval::TaggedInterval;
use crate::tags::TagId;

use chrono::{DateTime, Duration, Utc};

use std::ops::{BitAnd, BitOr, Not};

#[derive(Debug, Clone, PartialOrd, Ord, PartialEq, Eq, Hash)]
pub struct Filter {
    // Reverse-Polish-notation representation of the filter expression
    nodes: Vec<FilterNode>,
}

impl Filter {
    pub fn eval(&self, int: &TaggedInterval) -> bool {
        let mut stack = Vec::new();
        self.nodes.iter().for_each(|n| n.eval(&mut stack, int));
        stack.last().copied().unwrap_or(false)
    }

    pub fn eval_const(&self) -> ConstFilter {
        let mut stack = Vec::new();
        self.nodes.iter().for_each(|n| n.eval_const(&mut stack));
        stack.last().copied().unwrap_or(ConstFilter::False)
    }

    pub fn evals_true(&self) -> bool {
        self.eval_const() == ConstFilter::True
    }

    pub fn evals_false(&self) -> bool {
        self.eval_const() == ConstFilter::False
    }

    pub fn evals_nonconst(&self) -> bool {
        self.eval_const() == ConstFilter::NonConst
    }

    pub fn build(&self) -> impl Fn(&TaggedInterval) -> bool + '_ {
        move |int| self.eval(int)
    }

    pub fn build_ref(&self) -> impl Fn(&&TaggedInterval) -> bool + '_ {
        move |int| self.eval(int)
    }

    pub fn build_mut(&self) -> impl FnMut(&&mut TaggedInterval) -> bool + '_ {
        move |int| self.eval(int)
    }

    pub fn or(mut self, other: Filter) -> Filter {
        let self_nodes: &[_] = self.nodes.as_ref();
        let other_nodes: &[_] = other.nodes.as_ref();
        match (self_nodes, other_nodes) {
            ([FilterNode::True], _) => self,
            (_, [FilterNode::True]) => other,
            ([FilterNode::False], _) => other,
            (_, [FilterNode::False]) => self,

            (_, _) => {
                self.nodes.extend_from_slice(&other.nodes);
                self.nodes.push(FilterNode::Or);
                self
            }
        }
    }

    pub fn and(mut self, other: Filter) -> Filter {
        let self_nodes: &[_] = self.nodes.as_ref();
        let other_nodes: &[_] = other.nodes.as_ref();
        match (self_nodes, other_nodes) {
            ([FilterNode::True], _) => other,
            (_, [FilterNode::True]) => self,
            ([FilterNode::False], _) => self,
            (_, [FilterNode::False]) => other,

            (_, _) => {
                self.nodes.extend_from_slice(&other.nodes);
                self.nodes.push(FilterNode::And);
                self
            }
        }
    }

    pub fn inverted(mut self) -> Filter {
        match AsRef::<[_]>::as_ref(&self.nodes) {
            [FilterNode::False] => Filter {
                nodes: vec![FilterNode::True],
            },
            [FilterNode::True] => Filter {
                nodes: vec![FilterNode::False],
            },
            _ => {
                self.nodes.push(FilterNode::Not);
                self
            }
        }
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

pub fn filter_true() -> Filter {
    Filter {
        nodes: vec![FilterNode::True],
    }
}

pub fn filter_false() -> Filter {
    Filter {
        nodes: vec![FilterNode::False],
    }
}

pub fn and_all<I>(filters: I) -> Filter
where
    I: IntoIterator<Item = Filter>,
{
    filters.into_iter().fold(filter_true(), Filter::and)
}

pub fn or_all<I>(filters: I) -> Filter
where
    I: IntoIterator<Item = Filter>,
{
    filters.into_iter().fold(filter_false(), Filter::or)
}

pub fn has_tag(tag: TagId) -> Filter {
    Filter {
        nodes: vec![FilterNode::HasTag(tag)],
    }
}

pub fn is_closed() -> Filter {
    Filter {
        nodes: vec![FilterNode::IsClosed],
    }
}

pub fn is_open() -> Filter {
    !is_closed()
}

pub fn started_before(time: DateTime<Utc>) -> Filter {
    Filter {
        nodes: vec![FilterNode::StartedBefore(time)],
    }
}

pub fn ended_before(time: DateTime<Utc>) -> Filter {
    Filter {
        nodes: vec![FilterNode::EndedBefore(time)],
    }
}

pub fn started_after(time: DateTime<Utc>) -> Filter {
    !started_before(time)
}

pub fn ended_after(time: DateTime<Utc>) -> Filter {
    is_closed() & !ended_before(time)
}

pub fn shorter_than(duration: Duration) -> Filter {
    Filter { nodes: vec![FilterNode::ShorterThan(duration)] }
}

pub fn with_duration_at_least(duration: Duration) -> Filter {
    !shorter_than(duration)
}

pub fn with_duration_at_most(duration: Duration) -> Filter {
    shorter_than(duration + Duration::nanoseconds(1))
}

#[derive(Debug, Clone, PartialOrd, Ord, PartialEq, Eq, Hash)]
enum FilterNode {
    // Terminals
    True,
    False,
    HasTag(TagId),
    IsClosed,
    StartedBefore(DateTime<Utc>),
    EndedBefore(DateTime<Utc>),
    ShorterThan(Duration),

    // Operators
    Not,
    And,
    Or,
}

impl FilterNode {
    fn eval(&self, stack: &mut Vec<bool>, int: &TaggedInterval) {
        match self {
            FilterNode::True => stack.push(true),
            FilterNode::False => stack.push(false),
            FilterNode::HasTag(tag) => stack.push(int.tag() == *tag),
            FilterNode::IsClosed => stack.push(int.end().is_some()),
            FilterNode::StartedBefore(time) => stack.push(int.start() < *time),
            FilterNode::EndedBefore(time) => {
                stack.push(int.end().map(|end| end < *time).unwrap_or(false))
            }
            FilterNode::ShorterThan(dur) => stack.push(int.duration() < *dur),

            FilterNode::Not => {
                let b = stack.pop().unwrap_or(false);
                stack.push(!b);
            }
            FilterNode::And => {
                let (b2, b1) = (stack.pop().unwrap_or(false), stack.pop().unwrap_or(false));
                stack.push(b1 && b2);
            }
            FilterNode::Or => {
                let (b2, b1) = (stack.pop().unwrap_or(false), stack.pop().unwrap_or(false));
                stack.push(b1 || b2);
            }
        }
    }

    fn eval_const(&self, stack: &mut Vec<ConstFilter>) {
        match self {
            FilterNode::True => stack.push(ConstFilter::True),
            FilterNode::False => stack.push(ConstFilter::False),
            FilterNode::Not => {
                let b = stack.pop().unwrap_or(ConstFilter::False);
                stack.push(!b);
            }
            FilterNode::And => {
                let (b2, b1) = (
                    stack.pop().unwrap_or(ConstFilter::False),
                    stack.pop().unwrap_or(ConstFilter::False),
                );
                stack.push(b1 & b2);
            }
            FilterNode::Or => {
                let (b2, b1) = (
                    stack.pop().unwrap_or(ConstFilter::False),
                    stack.pop().unwrap_or(ConstFilter::False),
                );
                stack.push(b1 | b2);
            }

            _ => stack.push(ConstFilter::NonConst),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialOrd, Ord, PartialEq, Eq, Hash)]
pub enum ConstFilter {
    False,
    True,
    NonConst,
}

impl Not for ConstFilter {
    type Output = ConstFilter;
    fn not(self) -> ConstFilter {
        match self {
            ConstFilter::False => ConstFilter::True,
            ConstFilter::True => ConstFilter::False,
            ConstFilter::NonConst => ConstFilter::NonConst,
        }
    }
}

impl BitAnd for ConstFilter {
    type Output = ConstFilter;
    fn bitand(self, other: ConstFilter) -> ConstFilter {
        match (self, other) {
            (ConstFilter::False, _) => ConstFilter::False,
            (_, ConstFilter::False) => ConstFilter::False,
            (ConstFilter::True, rhs) => rhs,
            (lhs, ConstFilter::True) => lhs,
            (_, _) => ConstFilter::NonConst,
        }
    }
}

impl BitOr for ConstFilter {
    type Output = ConstFilter;
    fn bitor(self, other: ConstFilter) -> ConstFilter {
        match (self, other) {
            (ConstFilter::False, rhs) => rhs,
            (lhs, ConstFilter::False) => lhs,
            (ConstFilter::True, _) => ConstFilter::True,
            (_, ConstFilter::True) => ConstFilter::True,
            (_, _) => ConstFilter::NonConst,
        }
    }
}
