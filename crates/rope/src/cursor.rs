use bstr::ByteSlice;
use std::ops::{Deref, DerefMut, Range};

use sumtree::{CursorDirection, Item, Node, SumTree};

use crate::{Rope, RopeSlice, Slab, Stats};

pub(crate) struct CursorPosition<'a>(pub SlabCursor<'a>, pub Position<'a, Slab>);

pub(crate) struct Position<'a, T: sumtree::Item> {
    pub(crate) leaf: &'a SumTree<T>,
    pub(crate) offset: usize,
}

pub(crate) struct SlabCursor<'a>(pub sumtree::Cursor<'a, Slab>);

impl<'a> Deref for SlabCursor<'a> {
    type Target = sumtree::Cursor<'a, Slab>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<'a> DerefMut for SlabCursor<'a> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl<'a> SlabCursor<'a> {
    pub(crate) fn seek_to_byte(&mut self, offset: usize) -> Option<Position<'a, Slab>> {
        let mut offset = offset;
        let leaf = self.0.seek(|node| {
            let summary = node.summary();
            let left = summary.left.unwrap_or(Stats::default());
            if offset < left.len {
                CursorDirection::Left
            } else {
                offset -= left.len;
                CursorDirection::Right
            }
        });

        leaf.map(|leaf| Position { leaf, offset })
    }

    pub(crate) fn seek_to_line(&mut self, line: usize) -> Option<Position<'a, Slab>> {
        self.0.reset();
        let mut line = line;
        let leaf = self.0.seek(|node| {
            let summary = node.summary();
            let left = summary.left.unwrap_or(Stats::default());
            if line <= left.lines.line {
                CursorDirection::Left
            } else {
                line -= left.lines.line;
                CursorDirection::Right
            }
        });
        leaf.and_then(|leaf| match leaf.as_ref() {
            Node::Branch { .. } => unreachable!("sumtree seek must return leaf node"),
            Node::Leaf { item, summary, .. } => {
                if line <= summary.stats.lines.line {
                    let bytes = item.as_bytes();
                    let offset = if line == 0 {
                        Some(0) // memchr::memchr(b'\n', bytes)
                    } else if line == 1 {
                        memchr::memchr(b'\n', bytes).map(|i| i + 1)
                    } else {
                        memchr::memchr_iter(b'\n', bytes)
                            .enumerate()
                            .find(|(i, _)| *i == line - 1)
                            .map(|(_, p)| p + 1)
                    }
                    .unwrap();
                    if offset == bytes.len() {
                        self.0.next().map(|leaf| Position { leaf, offset: 0 })
                    } else {
                        Some(Position { leaf, offset })
                    }
                } else {
                    unreachable!("leaf must contain {} lines", line)
                }
            }
        })
    }
}

pub struct ChunkAndRanges<'a> {
    range: Range<usize>,
    offset: usize,
    cursor_pos: Option<CursorPosition<'a>>,
    trim_last_terminator: bool,
}

impl<'a> ChunkAndRanges<'a> {
    pub(super) fn new(rope: &'a Rope, range: Range<usize>, offset: usize) -> Self {
        let cursor_pos = rope.0.as_ref().and_then(|tree| {
            let mut cursor = SlabCursor(tree.cursor());
            cursor
                .seek_to_byte(range.start + offset)
                .map(|pos| CursorPosition(cursor, pos))
        });
        Self { range, offset, cursor_pos, trim_last_terminator: false }
    }

    pub(super) fn new_trim_last_terminator(
        rope: &'a Rope,
        range: Range<usize>,
        offset: usize,
    ) -> Self {
        let cursor_pos = rope.0.as_ref().and_then(|tree| {
            let mut cursor = SlabCursor(tree.cursor());
            cursor
                .seek_to_byte(range.start + offset)
                .map(|pos| CursorPosition(cursor, pos))
        });
        Self { range, offset, cursor_pos, trim_last_terminator: true }
    }
}

impl<'a> Iterator for ChunkAndRanges<'a> {
    type Item = (&'a [u8], Range<usize>);

    fn next(&mut self) -> Option<Self::Item> {
        if self.offset >= self.range.len() {
            // if self.range.is_empty() {
            return None;
        }

        match self.cursor_pos.take() {
            None => None,
            Some(CursorPosition(mut cursor, curr_pos)) => {
                if let Node::Leaf { item: slab, .. } = curr_pos.leaf.as_ref() {
                    let bytes = &slab.as_bytes()[curr_pos.offset..];
                    let chunk = if bytes.len() < self.range.len() {
                        Some(bytes)
                    } else {
                        Some(&bytes[..(self.range.len())])
                    };

                    let chunk = if self.trim_last_terminator {
                        trim_last_terminator(chunk)
                    } else {
                        chunk
                    };

                    if let Some(chunk) = chunk {
                        let chunk_range = (self.range.start + self.offset)
                            ..(self.range.start + self.offset + chunk.len());
                        self.cursor_pos = cursor
                            .0
                            .next()
                            .map(|leaf| Position { leaf, offset: 0 })
                            .map(|p| CursorPosition(cursor, p));
                        self.offset += slab.summary().stats.len - curr_pos.offset;

                        Some((chunk, chunk_range))
                    } else {
                        None
                    }
                } else {
                    unreachable!()
                }
            }
        }
    }
}

pub struct Chunks<'a>(ChunkAndRanges<'a>);

impl<'a> Chunks<'a> {
    pub(super) fn new(rope: &'a Rope, range: Range<usize>, offset: usize) -> Self {
        Self(ChunkAndRanges::new(rope, range, offset))
    }

    pub(super) fn new_trim_last_terminator(
        rope: &'a Rope,
        range: Range<usize>,
        offset: usize,
    ) -> Self {
        Self(ChunkAndRanges::new_trim_last_terminator(rope, range, offset))
    }
}

impl<'a> Iterator for Chunks<'a> {
    type Item = &'a [u8];

    fn next(&mut self) -> Option<Self::Item> {
        self.0.next().map(|(chunk, _)| chunk)
    }
}

pub struct CharAndRanges<'a> {
    chunks: ChunkAndRanges<'a>,
    curr: Option<((&'a [u8], Range<usize>), bstr::CharIndices<'a>, usize)>,
}

impl<'a> CharAndRanges<'a> {
    pub(super) fn new(rope: &'a Rope, range: Range<usize>, offset: usize) -> Self {
        let mut chunks = ChunkAndRanges::new(rope, range, offset);
        let curr = Self::chunks_next(&mut chunks);
        Self { chunks, curr }
    }

    fn chunks_next<'b>(
        chunks: &mut ChunkAndRanges<'b>,
    ) -> Option<((&'b [u8], Range<usize>), bstr::CharIndices<'b>, usize)> {
        chunks
            .next()
            .map(|(chunk, range)| ((chunk, range), chunk.char_indices(), 0))
    }

    pub fn offset(&self) -> usize {
        self.curr
            .as_ref()
            .map(|((_, range), _, offset)| range.start + offset)
            .unwrap_or(self.chunks.offset)
    }
}

impl<'a> Iterator for CharAndRanges<'a> {
    type Item = (char, Range<usize>);

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            match self.curr.as_mut() {
                None => break None,
                Some(((_, r), chars, ref mut offset)) => match chars.next() {
                    None => self.curr = CharAndRanges::chunks_next(&mut self.chunks),
                    Some((start, end, c)) => {
                        let range = (r.start + start)..(r.start + end);
                        *offset += range.len();
                        break Some((c, range));
                    }
                },
            }
        }
    }
}

pub struct Chars<'a>(CharAndRanges<'a>);

impl<'a> Chars<'a> {
    pub(super) fn new(rope: &'a Rope, range: Range<usize>, offset: usize) -> Self {
        Self(CharAndRanges::new(rope, range, offset))
    }

    pub fn offset(&self) -> usize {
        self.0.offset()
    }
}

impl<'a> Iterator for Chars<'a> {
    type Item = char;

    fn next(&mut self) -> Option<Self::Item> {
        self.0.next().map(|(chunk, _)| chunk)
    }
}

pub struct Lines<'a> {
    rope: &'a Rope,
    cursor_pos: Option<CursorPosition<'a>>,
    line_range: Range<usize>,
}

impl<'a> Lines<'a> {
    pub(crate) fn new(rope: &'a Rope, line_range: Range<usize>) -> Self {
        let cursor_pos = rope.0.as_ref().and_then(|tree| {
            let mut cursor = SlabCursor(tree.cursor_with_summary());
            cursor
                .seek_to_line(line_range.start)
                .map(|pos| CursorPosition(cursor, pos))
        });
        Self { rope, cursor_pos, line_range }
    }
}

impl<'a> Iterator for Lines<'a> {
    type Item = RopeSlice<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.line_range.is_empty() {
            return None;
        }
        match self.cursor_pos.take() {
            None => None,
            Some(CursorPosition(mut cursor, curr_pos)) => {
                self.line_range = (self.line_range.start + 1)..self.line_range.end;

                let start_byte = cursor.summary().stats.len + curr_pos.offset;
                let next_pos = if let Node::Leaf { .. } = curr_pos.leaf.as_ref() {
                    cursor.seek_to_line(self.line_range.start)
                } else {
                    unreachable!("cursor position must be a leaf node")
                };

                let end_byte =
                    cursor.summary().stats.len + next_pos.as_ref().map(|p| p.offset).unwrap_or(0);
                self.cursor_pos = next_pos.map(|pos| CursorPosition(cursor, pos));
                Some(RopeSlice::new_trim_last_terminator(self.rope, start_byte..end_byte))
            }
        }
    }
}

fn trim_last_terminator(s: Option<&[u8]>) -> Option<&[u8]> {
    match s {
        None => None,
        Some(mut s) => {
            if s.last_byte() == Some(b'\n') {
                s = &s[..s.len() - 1];
                if s.last_byte() == Some(b'\r') {
                    s = &s[..s.len() - 1];
                }
            }
            if s.is_empty() {
                None
            } else {
                Some(s)
            }
        }
    }
}
