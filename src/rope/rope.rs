#![allow(clippy::module_inception)]
use bstr::BString;
use std::cmp::Ordering;
use std::ops::RangeBounds;
use std::sync::Arc;

use super::block::Slab;
use super::cursor::Cursor;
use super::error::{Error, Result};
use super::iterator::{ChunkAndRanges, Chunks, Lines};
use super::tree::{self, Node, NodeColour};
use super::util;

use Node::*;
use NodeColour::{Black, Red};

#[derive(Debug, Clone)]
pub struct Rope(pub(super) Arc<Node>);

impl Rope {
    pub fn empty() -> Self {
        let root = Arc::new(Node::empty());
        Self(root)
    }

    pub(crate) fn cursor(&self) -> Cursor {
        Cursor::new(self.clone())
    }

    pub(crate) fn len(&self) -> usize {
        self.0.len()
    }

    pub(super) fn num_lines(&self) -> usize {
        self.0.num_lines() + 1
    }

    pub fn append(&self, text: Slab) -> Result<Self> {
        self.insert(self.len(), text)
    }

    pub fn line_at_offset(&self, offset: usize) -> Result<(usize, usize)> {
        if offset > self.len() {
            return Err(Error::IndexOutOfBounds(offset, self.len()));
        }
        let (line, offset) = line_at_offset(&self.0, offset);
        Ok((line, offset))
    }

    pub fn insert(&self, offset: usize, text: Slab) -> Result<Self> {
        if offset > self.len() {
            return Err(Error::IndexOutOfBounds(offset, self.len()));
        }
        if text.is_empty() {
            return Ok(self.clone());
        }
        let root = Arc::new(insert(&self.0, offset, text));
        let root = make_black(root);
        Ok(Rope(root))
    }

    #[allow(dead_code)]
    pub(super) fn split(&self, offset: usize) -> Result<(Rope, Rope)> {
        if offset > self.len() {
            return Err(Error::IndexOutOfBounds(offset, self.len()));
        }

        let (left, right) = split(&self.0, offset);
        // debug_assert_split_balanced("split", at, &self.0, &left, &right);
        let left = left.map(|(node, _)| Rope(node)).unwrap_or(Rope::empty());
        let right = right.map(|(node, _)| Rope(node)).unwrap_or(Rope::empty());
        Ok((left, right))
    }

    pub fn delete(&self, range: impl RangeBounds<usize>) -> Result<(Rope, Rope)> {
        let bounded_range = util::bound_range(&range, 0..self.len());
        if bounded_range.start > self.len() {
            return Err(Error::RangeOutOfBounds(
                Error::deref_bound(range.start_bound()),
                Error::deref_bound(range.end_bound()),
                self.len(),
            ));
        }
        let range = bounded_range;
        if range.is_empty() {
            let deleted = Self::empty();
            let updated = Self(self.0.clone());
            return Ok((updated, deleted));
        }

        let (left, right) = split(&self.0, range.start);
        // debug_assert_split_balanced("delete_split1", offset, &self.0, &left, &right);

        let s2root = right.unwrap().0;
        let (deleted, right) = split(&s2root, range.len());
        // debug_assert_split_balanced("delete_split2", len, &s2root, &deleted, &right);

        let updated = match join(left, right) {
            None => Self::empty(),
            Some((root, _)) => Self(make_black(root)),
        };
        let deleted = match deleted {
            None => Self::empty(),
            Some((root, _)) => Self(make_black(root)),
        };
        Ok((updated, deleted))
    }

    #[allow(dead_code)]
    pub(super) fn write_dot(&self, w: &mut impl std::io::Write) -> std::io::Result<()> {
        writeln!(w, "digraph {{")?;
        self.0.write_dot(w)?;
        writeln!(w, "}}")?;
        Ok(())
    }

    pub fn is_balanced(&self) -> bool {
        self.0.is_balanced()
    }

    #[allow(dead_code)]
    pub(super) fn to_bstring(&self) -> BString {
        self.0.to_bstring()
    }

    pub(crate) fn chunks(&self, range: impl RangeBounds<usize>) -> Chunks {
        let range = util::bound_range(&range, 0..self.len());
        Chunks::new(self, range)
    }

    #[allow(dead_code)]
    pub(crate) fn chunk_and_ranges(&self, range: impl RangeBounds<usize>) -> ChunkAndRanges {
        let range = util::bound_range(&range, 0..self.len());
        ChunkAndRanges::new(self, range)
    }

    pub(crate) fn lines(&self, range: impl RangeBounds<usize>) -> Lines {
        let range = util::bound_range(&range, 0..self.num_lines());
        Lines::new(self, range)
    }
}

fn make_black(node: Arc<Node>) -> Arc<Node> {
    match node.as_ref() {
        Branch { colour: NodeColour::Red, left, right, metrics } => Arc::new(Branch {
            colour: NodeColour::Black,
            left: left.clone(),
            right: right.clone(),
            metrics: *metrics,
        }),
        _ => node,
    }
}

fn insert(node: &Arc<Node>, offset: usize, text: Slab) -> Node {
    match node.as_ref() {
        Empty => Node::new_leaf(text),
        Leaf { slab: block_ref, .. } => {
            if node.len() == 0 {
                Node::new_leaf(text)
            } else if offset == 0 {
                let left = Arc::new(Node::new_leaf(text));
                Node::new_branch(NodeColour::Red, left, node.clone())
            } else if offset == node.len() {
                let right = Arc::new(Node::new_leaf(text));
                Node::new_branch(NodeColour::Red, node.clone(), right)
            } else {
                let left = Arc::new(Node::new_leaf(block_ref.substr(..offset)));
                let rl = Arc::new(Node::new_leaf(text));
                let rr = Arc::new(Node::new_leaf(block_ref.substr(offset..)));
                let right = Arc::new(Node::new_branch(NodeColour::Red, rl, rr));
                Node::new_branch(NodeColour::Red, left, right)
            }
        }
        Branch { colour, left, right, .. } => {
            let left_len = left.len();
            if left_len > offset {
                let left = insert(left, offset, text);
                let (node, _) = balance(*colour, Arc::new(left), right.clone());
                node
            } else {
                let offset = offset - left_len;
                let right = insert(right, offset, text);
                let (node, _) = balance(*colour, left.clone(), Arc::new(right));
                node
            }
        }
    }
}

fn balance(colour: NodeColour, left: Arc<Node>, right: Arc<Node>) -> (Node, bool) {
    if colour == Red {
        return (Node::new_branch(colour, left, right), false);
    }

    if let Branch { colour: Red, left: ll, right: lr, .. } = left.as_ref() {
        if let Branch { colour: Red, left: a, right: b, .. } = ll.as_ref() {
            let c = lr;
            let d = right;
            let l = Node::new_branch(Black, a.clone(), b.clone());
            let r = Node::new_branch(Black, c.clone(), d.clone());
            return (Node::new_branch(Red, Arc::new(l), Arc::new(r)), true);
        } else if let Branch { colour: Red, left: b, right: c, .. } = lr.as_ref() {
            let a = ll;
            let d = right;
            let l = Node::new_branch(Black, a.clone(), b.clone());
            let r = Node::new_branch(Black, c.clone(), d.clone());
            return (Node::new_branch(Red, Arc::new(l), Arc::new(r)), true);
        }
    };

    if let Branch { colour: Red, left: rl, right: rr, .. } = right.as_ref() {
        if let Branch { colour: Red, left: b, right: c, .. } = rl.as_ref() {
            let a = left;
            let d = rr;
            let l = Node::new_branch(Black, a.clone(), b.clone());
            let r = Node::new_branch(Black, c.clone(), d.clone());
            return (Node::new_branch(Red, Arc::new(l), Arc::new(r)), true);
        } else if let Branch { colour: Red, left: c, right: d, .. } = rr.as_ref() {
            let a = left;
            let b = rl.clone();
            let l = Node::new_branch(Black, a.clone(), b.clone());
            let r = Node::new_branch(Black, c.clone(), d.clone());
            return (Node::new_branch(Red, Arc::new(l), Arc::new(r)), true);
        }
    }

    (Node::new_branch(colour, left, right), false)
}

type NodeWithHeight = (Arc<Node>, usize);
// if (TL.color=black) and (TL.blackHeight=TR.blackHeight):
//         return Node(TL,⟨k,red⟩,TR)
//     T'=Node(TL.left,⟨TL.key,TL.color⟩,joinRightRB(TL.right,k,TR))
//     if (TL.color=black) and (T'.right.color=T'.right.right.color=red):
//         T'.right.right.color=black;
//         return rotateLeft(T')
//     return T' /* T''[recte T'] */
fn join_right(left: NodeWithHeight, right: NodeWithHeight) -> NodeWithHeight {
    let (left, lheight) = left;
    let (right, rheight) = right;
    debug_assert_eq!(lheight, black_height(left.as_ref()));
    debug_assert_eq!(rheight, black_height(right.as_ref()));
    if lheight == rheight {
        if let Branch { colour, .. } = left.as_ref() {
            if *colour == NodeColour::Black {
                let node = Node::new_branch(NodeColour::Red, left.clone(), right.clone());
                return (Arc::new(node), lheight);
            }
        } else {
            let node = Node::new_branch(NodeColour::Red, left.clone(), right.clone());
            return (Arc::new(node), lheight);
        }
    }
    match (left.as_ref(), right.as_ref()) {
        (Branch { colour, left: ll, right: lr, .. }, _) => {
            let lrheight = lheight - (colour.black_height() as usize);
            let (right, jrheight) = join_right((lr.clone(), lrheight), (right, rheight));
            let (node, _) = balance(*colour, ll.clone(), right);
            (Arc::new(node), jrheight + (colour.black_height() as usize))
        }
        _ => unreachable!(),
    }
}

fn join_left(left: NodeWithHeight, right: NodeWithHeight) -> NodeWithHeight {
    let (left, lheight) = left;
    let (right, rheight) = right;
    debug_assert_eq!(lheight, black_height(left.as_ref()));
    debug_assert_eq!(rheight, black_height(right.as_ref()));
    if lheight == rheight {
        if let Branch { colour, .. } = right.as_ref() {
            if *colour == NodeColour::Black {
                let (node, _) = balance(NodeColour::Red, left.clone(), right.clone());
                return (Arc::new(node), lheight);
            }
        } else {
            let (node, _) = balance(NodeColour::Red, left.clone(), right.clone());
            return (Arc::new(node), lheight);
        }
    }
    match (left.as_ref(), right.as_ref()) {
        (_, Branch { colour, left: rl, right: rr, .. }) => {
            let rlheight = rheight - (colour.black_height() as usize);
            let (left, jlheight) = join_left((left, lheight), (rl.clone(), rlheight));
            let (node, _) = balance(*colour, left, rr.clone());
            (Arc::new(node), jlheight + (colour.black_height() as usize))
        }
        _ => unreachable!(),
    }
}

fn join(
    maybe_left: Option<NodeWithHeight>,
    maybe_right: Option<NodeWithHeight>,
) -> Option<NodeWithHeight> {
    let joined = match (maybe_left.clone(), maybe_right.clone()) {
        (None, None) => None,
        (None, Some(right)) => Some(right.clone()),
        (Some(left), None) => Some(left.clone()),
        (Some((left, lheight)), Some((right, rheight))) => {
            debug_assert_eq!(lheight, black_height(left.as_ref()));
            debug_assert_eq!(rheight, black_height(right.as_ref()));
            match rheight.cmp(&lheight) {
                Ordering::Greater => join_left((left, lheight), (right, rheight)).into(),
                Ordering::Less => join_right((left, lheight), (right, rheight)).into(),
                Ordering::Equal => {
                    let colour = if left.colour() == NodeColour::Black
                        && right.colour() == NodeColour::Black
                    {
                        NodeColour::Red
                    } else {
                        NodeColour::Black
                    };

                    let node = Node::new_branch(colour, left.clone(), right.clone());
                    Some((Arc::new(node), lheight + (colour.black_height() as usize)))
                }
            }
        }
    };
    match joined {
        Some((ref node, height)) if node.colour() == NodeColour::Red => {
            Some((make_black(node.clone()), height + 1))
        }
        x => x,
    }
}

fn split(node: &Node, at: usize) -> (Option<NodeWithHeight>, Option<NodeWithHeight>) {
    let (left, right, _) = split_recurse(node, at);
    let left = match left {
        Some((ref node, height)) if node.colour() == NodeColour::Red => {
            Some((make_black(node.clone()), height + 1))
        }
        x => x,
    };
    let right = match right {
        Some((ref node, height)) if node.colour() == NodeColour::Red => {
            Some((make_black(node.clone()), height + 1))
        }
        x => x,
    };

    (left, right)
}

fn split_recurse(
    node: &Node,
    at: usize,
) -> (Option<NodeWithHeight>, Option<NodeWithHeight>, usize) {
    match node {
        Empty => (None, None, 0),
        Leaf { slab, .. } => {
            // TODO: stop making copies if possible
            let split_left = if at == 0 {
                None
            } else {
                Some((Arc::new(Node::new_leaf(slab.substr(..at))), 0))
            };
            let split_right = if at == slab.len() {
                None
            } else {
                Some((Arc::new(Node::new_leaf(slab.substr(at..))), 0))
            };
            (split_left, split_right, 0)
        }
        Branch { colour, left, right, .. } => {
            if at <= left.len() {
                let (split_left, split_right, lheight) = split_recurse(left, at);
                let join_right = Some((right.clone(), lheight));
                let split_right = join(split_right, join_right);
                let height = lheight + (colour.black_height() as usize);
                (split_left, split_right, height)
            } else {
                let (split_left, split_right, rheight) = split_recurse(right, at - left.len());
                let join_left = Some((left.clone(), rheight));
                let split_left = join(join_left, split_left);
                let height = rheight + (colour.black_height() as usize);
                (split_left, split_right, height)
            }
        }
    }
}

fn line_at_offset(node: &Node, offset: usize) -> (usize, usize) {
    let mut parents = vec![];
    let (_, metrics) = tree::leaf_at_byte_offset(&mut parents, node, offset);
    parents.clear();
    let line = metrics.num_lines;
    let (node, metrics) = tree::leaf_at_line_offset(&mut parents, node, line);
    if let Some((_, node_offset)) = node {
        (line, metrics.len + node_offset)
    } else {
        (0, 0)
    }
}

fn black_height(node: &Node) -> usize {
    match &node {
        Empty | Leaf { .. } => 0,
        Branch { colour, ref left, ref right, .. } => {
            let lheight = black_height(left);
            let rheight = black_height(right);
            assert_eq!(lheight, rheight);
            lheight + (colour.black_height() as usize)
        }
    }
}

#[cfg(test)]
mod tests {
    use bstr::ByteSlice;
    use rand::{Rng, SeedableRng};

    use super::*;
    use crate::rope::block::SlabAllocator;
    use crate::rope::macros::*;

    enum Operation {
        Insert { at: usize, buf: Vec<u8> },
        Delete { at: usize, len: usize },
    }

    impl Operation {
        const GEN_BLOCK_SIZE: usize = 8192;

        fn gen_valid_insert(rng: &mut impl Rng, tree: &Rope) -> Self {
            let at = if tree.len() == 0 {
                0
            } else {
                rng.gen_range(0..tree.len())
            };
            let mut buf: Vec<u8> = vec![0; rng.gen_range(0..Self::GEN_BLOCK_SIZE)];
            rng.fill_bytes(&mut buf);
            Operation::Insert { at, buf }
        }

        fn gen_valid_delete(rng: &mut impl Rng, tree: &Rope) -> Self {
            let at = rng.gen_range(0..tree.len());
            let len = rng.gen_range(0..(tree.len() - at));
            Operation::Delete { at, len }
        }

        fn gen_valid(rng: &mut impl Rng, tree: &Rope) -> Self {
            if tree.len() == 0 {
                Self::gen_valid_insert(rng, tree)
            } else {
                match rng.gen_range(0..2) {
                    0 => Self::gen_valid_insert(rng, tree),
                    1 => Self::gen_valid_delete(rng, tree),
                    _ => unreachable!(),
                }
            }
        }
    }

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

        let line_offsets = vec![0, 34, 78, 109, 134, 187];
        for (line_num, (line, expected)) in rope
            .lines(..)
            // .zip(std::iter::once(&0).chain(line_offsets.iter()))
            .zip(line_offsets.iter())
            .enumerate()
        {
            let offset = line.range.start;
            assert_eq!(offset, *expected, "line num={}", line_num)
        }

        let mut line_number = 0;
        let mut line_start = 0;
        let mut line_offsets = line_offsets.iter();
        let mut maybe_next_offset = line_offsets.next();
        for idx in 0..rope.len() {
            if let Some(next_offset) = maybe_next_offset {
                match next_offset.cmp(&idx) {
                    Ordering::Less => unreachable!(),
                    Ordering::Equal => {
                        maybe_next_offset = line_offsets.next();
                        line_number += 1;
                        line_start = *next_offset;
                    }
                    Ordering::Greater => { /*ignore */ }
                }
            }
            let res = rope.line_at_offset(idx).expect("line at offset");
            assert_eq!(res, (line_number - 1, line_start), "offset={}", idx);
        }

        #[rustfmt::skip]
        let parts = vec![
            "This ", "is", " the", " ", "song ", "that", " never ", "ends.\n",
            "It ", "just ", "goes ", "'round ", "and", " 'round", ", my", " ", "fr", "i", "ends.\n",
            "Some ", "people ", "started ", "singing ", "it\n",
            "not ", "knowing ", "what", " it", " was;\n",
            "and ", "they", " ", "continue", " singing", " ", "i", "t", " ", "forever", " j", "us", "t ", "because...", "\n",
        ];
        for (i, actual) in rope.chunks(..).enumerate() {
            let expected = parts.get(i).unwrap_or(&"");
            assert_eq!(actual.as_bstr(), expected, "part={}", i);
        }
        for (i, actual) in rope.chunks(11..).enumerate() {
            let expected = parts.get(i + 3).unwrap_or(&"");
            assert_eq!(actual.as_bstr(), expected, "part={}", i);
        }
        for (i, actual) in rope.chunks(..172).enumerate() {
            let expected = parts.get(i).unwrap_or(&"");
            assert_eq!(actual.as_bstr(), expected, "part={}", i);
        }

        assert_eq!(rope.num_lines(), 6);
        for (i, line) in rope.lines(..).enumerate() {
            let line = line
                .chunks(..)
                .fold(BString::new(Vec::with_capacity(64)), |s, part| {
                    [s, part.as_bstr().into()].concat().into()
                });
            assert_eq!(line, lines[i].as_bstr(), "line={}", i);
        }

        for at in 0..rope.len() {
            let (split_left, split_right) = rope.split(at).expect("split rope");

            // let mut file = std::fs::File::create(format!("target/tests/split_left{:02}.dot", at))
            //     .expect("create file");
            // split_left.write_dot(&mut file).expect("write dot file");
            // let mut file = std::fs::File::create(format!("target/tests/split_right{:02}.dot", at))
            //     .expect("create file");
            // split_right.write_dot(&mut file).expect("write dot file");

            assert_eq!(split_left.to_bstring(), contents[..at].as_bstr());
            assert_eq!(split_right.to_bstring(), contents[at..].as_bstr());

            assert!(split_left.is_balanced(), "unbalanced left; split at {}", at);
            assert!(split_right.is_balanced(), "unbalaced right; split at {}", at);
        }

        // delete from start of rope
        (1..=rope.len()).fold(rope.clone(), |rope, i| {
            let (updated, deleted) = rope.delete(0..1).expect("delete rope");

            // let mut file =
            //     std::fs::File::create(format!("target/tests/delete_updated{:02}.dot", i))
            //         .expect("create file");
            // updated.write_dot(&mut file).expect("write dot file");
            // let mut file =
            //     std::fs::File::create(format!("target/tests/delete_deleted{:02}.dot", i))
            //         .expect("create file");
            // deleted.write_dot(&mut file).expect("write dot file");

            assert_eq!(updated.to_bstring(), contents[i..].as_bstr());
            assert_eq!(deleted.to_bstring(), [contents[i - 1]].as_bstr());
            assert!(updated.is_balanced());
            assert!(deleted.is_balanced());
            updated
        });

        // delete from end of string
        (1..=rope.len()).fold(rope.clone(), |rope, i| {
            let (updated, deleted) = rope.delete(rope.len() - 1..).expect("delete rope");

            // let mut file =
            //     std::fs::File::create(format!("target/tests/delete_updated{:02}.dot", i))
            //         .expect("create file");
            // updated.write_dot(&mut file).expect("write dot file");
            // let mut file =
            //     std::fs::File::create(format!("target/tests/delete_deleted{:02}.dot", i))
            //         .expect("create file");
            // deleted.write_dot(&mut file).expect("write dot file");

            assert_eq!(updated.to_bstring(), contents[..(rope.len() - 1)].as_bstr());
            assert_eq!(deleted.to_bstring(), [contents[rope.len() - 1]].as_bstr());
            assert!(updated.is_balanced(), "unbalanced left node; delete end {}", i);
            assert!(deleted.is_balanced(), "unbalanced right node; delete end {}", i);
            updated
        });

        // delete from middle of string
        (1..=rope.len()).fold(rope.clone(), |rope, i| {
            let middle = rope.len() / 2;
            let (updated, deleted) = rope.delete(middle..middle + 1).expect("delete rope");

            // let mut file =
            //     std::fs::File::create(format!("target/tests/delete_updated{:02}.dot", i))
            //         .expect("create file");
            // updated.write_dot(&mut file).expect("write dot file");
            // let mut file =
            //     std::fs::File::create(format!("target/tests/delete_deleted{:02}.dot", i))
            //         .expect("create file");
            // deleted.write_dot(&mut file).expect("write dot file");

            let updated_str = updated.to_bstring();
            assert_eq!(updated_str[..middle].as_bstr(), contents[..middle].as_bstr());
            assert_eq!(updated_str[middle..].as_bstr(), contents[(middle + i)..].as_bstr());
            // assert_eq!(
            //     deleted.to_string(),
            //     String::from_utf8(vec![contents.as_bytes()[middle]]).expect("utf8 string")
            // );
            assert!(updated.is_balanced(), "unbalanced left node; delete middle {}", i);
            assert!(deleted.is_balanced(), "unbalanced right node; delete middle {}", i);
            updated
        });
    }

    #[test]
    fn regression_1() {
        let mut buf = SlabAllocator::new();

        let tree = Rope(branch_b!(
            branch_b!(
                branch_b!(leaf_e!(buf, 58), leaf_e!(buf, 802)),
                branch_r!(
                    branch_b!(branch_r!(leaf_e!(buf, 896), leaf_e!(buf, 10)), leaf_e!(buf, 53)),
                    branch_b!(leaf_e!(buf, 1048), leaf_e!(buf, 249)),
                ),
            ),
            branch_b!(
                branch_r!(
                    branch_b!(branch_r!(leaf_e!(buf, 927), leaf_e!(buf, 448)), leaf_e!(buf, 365)),
                    branch_b!(leaf_e!(buf, 138), leaf_e!(buf, 269)),
                ),
                branch_b!(leaf_e!(buf, 3), leaf_e!(buf, 7)),
            ),
        ));
        _ = tree.split(151);
    }

    #[test]
    fn random_tests() {
        let _ = std::fs::remove_dir_all("target/assert/");
        std::fs::create_dir_all("target/assert/").expect("create directory");

        let tree = Rope::empty();
        let mut buffer = SlabAllocator::new();
        let mut rng = rand::rngs::SmallRng::from_entropy();

        (0..1_000).fold(tree, |rope, i| {
            let op = Operation::gen_valid(&mut rng, &rope);
            match op {
                Operation::Insert { at, buf } => {
                    let mut rope = rope;
                    let mut buf = &buf[..];
                    let mut at = at;
                    while buf.len() > 0 {
                        let (block, written) = buffer.append(buf).expect("buffer append");
                        rope = rope.insert(at, block).expect("insert rope");
                        buf = &buf[written..];
                        at += written;
                    }
                    assert!(rope.is_balanced(), "unbalanced left node; delete middle {}", i);
                    rope
                }
                Operation::Delete { at, len } => {
                    let (updated, deleted) = rope.delete(at..at + len).expect("delete rope");
                    let deleted_str = deleted.to_bstring();
                    let updated_str = updated.to_bstring();
                    let original_str = rope.to_bstring();
                    assert_eq!(updated_str[..at], BString::from(&original_str[..at]));
                    assert_eq!(updated_str[at..], BString::from(&original_str[(at + len)..]));
                    assert_eq!(deleted_str, BString::from(&original_str[at..(at + len)]));
                    assert!(updated.is_balanced(), "unbalanced left node; delete middle {}", i);
                    assert!(deleted.is_balanced(), "unbalanced right node; delete middle {}", i);
                    updated
                }
            }
        });
    }
}
