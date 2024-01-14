use std::ops::{Deref, Range, RangeBounds};

use super::iterator::{ChunkAndRanges, Chunks};

use super::{util, Rope};

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
    pub(crate) range: Range<usize>,
}

// impl<'a> RopeSlice<'a> {
//     pub(super) fn new(rope: &'a Rope, range: Range<usize>) -> Self {
//         Self { rope, range }
//     }

//     pub(crate) fn chunks(&self, range: impl RangeBounds<usize>) -> Chunks<'a> {
//         let range = util::bound_range(&range, self.range.clone());
//         Chunks::new(self.rope, range)
//     }
// }

pub(crate) struct RopeLine<'a>(RopeSlice<'a>);

impl<'a> Deref for RopeLine<'a> {
    type Target = RopeSlice<'a>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<'a> RopeLine<'a> {
    pub(super) fn new(rope: &'a Rope, range: Range<usize>) -> Self {
        RopeLine(RopeSlice { rope, range })
    }

    #[allow(dead_code)]
    pub(crate) fn chunks(&self, range: impl RangeBounds<usize>) -> Chunks {
        let range = util::bound_range(&range, self.0.range.clone());
        Chunks::new_trim_last_terminator(self.0.rope, range)
    }

    pub fn chunk_and_ranges(&self, range: impl RangeBounds<usize>) -> ChunkAndRanges {
        let range = util::bound_range(&range, self.0.range.clone());
        ChunkAndRanges::new_trim_last_terminator(self.0.rope, range)
    }
}
