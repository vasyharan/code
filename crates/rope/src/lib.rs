use std::ops::{Range, RangeBounds};
use sumtree::{Colour, Node, SumTree};
use tore::Point;

#[cfg(test)]
use bstr::{BString, ByteVec};

mod cursor;
mod error;
mod slab;
mod util;

use crate::cursor::SlabCursor;
use crate::error::{Error, Result};
use crate::slab::Slab;

pub use crate::cursor::{CharRange, Chars, ChunkAndRanges, Chunks, Lines};
pub use crate::slab::SlabAllocator;

#[derive(Debug, Clone)]
pub struct Rope(pub(crate) Option<SumTree<Slab>>);

impl Rope {
    pub fn new(tree: SumTree<Slab>) -> Self {
        Self(Some(tree))
    }

    pub fn empty() -> Self {
        Self(None)
    }

    pub fn point_to_offset(&self, p: Point) -> Option<usize> {
        match self.line(p.line) {
            None => None,
            Some(line) => {
                if p.column < line.range.len() {
                    Some(line.range.start + p.column)
                } else {
                    None
                }
            }
        }
    }

    pub fn offset_to_point(&self, offset: usize) -> Option<Point> {
        self.0.as_ref().and_then(|tree| {
            let mut cursor = SlabCursor(tree.cursor_with_summary());
            let pos = cursor.seek_to_byte(offset);
            pos.map(|pos| match pos.leaf.as_ref() {
                Node::Branch { .. } => unreachable!("sumtree seek must return leaf node"),
                Node::Leaf { item, .. } => {
                    let summary = cursor.summary();
                    let (line_offset, column_offset) =
                        item.as_bytes()[..pos.offset]
                            .iter()
                            .fold((0, 0), |(ls, cs), b| {
                                if *b == b'\n' {
                                    (ls + 1, 0)
                                } else {
                                    (ls, cs + 1)
                                }
                            });
                    let line = summary.stats.lines.line + line_offset;
                    let column = summary.stats.lines.column + column_offset;
                    Point { line, column }
                }
            })
        })
    }

    pub fn chunks(&self, range: impl RangeBounds<usize>, offset: usize) -> Chunks {
        let range = util::bound_range(&range, 0..self.len());
        Chunks::new(self, range, offset)
    }

    pub fn char_range(&self, range: impl RangeBounds<usize>, offset: usize) -> CharRange {
        let range = util::bound_range(&range, 0..self.len());
        CharRange::new(self, range, offset)
    }

    pub fn chars(&self, range: impl RangeBounds<usize>, offset: usize) -> Chars {
        let range = util::bound_range(&range, 0..self.len());
        Chars::new(self, range, offset)
    }

    pub fn lines(&self, lines: impl RangeBounds<usize>) -> Lines {
        let lines = util::bound_range(&lines, 0..self.len_lines());
        Lines::new(self, lines)
    }

    pub fn line(&self, line: usize) -> Option<RopeSlice<'_>> {
        let range = util::bound_range(&(line..line + 1), 0..self.len_lines());
        Lines::new(self, range).next()
    }

    pub fn slice(&self, range: impl RangeBounds<usize>) -> RopeSlice<'_> {
        let range = util::bound_range(&range, 0..self.len());
        RopeSlice { rope: self, range, trim_last_terminator: false }
    }

    pub fn char_at(&self, point: Point) -> Option<char> {
        // use bstr::ByteSlice;

        // self.line(point.line).and_then(|line| {
        //     let mut column = point.column;
        //     for chunk in line.chunks(0) {
        //         for c in chunk.chars() {
        //             if column == 0 {
        //                 return Some(c);
        //             }
        //             column -= 1; // TODO: width compute
        //         }
        //     }
        //     None
        // })
        self.point_to_offset(point).and_then(|offset| {
            /* abd */
            self.chars(.., offset).next()
        })
    }

    pub fn insert(&self, offset: usize, text: Slab) -> Result<Self> {
        if offset > self.len() {
            return Err(Error::IndexOutOfBounds(offset, self.len()));
        }
        if text.is_empty() {
            return Ok(self.clone());
        }
        match &self.0 {
            None => Ok(Self(Some(SumTree::new_leaf(text)))),
            Some(tree) => {
                let mut offset = offset;
                let mut cursor = tree.cursor();
                let leaf = cursor
                    .seek(|node| {
                        let summary = node.summary();
                        let left = summary.left.unwrap_or_default();
                        if offset < left.len {
                            sumtree::cursor::Direction::Left
                        } else if offset >= left.len {
                            offset -= left.len;
                            sumtree::cursor::Direction::Right
                        } else {
                            unreachable!()
                        }
                    })
                    .unwrap();
                let pos = cursor.into_position();
                let summary = leaf.summary();
                let slab = leaf.deref_item();
                let tree = if offset == 0 {
                    pos.insert_left(text)
                } else if offset == summary.stats.len {
                    pos.insert_right(text)
                } else {
                    let left = SumTree::new_leaf(slab.substr(..offset));
                    let rl = SumTree::new_leaf(text);
                    let rr = SumTree::new_leaf(slab.substr(offset..));
                    let right = SumTree::new_branch(Colour::Red, rl, rr);
                    pos.replace(left, right)
                };
                Ok(Self(Some(tree)))
            }
        }
    }

    pub fn append(&self, text: Slab) -> Result<Self> {
        self.insert(self.len(), text)
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn len(&self) -> usize {
        match &self.0 {
            None => 0,
            Some(tree) => tree.summary().stats.len,
        }
    }

    pub fn len_lines(&self) -> usize {
        match &self.0 {
            None => 0,
            Some(tree) => tree.summary().stats.lines.line,
        }
    }

    #[cfg(test)]
    pub(crate) fn to_bstring(&self) -> BString {
        match &self.0 {
            None => b"".into(),
            Some(tree) => match tree.as_ref() {
                Node::Leaf { item, .. } => item.as_bytes().into(),
                Node::Branch { left, right, .. } => {
                    let mut bstr = Rope::new(left.clone()).to_bstring();
                    bstr.push_str(Rope::new(right.clone()).to_bstring());
                    bstr
                }
            },
        }
    }

    #[cfg(test)]
    pub(crate) fn is_balanced(&self) -> bool {
        match self.0 {
            None => true,
            Some(ref tree) => tree.is_balanced(),
        }
    }

    #[cfg(test)]
    pub(crate) fn write_dot(&self, w: &mut impl std::io::Write) -> std::io::Result<()> {
        match self.0 {
            None => Ok(()),
            Some(ref tree) => tree.write_dot(w),
        }
    }
}

pub struct RopeSlice<'a> {
    rope: &'a Rope,
    range: Range<usize>,
    trim_last_terminator: bool,
}

impl<'a> RopeSlice<'a> {
    pub fn new(rope: &'a Rope, range: Range<usize>) -> Self {
        Self { rope, range, trim_last_terminator: false }
    }

    pub fn new_trim_last_terminator(rope: &'a Rope, range: Range<usize>) -> Self {
        Self { rope, range, trim_last_terminator: true }
    }

    pub fn is_empty(&self) -> bool {
        self.range.is_empty()
    }

    pub fn len(&self) -> usize {
        self.range.len()
    }

    pub fn chunk_and_ranges(&self, offset: usize) -> ChunkAndRanges {
        if self.trim_last_terminator {
            ChunkAndRanges::new_trim_last_terminator(self.rope, self.range.clone(), offset)
        } else {
            ChunkAndRanges::new(self.rope, self.range.clone(), offset)
        }
    }

    pub fn chunks(&self, offset: usize) -> Chunks {
        if self.trim_last_terminator {
            Chunks::new_trim_last_terminator(self.rope, self.range.clone(), offset)
        } else {
            Chunks::new(self.rope, self.range.clone(), offset)
        }
    }
}
#[derive(Default, Clone, Copy)]
pub struct Stats {
    pub len: usize,
    pub lines: Point,
    pub len_first_line: usize,
    pub len_last_line: usize,
}

#[derive(Default, Clone, Copy)]
pub struct Metrics {
    pub stats: Stats,
    pub left: Option<Stats>,
}

impl std::fmt::Debug for Metrics {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let stats = self.stats;
        write!(
            f,
            "({},{}/{},{}/{})",
            stats.len,
            stats.lines.line,
            stats.lines.column,
            stats.len_first_line,
            stats.len_last_line,
        )
    }
}

impl sumtree::Item for Slab {
    type Summary = Metrics;

    fn summary(&self) -> Self::Summary {
        let bs = self.as_bytes();
        let len = bs.len();
        let (len_lines, len_last_line) =
            bs.iter()
                .fold((vec![], 0_usize), |(counts, last_count), ch| {
                    if *ch == b'\n' {
                        ([counts, [last_count].to_vec()].concat(), 0)
                    } else {
                        (counts, last_count + 1)
                    }
                });
        let len_first_line = *len_lines.first().unwrap_or(&0);
        let lines = Point { line: len_lines.len(), column: len_last_line };
        let stats = Stats { len, lines, len_first_line, len_last_line };
        Metrics { stats, left: None }
    }
}

impl sumtree::Summary for Metrics {
    fn combine(&self, rhs: &Self) -> Self {
        let len = self.stats.len + rhs.stats.len;
        let (len_first_line, lines, len_last_line) = if rhs.stats.lines.line == 0 {
            let line = self.stats.lines.line;
            let column = self.stats.lines.column + rhs.stats.lines.column;
            let lines = Point { line, column };
            let len_first_line = self.stats.len_first_line;
            let len_last_line = self.stats.len_last_line + rhs.stats.len_last_line;
            (len_first_line, lines, len_last_line)
        } else {
            let line = self.stats.lines.line + rhs.stats.lines.line;
            let column = rhs.stats.lines.column;
            let lines = Point { line, column };
            let len_first_line = self.stats.len_last_line + rhs.stats.len_first_line;
            let len_last_line = rhs.stats.len_last_line;
            (len_first_line, lines, len_last_line)
        };
        let stats = Stats { len, lines, len_first_line, len_last_line };
        Metrics { stats, left: Some(self.stats) }
    }

    fn scan_branch(&mut self, from: &Self) {
        let left = from.left.expect("branch must have left stats");
        self.stats.len += left.len;
        if left.lines.line == 0 {
            self.stats.lines.column += left.lines.column;
            self.stats.len_first_line += left.len_first_line;
            self.stats.len_last_line += left.len_last_line;
        } else {
            self.stats.lines.line += left.lines.line;
            self.stats.lines.column = left.len_last_line;
            self.stats.len_first_line += left.len_first_line;
        };
    }

    fn scan_leaf(&mut self, from: &Self) {
        let left = from.stats;
        self.stats.len += left.len;
        if left.lines.line == 0 {
            self.stats.lines.column += left.lines.column;
            self.stats.len_first_line += left.len_first_line;
            self.stats.len_last_line += left.len_last_line;
        } else {
            self.stats.lines.line += left.lines.line;
            self.stats.lines.column = left.len_last_line;
            self.stats.len_first_line += left.len_first_line;
        };
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use bstr::ByteSlice;
    use circular_buffer::CircularBuffer;

    #[test]
    fn basic_tests() {
        let _ = std::fs::remove_dir_all("target/tests/");
        std::fs::create_dir_all("target/tests/").expect("create directory");
        let parts = vec![
            (0, "Some "),
            (5, "people "),
            (0, "It "),
            (15, "not "),
            (3, "just "),
            (24, "knowing "),
            (8, "goes and"),
            (28, "started "),
            (13, "'round "),
            (23, " 'round "),
            (51, "singing "),
            (71, "what was;\n"),
            (75, " it"),
            (30, ", my"),
            (63, "it\n"),
            (35, "frends.\n"),
            (37, "i"),
            (100, " forever"),
            (0, "This "),
            (113, "because..."),
            (5, " the"),
            (5, "is"),
            (111, "and "),
            (115, "they"),
            (11, "ends.\n"),
            (11, " never "),
            (133, "continue "),
            (11, " that"),
            (146, " singing"),
            (12, "song "),
            (159, " t"),
            (160, "i"),
            (170, " jt "),
            (172, "us"),
            (186, "\n"),
        ];
        let contents: BString = "This is the song that never ends.\n\
                 It just goes 'round and 'round, my friends.\n\
                 Some people started singing it\n\
                 not knowing what it was;\n\
                 and they continue singing it forever just because...\n\
             "
        .into();
        let mut lines: Vec<_> = contents.lines().collect();
        lines.push("".as_bytes());

        let mut rope = Rope::empty();
        assert!(rope.is_balanced());

        let mut buffer = SlabAllocator::new();
        for (i, (at, p)) in parts.iter().enumerate() {
            let (block, w) = buffer.append(p.as_bytes()).unwrap();
            assert_eq!(w, p.len());
            rope = rope.insert(*at, block).unwrap();

            let mut file = std::fs::File::create(format!("target/tests/insert{:02}.dot", i))
                .expect("create file");
            rope.write_dot(&mut file).expect("write dot file");

            assert!(rope.is_balanced());
        }
        assert!(rope.is_balanced());
        assert_eq!(rope.to_bstring(), contents);

        let mut chars = rope.char_range(.., 0);
        let mut lookback = CircularBuffer::<2, _>::new();
        for (start, end, c) in contents.char_indices() {
            assert_eq!(chars.next(), Some((c, start..end)));
            assert_eq!(chars.offset(), end);
            lookback.push_front((c, start..end));

            assert_eq!(chars.prev().as_ref(), lookback.get(0));
            if start > 1 {
                assert_eq!(chars.prev().as_ref(), lookback.get(1));
                _ = chars.next();
            }

            assert_eq!(chars.next(), Some((c, start..end)));
        }
        assert_eq!(chars.next(), None);
        assert_eq!(chars.prev().as_ref(), lookback.get(0));
        assert_eq!(chars.prev().as_ref(), lookback.get(1));

        let line_offsets = [0, 34, 78, 109, 134, 187];
        for (line_num, (line, expected)) in rope.lines(..).zip(line_offsets.iter()).enumerate() {
            let offset = line.range.start;
            assert_eq!(offset, *expected, "line num={}", line_num)
        }

        for (linenum, (line, offset)) in lines.iter().zip(line_offsets.iter()).enumerate() {
            for (start, end, _) in line.char_indices() {
                let point = Point { line: linenum, column: start };
                let actual = rope.point_to_offset(point);
                assert_eq!(actual, Some(offset + start), "{:?}", point);

                let actual = rope.offset_to_point(offset + start);
                assert_eq!(actual, Some(point), "{:?}", point);

                for column in (start + 1)..end {
                    let actual = rope.point_to_offset(Point { line: linenum, column });
                    assert_eq!(actual, None);
                }
            }
        }

        // let mut line_number = 0;
        // let mut line_start = 0;
        // let mut line_offsets = line_offsets.iter();
        // let mut maybe_next_offset = line_offsets.next();
        // for idx in 0..rope.len() {
        //     if let Some(next_offset) = maybe_next_offset {
        //         match next_offset.cmp(&idx) {
        //             Ordering::Less => unreachable!(),
        //             Ordering::Equal => {
        //                 maybe_next_offset = line_offsets.next();
        //                 line_number += 1;
        //                 line_start = *next_offset;
        //             }
        //             Ordering::Greater => { /*ignore */ }
        //         }
        //     }
        //     let res = rope.line_at_offset(idx).expect("line at offset");
        //     assert_eq!(res, (line_number - 1, line_start), "offset={}", idx);
        // }

        #[rustfmt::skip]
        let parts = vec![
            "This ", "is", " the", " ", "song ", "that", " never ", "ends.\n",
            "It ", "just ", "goes ", "'round ", "and", " 'round", ", my", " ", "fr", "i", "ends.\n",
            "Some ", "people ", "started ", "singing ", "it\n",
            "not ", "knowing ", "what", " it", " was;\n",
            "and ", "they", " ", "continue", " singing", " ", "i", "t", " ", "forever", " j", "us", "t ", "because...", "\n",
        ];
        for (i, actual) in rope.chunks(.., 0).enumerate() {
            let expected = parts.get(i).unwrap_or(&"");
            assert_eq!(actual.as_bstr(), expected, "part={}", i);
        }
        for (i, actual) in rope.chunks(11.., 0).enumerate() {
            let expected = parts.get(i + 3).unwrap_or(&"");
            assert_eq!(actual.as_bstr(), expected, "part={}", i);
        }
        for (i, actual) in rope.chunks(..172, 0).enumerate() {
            let expected = parts.get(i).unwrap_or(&"");
            assert_eq!(actual.as_bstr(), expected, "part={}", i);
        }

        assert_eq!(rope.len_lines(), 5);
        for (i, line) in rope.lines(..).enumerate() {
            let line = line
                .chunks(0)
                .fold(BString::new(Vec::with_capacity(64)), |s, part| {
                    [s, part.as_bstr().into()].concat().into()
                });
            assert_eq!(line, lines[i].as_bstr(), "line={}", i);
        }

        let mut char_indicies = rope.char_range(.., 0);
        for (start, end, c) in contents.char_indices() {
            assert_eq!(char_indicies.next(), Some((c, start..end)));
        }
        assert_eq!(char_indicies.next(), None);
    }
}

// #[cfg(test)]
// mod tests {
//     use bstr::{BString, ByteSlice};

//     use super::slab::SlabAllocator;
//     use super::Rope;

//     #[test]
//     fn basic_tests() {
//         for at in 0..rope.len() {
//             let (split_left, split_right) = rope.split(at).expect("split rope");

//             // let mut file = std::fs::File::create(format!("target/tests/split_left{:02}.dot", at))
//             //     .expect("create file");
//             // split_left.write_dot(&mut file).expect("write dot file");
//             // let mut file = std::fs::File::create(format!("target/tests/split_right{:02}.dot", at))
//             //     .expect("create file");
//             // split_right.write_dot(&mut file).expect("write dot file");

//             assert_eq!(split_left.to_bstring(), contents[..at].as_bstr());
//             assert_eq!(split_right.to_bstring(), contents[at..].as_bstr());

//             assert!(split_left.is_balanced(), "unbalanced left; split at {}", at);
//             assert!(split_right.is_balanced(), "unbalaced right; split at {}", at);
//         }

//         // delete from start of rope
//         (1..=rope.len()).fold(rope.clone(), |rope, i| {
//             let (updated, deleted) = rope.delete(0..1).expect("delete rope");

//             // let mut file =
//             //     std::fs::File::create(format!("target/tests/delete_updated{:02}.dot", i))
//             //         .expect("create file");
//             // updated.write_dot(&mut file).expect("write dot file");
//             // let mut file =
//             //     std::fs::File::create(format!("target/tests/delete_deleted{:02}.dot", i))
//             //         .expect("create file");
//             // deleted.write_dot(&mut file).expect("write dot file");

//             assert_eq!(updated.to_bstring(), contents[i..].as_bstr());
//             assert_eq!(deleted.to_bstring(), [contents[i - 1]].as_bstr());
//             assert!(updated.is_balanced());
//             assert!(deleted.is_balanced());
//             updated
//         });

//         // delete from end of string
//         (1..=rope.len()).fold(rope.clone(), |rope, i| {
//             let (updated, deleted) = rope.delete(rope.len() - 1..).expect("delete rope");

//             // let mut file =
//             //     std::fs::File::create(format!("target/tests/delete_updated{:02}.dot", i))
//             //         .expect("create file");
//             // updated.write_dot(&mut file).expect("write dot file");
//             // let mut file =
//             //     std::fs::File::create(format!("target/tests/delete_deleted{:02}.dot", i))
//             //         .expect("create file");
//             // deleted.write_dot(&mut file).expect("write dot file");

//             assert_eq!(updated.to_bstring(), contents[..(rope.len() - 1)].as_bstr());
//             assert_eq!(deleted.to_bstring(), [contents[rope.len() - 1]].as_bstr());
//             assert!(updated.is_balanced(), "unbalanced left node; delete end {}", i);
//             assert!(deleted.is_balanced(), "unbalanced right node; delete end {}", i);
//             updated
//         });

//         // delete from middle of string
//         (1..=rope.len()).fold(rope.clone(), |rope, i| {
//             let middle = rope.len() / 2;
//             let (updated, deleted) = rope.delete(middle..middle + 1).expect("delete rope");

//             // let mut file =
//             //     std::fs::File::create(format!("target/tests/delete_updated{:02}.dot", i))
//             //         .expect("create file");
//             // updated.write_dot(&mut file).expect("write dot file");
//             // let mut file =
//             //     std::fs::File::create(format!("target/tests/delete_deleted{:02}.dot", i))
//             //         .expect("create file");
//             // deleted.write_dot(&mut file).expect("write dot file");

//             let updated_str = updated.to_bstring();
//             assert_eq!(updated_str[..middle].as_bstr(), contents[..middle].as_bstr());
//             assert_eq!(updated_str[middle..].as_bstr(), contents[(middle + i)..].as_bstr());
//             // assert_eq!(
//             //     deleted.to_string(),
//             //     String::from_utf8(vec![contents.as_bytes()[middle]]).expect("utf8 string")
//             // );
//             assert!(updated.is_balanced(), "unbalanced left node; delete middle {}", i);
//             assert!(deleted.is_balanced(), "unbalanced right node; delete middle {}", i);
//             updated
//         });
//     }
// }
