use bstr::ByteSlice;
use std::ops::{Deref, DerefMut, Range};

use sumtree::{CursorDirection, Item, Node, SumTree};

use crate::{Rope, RopeSlice, Slab};

struct CursorPosition<'a>(SlabCursor<'a>, Position<'a, Slab>);

struct Position<'a, T: sumtree::Item> {
    leaf: &'a SumTree<T>,
    offset: usize,
}

struct SlabCursor<'a>(sumtree::Cursor<'a, Slab>);

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
    fn seek_to_byte(&mut self, offset: usize) -> Option<Position<'a, Slab>> {
        let mut offset = offset;
        let leaf = self.0.seek(|node| {
            let summary = node.summary();
            if offset < summary.len_left {
                CursorDirection::Left
            } else {
                offset -= summary.len_left;
                CursorDirection::Right
            }
        });

        leaf.map(|leaf| Position { leaf, offset })
    }

    fn seek_to_line(&mut self, line: usize) -> Option<Position<'a, Slab>> {
        self.0.reset();
        let mut line = line;
        let leaf = self.0.seek(|node| {
            let summary = node.summary();
            if line <= summary.len_left_lines {
                CursorDirection::Left
            } else {
                line -= summary.len_left_lines;
                CursorDirection::Right
            }
        });
        leaf.and_then(|leaf| match leaf.as_ref() {
            Node::Branch { .. } => unreachable!("sumtree seek must return leaf node"),
            Node::Leaf { item, summary, .. } => {
                if line <= summary.len_lines {
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
    cursor_pos: Option<CursorPosition<'a>>,
    trim_last_terminator: bool,
}

impl<'a> ChunkAndRanges<'a> {
    pub(super) fn new(rope: &'a Rope, range: Range<usize>) -> Self {
        let cursor_pos = rope.0.as_ref().and_then(|tree| {
            let mut cursor = SlabCursor(tree.cursor());
            cursor
                .seek_to_byte(range.start)
                .map(|pos| CursorPosition(cursor, pos))
        });
        Self { range, cursor_pos, trim_last_terminator: false }
    }

    pub(super) fn new_trim_last_terminator(rope: &'a Rope, range: Range<usize>) -> Self {
        let cursor_pos = rope.0.as_ref().and_then(|tree| {
            let mut cursor = SlabCursor(tree.cursor());
            cursor
                .seek_to_byte(range.start)
                .map(|pos| CursorPosition(cursor, pos))
        });
        Self { range, cursor_pos, trim_last_terminator: true }
    }
}

impl<'a> Iterator for ChunkAndRanges<'a> {
    type Item = (&'a [u8], Range<usize>);

    fn next(&mut self) -> Option<Self::Item> {
        if self.range.is_empty() {
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
                        let chunk = Some(&bytes[..(self.range.len())]);
                        if self.trim_last_terminator {
                            trim_last_terminator(chunk)
                        } else {
                            chunk
                        }
                    };

                    if let Some(chunk) = chunk {
                        let chunk_range = self.range.start..(self.range.start + chunk.len());
                        self.cursor_pos = cursor
                            .next()
                            .map(|leaf| Position { leaf, offset: 0 })
                            .map(|p| CursorPosition(cursor, p));

                        self.range = (self.range.start + slab.summary().len - curr_pos.offset)
                            ..self.range.end;

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
    pub(super) fn new(rope: &'a Rope, range: Range<usize>) -> Self {
        Self(ChunkAndRanges::new(rope, range))
    }

    pub(super) fn new_trim_last_terminator(rope: &'a Rope, range: Range<usize>) -> Self {
        Self(ChunkAndRanges::new_trim_last_terminator(rope, range))
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
    curr_chunk: Option<(&'a [u8], Range<usize>)>,
    chars: Option<bstr::CharIndices<'a>>,
}

impl<'a> CharAndRanges<'a> {
    pub(super) fn new(rope: &'a Rope, range: Range<usize>) -> Self {
        let mut chunks = ChunkAndRanges::new(rope, range);
        let curr_chunk = chunks.next();
        let chars = curr_chunk.as_ref().map(|(c, _)| c.char_indices());
        Self { chunks, curr_chunk, chars }
    }
}

impl<'a> Iterator for CharAndRanges<'a> {
    type Item = (char, Range<usize>);

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            match (self.curr_chunk.as_mut(), self.chars.as_mut()) {
                (None, None) => break None,
                (_, None) => self.curr_chunk = self.chunks.next(),
                (_, Some(chars)) => {
                    if let Some((start, end, c)) = chars.next() {
                        break Some((c, start..end));
                    }
                }
            }
        }
    }
}

pub struct Chars<'a>(CharAndRanges<'a>);

impl<'a> Chars<'a> {
    pub(super) fn new(rope: &'a Rope, range: Range<usize>) -> Self {
        Self(CharAndRanges::new(rope, range))
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
    trim_last_terminator: bool,
}

impl<'a> Lines<'a> {
    pub(crate) fn new(rope: &'a Rope, line_range: Range<usize>) -> Self {
        let cursor_pos = rope.0.as_ref().and_then(|tree| {
            let mut cursor = SlabCursor(tree.cursor_with_summary());
            cursor
                .seek_to_line(line_range.start)
                .map(|pos| CursorPosition(cursor, pos))
        });
        Self { rope, cursor_pos, line_range, trim_last_terminator: false }
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

                let start_byte = cursor.summary().len + curr_pos.offset;
                let next_pos = if let Node::Leaf { .. } = curr_pos.leaf.as_ref() {
                    cursor.seek_to_line(self.line_range.start)
                } else {
                    unreachable!("cursor position must be a leaf node")
                };

                let end_byte =
                    cursor.summary().len + next_pos.as_ref().map(|p| p.offset).unwrap_or(0);
                self.cursor_pos = next_pos.map(|pos| CursorPosition(cursor, pos));
                Some(RopeSlice::new(self.rope, start_byte..end_byte))
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
