use std::ops::Range;

use super::cursor::Chunks;

use super::Rope;

// #[derive(Debug, Clone)]
// pub enum RopeRange {
//     Bytes(Range<usize>),
//     Lines(Range<usize>),
// }

// impl RopeRange {
//     pub(super) fn is_empty(&self) -> bool {
//         use RopeRange::*;
//         match self {
//             Bytes(range) | Lines(range) => range.is_empty(),
//         }
//     }

//     pub(super) fn advance_lines(&self, num_lines: usize) -> RopeRange {
//         use RopeRange::*;
//         match self {
//             Bytes(_) => unreachable!(),
//             Lines(range) => Lines((range.start + num_lines)..range.end),
//         }
//     }

//     pub(super) fn advance_by(&self, metrics: &NodeMetrics) -> RopeRange {
//         use RopeRange::*;
//         match self {
//             Bytes(range) => Bytes((range.start + metrics.len)..range.end),
//             Lines(range) => Lines((range.start + metrics.num_lines)..range.end),
//         }
//     }
// }

pub(crate) struct RopeSlice<'a> {
    rope: &'a Rope,
    range: Range<usize>,
}

// impl<'a> RopeSlice<'a> {
//     pub(super) fn new(rope: &'a Rope, range: Range<usize>) -> Self {
//         Self { rope, range }
//     }

//     pub(crate) fn chunks(&self) -> Chunks<'a> {
//         Chunks::new(self.rope, self.range.clone())
//     }
// }

pub(crate) struct RopeLine<'a>(RopeSlice<'a>);

impl<'a> RopeLine<'a> {
    pub(super) fn new(rope: &'a Rope, range: Range<usize>) -> Self {
        RopeLine(RopeSlice { rope, range })
    }

    pub(crate) fn chunks(&self) -> Chunks<'a> {
        Chunks::new_trim_last_terminator(self.0.rope, self.0.range.clone())
    }
}
