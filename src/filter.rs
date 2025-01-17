//! Boolean precidates for filtering tagged intervals.

use crate::interval::TaggedInterval;
use crate::tags::TagId;

use chrono::{DateTime, Duration, Utc};

use std::ops::{BitAnd, BitOr, Not};

use std::fmt::{self, Debug, Formatter};

/// A filter for tagged intervals.
///
/// This is a boolean expression evaluated on tagged intervals. If it evaluates to true, the
/// interval passes the filter.
#[derive(Clone, PartialOrd, Ord, PartialEq, Eq, Hash)]
pub struct Filter {
    // Reverse-Polish-notation representation of the filter expression
    nodes: Vec<FilterNode>,
}

impl Filter {
    /// Evaluate this filter on the given interval.
    pub fn eval(&self, int: &TaggedInterval) -> bool {
        let mut stack = Vec::new();
        self.nodes.iter().for_each(|n| n.eval(&mut stack, int));
        stack.last().copied().unwrap_or(false)
    }

    /// Evaluate this filter as a constant boolean value.
    ///
    /// If this filter evaluates to a constant value regardless of the interval upon which it is
    /// evaluated, this will return that value. If the filter depends upon the particular interval,
    /// this will return `ConstFilter::NonConst`.
    pub fn eval_const(&self) -> ConstFilter {
        let mut stack = Vec::new();
        self.nodes.iter().for_each(|n| n.eval_const(&mut stack));
        stack.last().copied().unwrap_or(ConstFilter::False)
    }

    /// Does this filter always evaluate to true?
    pub fn evals_true(&self) -> bool {
        self.eval_const() == ConstFilter::True
    }

    /// Does this filter always evaluate to false?
    pub fn evals_false(&self) -> bool {
        self.eval_const() == ConstFilter::False
    }

    /// Does this filter's value depend on the interval it is evaluated upon?
    pub fn evals_nonconst(&self) -> bool {
        self.eval_const() == ConstFilter::NonConst
    }

    /// Create a closure that evaluates this filter on a tagged interval.
    pub fn build(&self) -> impl Fn(&TaggedInterval) -> bool + '_ {
        move |int| self.eval(int)
    }

    /// Create a closure that evaluates this filter on a tagged interval.
    pub fn build_ref(&self) -> impl Fn(&&TaggedInterval) -> bool + '_ {
        move |int| self.eval(int)
    }

    /// Create a closure that evaluates this filter on a tagged interval.
    pub fn build_mut(&self) -> impl FnMut(&&mut TaggedInterval) -> bool + '_ {
        move |int| self.eval(int)
    }

    /// Create a filter that evaluates to true if either this or the given filter evaluate to true.
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

    /// Create a filter that evaluates to true if both this and the given filter evaluate to true.
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

    /// Create a filter that evaluates to true if this filter evaluates to false.
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

/// A filter that always evaluates to true.
pub fn filter_true() -> Filter {
    Filter {
        nodes: vec![FilterNode::True],
    }
}

/// A filter that always evaluates to false.
pub fn filter_false() -> Filter {
    Filter {
        nodes: vec![FilterNode::False],
    }
}

/// Create a filter that evaluates to true if all of the given filters evaluate to true.
pub fn and_all<I>(filters: I) -> Filter
where
    I: IntoIterator<Item = Filter>,
{
    filters.into_iter().fold(filter_true(), Filter::and)
}

/// Create a filter that evaluates to true if any of the given filters evaluate to true.
pub fn or_all<I>(filters: I) -> Filter
where
    I: IntoIterator<Item = Filter>,
{
    filters.into_iter().fold(filter_false(), Filter::or)
}

/// A filter that passes if the interval has the given tag.
pub fn has_tag(tag: TagId) -> Filter {
    Filter {
        nodes: vec![FilterNode::HasTag(tag)],
    }
}

/// A filter that passes if the interval is closed.
pub fn is_closed() -> Filter {
    Filter {
        nodes: vec![FilterNode::IsClosed],
    }
}

/// A filter that passes if the interval is open.
pub fn is_open() -> Filter {
    !is_closed()
}

/// A filter that passes if the interval's start time is no later than the given time.
pub fn started_before(time: DateTime<Utc>) -> Filter {
    Filter {
        nodes: vec![FilterNode::StartedBefore(time)],
    }
}

/// A filter that passes if the interval's end time is no later than the given time.
pub fn ended_before(time: DateTime<Utc>) -> Filter {
    Filter {
        nodes: vec![FilterNode::EndedBefore(time)],
    }
}

/// A filter that passes if the interval's duration is at most the given duration.
pub fn shorter_than(duration: Duration) -> Filter {
    Filter {
        nodes: vec![FilterNode::ShorterThan(duration)],
    }
}

/// A filter that passes if the interval's start time is _strictly_ before the given time.
pub fn started_before_strict(time: DateTime<Utc>) -> Filter {
    Filter {
        nodes: vec![FilterNode::StartedBeforeStrict(time)],
    }
}

/// A filter that passes if the interval's end time is _strictly_ before the given time.
pub fn ended_before_strict(time: DateTime<Utc>) -> Filter {
    Filter {
        nodes: vec![FilterNode::EndedBeforeStrict(time)],
    }
}

/// A filter that passes if the interval's duration is strictly shorter than the given duration.
pub fn shorter_than_strict(duration: Duration) -> Filter {
    Filter {
        nodes: vec![FilterNode::ShorterThanStrict(duration)],
    }
}

/// A filter that passes if the interval started no earlier than the given time.
pub fn started_after(time: DateTime<Utc>) -> Filter {
    !started_before_strict(time)
}

/// A filter that passes if the interval's end time is no earlier than the given time.
pub fn ended_after(time: DateTime<Utc>) -> Filter {
    is_closed() & !ended_before_strict(time)
}

/// A filter that passes if the interval's start time is strictly after the given time.
pub fn started_after_strict(time: DateTime<Utc>) -> Filter {
    !started_before(time)
}

/// A filter that passes if the interval's end time is strictly after the given time.
pub fn ended_after_strict(time: DateTime<Utc>) -> Filter {
    is_closed() & !ended_before(time)
}

/// A filter that passes if the interval's duration is at least the given duration.
pub fn longer_than(duration: Duration) -> Filter {
    !shorter_than_strict(duration)
}

/// A filter that passes if the interval's duration is strictly longer than the given duration.
pub fn longer_than_strict(duration: Duration) -> Filter {
    !shorter_than(duration)
}

impl Debug for Filter {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "Filter {{ nodes: ")?;
        write_as_tree(&self.nodes[..], self.nodes.len(), f)?;
        write!(f, " }}")
    }
}

fn write_as_tree(nodes: &[FilterNode], idx: usize, f: &mut Formatter) -> Result<usize, fmt::Error> {
    if let Some(node) = nodes.get(idx - 1) {
        match node {
            FilterNode::True => {
                write!(f, "True")?;
                Ok(idx - 1)
            }
            FilterNode::False => {
                write!(f, "False")?;
                Ok(idx - 1)
            }
            FilterNode::HasTag(tag) => {
                write!(f, "HasTag({})", tag)?;
                Ok(idx - 1)
            }
            FilterNode::IsClosed => {
                write!(f, "IsClosed")?;
                Ok(idx - 1)
            }
            FilterNode::StartedBefore(time) => {
                write!(f, "StartedBefore({:?})", time)?;
                Ok(idx - 1)
            }
            FilterNode::EndedBefore(time) => {
                write!(f, "EndedBefore({:?})", time)?;
                Ok(idx - 1)
            }
            FilterNode::ShorterThan(dur) => {
                write!(f, "ShorterThan({:?})", dur)?;
                Ok(idx - 1)
            }
            FilterNode::StartedBeforeStrict(time) => {
                write!(f, "StartedBeforeStrict({:?})", time)?;
                Ok(idx - 1)
            }
            FilterNode::EndedBeforeStrict(time) => {
                write!(f, "EndedBeforeStrict({:?})", time)?;
                Ok(idx - 1)
            }
            FilterNode::ShorterThanStrict(dur) => {
                write!(f, "ShorterThanStrict({:?})", dur)?;
                Ok(idx - 1)
            }

            FilterNode::Not => {
                write!(f, "Not(")?;
                let new_idx = write_as_tree(nodes, idx - 1, f)?;
                write!(f, ")")?;
                Ok(new_idx)
            }

            FilterNode::And => {
                write!(f, "And(")?;
                let new_idx = write_as_tree(nodes, idx - 1, f)?;
                write!(f, ", ")?;
                let new_idx = write_as_tree(nodes, new_idx, f)?;
                write!(f, ")")?;
                Ok(new_idx)
            }

            FilterNode::Or => {
                write!(f, "Or(")?;
                let new_idx = write_as_tree(nodes, idx - 1, f)?;
                write!(f, ", ")?;
                let new_idx = write_as_tree(nodes, new_idx, f)?;
                write!(f, ")")?;
                Ok(new_idx)
            }
        }
    } else {
        Ok(0)
    }
}

/// Filters are implemented internally as an RPN representation, using these operators, values, and
/// predicates.
#[derive(Debug, Clone, PartialOrd, Ord, PartialEq, Eq, Hash)]
enum FilterNode {
    // Terminals
    /// Constant true
    True,
    /// Constant false
    False,
    /// True if the interval has the given tag
    HasTag(TagId),
    /// True if the interval is closed
    IsClosed,
    /// True if the interval started before this time (non-strict)
    StartedBefore(DateTime<Utc>),
    /// True if the interval ended before this time (non-strict; false if the interval is open)
    EndedBefore(DateTime<Utc>),
    /// True if the interval is shorter than this duration (non-strict)
    ShorterThan(Duration),
    /// True if the interval started before this time (strict)
    StartedBeforeStrict(DateTime<Utc>),
    /// True if the interval ended before this time (strict; false if the interval is open)
    EndedBeforeStrict(DateTime<Utc>),
    /// True if the interval is shorter than this duration (strict)
    ShorterThanStrict(Duration),

    // Operators
    /// Invert top of stack
    Not,
    /// AND top two stack values
    And,
    /// OR top two stack values
    Or,
}

impl FilterNode {
    /// Evaluate this filter node on a given value stack and interval.
    fn eval(&self, stack: &mut Vec<bool>, int: &TaggedInterval) {
        match self {
            FilterNode::True => stack.push(true),
            FilterNode::False => stack.push(false),
            FilterNode::HasTag(tag) => stack.push(int.tag() == *tag),
            FilterNode::IsClosed => stack.push(int.end().is_some()),
            FilterNode::StartedBefore(time) => stack.push(int.start() <= *time),
            FilterNode::EndedBefore(time) => {
                stack.push(int.end().map(|end| end <= *time).unwrap_or(false))
            }
            FilterNode::ShorterThan(dur) => stack.push(int.duration() <= *dur),
            FilterNode::StartedBeforeStrict(time) => stack.push(int.start() < *time),
            FilterNode::EndedBeforeStrict(time) => {
                stack.push(int.end().map(|end| end < *time).unwrap_or(false))
            }
            FilterNode::ShorterThanStrict(dur) => stack.push(int.duration() < *dur),

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

    /// Attempt to evaluate this filter node without reference to an interval.
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

/// Possible results from evaluating a filter without reference to an interval.
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
