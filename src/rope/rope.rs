use bstr::BString;
use std::ops::RangeBounds;
use std::sync::Arc;

use super::block::BlockRange;
use super::cursor::{Chunks, Lines};
use super::error::{Error, Result};
use super::slice::RopeSlice;
use super::tree::{Node, NodeColour};
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

    pub(crate) fn len(&self) -> usize {
        self.0.len()
    }

    pub(super) fn num_lines(&self) -> usize {
        self.0.num_lines() + 1
    }

    pub fn insert(&self, offset: usize, text: BlockRange) -> Result<Self> {
        if offset > self.len() {
            return Err(Error::IndexOutOfBounds(offset, self.len()));
        }
        if text.len() == 0 {
            return Ok(self.clone());
        }
        let root = Arc::new(insert(&self.0, offset, text));
        let root = make_black(root);
        Ok(Rope(root))
    }

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
        if range.len() == 0 {
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

    // pub(super) fn line_range<'a>(&'a self, linenum: usize) -> Option<TreeView<'a>> {
    //     if self.num_lines() < linenum {
    //         return None;
    //     }
    //     let start = if linenum == 1 {
    //         Some(0)
    //     } else {
    //         byte_offset_for_line(&self.0, linenum - 1).map(|i| i + 1)
    //     };
    //     let end = if self.num_lines() == linenum {
    //         None
    //     } else {
    //         byte_offset_for_line(&self.0, linenum).map(|i| i + 1)
    //     };
    //     match (start, end) {
    //         (None, None) => None,
    //         (None, Some(_)) => unreachable!(),
    //         (Some(start), None) => {
    //             if start == self.len() {
    //                 None
    //             } else {
    //                 Some(TreeView::new(self, start..self.len()))
    //             }
    //         }
    //         (Some(start), Some(end)) => Some(TreeView::new(self, start..end)),
    //     }
    // }

    // fn leaf_node_at<'a>(&'a self, at: usize) -> Option<(&'a Node, usize)> {
    //     if at > self.0.len() {
    //         None
    //     } else {
    //         Some(leaf_node_at(self.0.as_ref(), at))
    //     }
    // }

    pub(super) fn write_dot(&self, w: &mut impl std::io::Write) -> std::io::Result<()> {
        write!(w, "digraph {{\n")?;
        self.0.write_dot(w)?;
        write!(w, "}}")?;
        Ok(())
    }

    pub fn is_balanced(&self) -> bool {
        self.0.is_balanced()
    }

    pub(super) fn to_bstring(&self) -> BString {
        self.0.to_bstring()
    }

    fn slice<'a>(&'a self, range: impl RangeBounds<usize>) -> RopeSlice<'a> {
        let range = util::bound_range(&range, 0..self.len());
        RopeSlice::new(self, range)
    }

    pub(crate) fn chunks(&self) -> Chunks {
        Chunks::new(self, 0..self.len())
    }

    pub(crate) fn lines(&self) -> Lines {
        Lines::new(self, 0..self.num_lines())
    }

    // fn lines(&self, range: impl RangeBounds<usize>) -> Lines {
    //     Lines::new(self, range)
    // }
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

fn insert(node: &Arc<Node>, offset: usize, text: BlockRange) -> Node {
    match node.as_ref() {
        Empty => Node::new_leaf(text),
        Leaf { block_ref, .. } => {
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

// if (TL.color=black) and (TL.blackHeight=TR.blackHeight):
//         return Node(TL,⟨k,red⟩,TR)
//     T'=Node(TL.left,⟨TL.key,TL.color⟩,joinRightRB(TL.right,k,TR))
//     if (TL.color=black) and (T'.right.color=T'.right.right.color=red):
//         T'.right.right.color=black;
//         return rotateLeft(T')
//     return T' /* T''[recte T'] */
fn join_right(left: (Arc<Node>, usize), right: (Arc<Node>, usize)) -> (Arc<Node>, usize) {
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

fn join_left(left: (Arc<Node>, usize), right: (Arc<Node>, usize)) -> (Arc<Node>, usize) {
    let (left, lheight) = left;
    let (right, rheight) = right;
    debug_assert_eq!(lheight, black_height(left.as_ref()));
    debug_assert_eq!(rheight, black_height(right.as_ref()));
    if lheight == rheight {
        if let Branch { colour, .. } = right.as_ref() {
            if *colour == NodeColour::Black {
                // let node = Node::new_branch(NodeColour::Red, left.clone(), right.clone());
                let (node, _) = balance(NodeColour::Red, left.clone(), right.clone());
                return (Arc::new(node), lheight);
            }
        } else {
            // let node = Node::new_branch(NodeColour::Red, left.clone(), right.clone());
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
    maybe_left: Option<(Arc<Node>, usize)>,
    maybe_right: Option<(Arc<Node>, usize)>,
) -> Option<(Arc<Node>, usize)> {
    let joined = match (maybe_left.clone(), maybe_right.clone()) {
        (None, None) => None,
        (None, Some(right)) => Some(right.clone()),
        (Some(left), None) => Some(left.clone()),
        (Some((left, lheight)), Some((right, rheight))) => {
            debug_assert_eq!(lheight, black_height(left.as_ref()));
            debug_assert_eq!(rheight, black_height(right.as_ref()));
            // let lheight = black_height(left.as_ref());
            // let rheight = black_height(right.as_ref());
            if rheight > lheight {
                let (node, height) = join_left((left, lheight), (right, rheight));
                Some((node, height))
            } else if lheight > rheight {
                let (node, height) = join_right((left, lheight), (right, rheight));
                Some((node, height))
            } else {
                let colour =
                    if left.colour() == NodeColour::Black && right.colour() == NodeColour::Black {
                        NodeColour::Red
                    } else {
                        NodeColour::Black
                    };

                let node = Node::new_branch(colour, left.clone(), right.clone());
                Some((Arc::new(node), lheight + (colour.black_height() as usize)))
                // let (node, _) = balance(colour, left.clone(), right.clone());
                // Some((Arc::new(node), lheight + (colour.black_height() as usize)))
            }
        }
    };
    let joined = match joined {
        Some((ref node, height)) if node.colour() == NodeColour::Red => {
            Some((make_black(node.clone()), height + 1))
        }
        x => x,
    };
    // debug_assert_join_balanced("join", &maybe_left, &maybe_right, &joined);
    joined
}

fn split(node: &Node, at: usize) -> (Option<(Arc<Node>, usize)>, Option<(Arc<Node>, usize)>) {
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
) -> (Option<(Arc<Node>, usize)>, Option<(Arc<Node>, usize)>, usize) {
    match node {
        Empty => (None, None, 0),
        Leaf { block_ref, .. } => {
            // TODO: stop making copies if possible
            let split_left = if at == 0 {
                None
            } else {
                Some((Arc::new(Node::new_leaf(block_ref.substr(..at))), 0))
            };
            let split_right = if at == block_ref.len() {
                None
            } else {
                Some((Arc::new(Node::new_leaf(block_ref.substr(at..))), 0))
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

// fn debug_assert_split_balanced(
//     prefix: &str,
//     at: usize,
//     pre_split: &Arc<Node>,
//     split_left: &Option<(Arc<Node>, usize)>,
//     split_right: &Option<(Arc<Node>, usize)>,
// ) {
//     if cfg!(debug_assertions) {
//         let left_bal = if let Some((ref node, _)) = split_left {
//             node.is_balanced()
//         } else {
//             true
//         };
//         let right_bal = if let Some((ref node, _)) = split_right {
//             node.is_balanced()
//         } else {
//             true
//         };

//         if !left_bal || !right_bal {
//             std::fs::create_dir_all("target/assert/").expect("create directory");
//             let mut file = std::fs::File::create(format!("target/assert/{}_pre_split.dot", prefix))
//                 .expect("create file");
//             Tree(pre_split.clone())
//                 .write_dot(&mut file)
//                 .expect("write dot file");

//             if let Some((ref node, _)) = split_left {
//                 let mut file =
//                     std::fs::File::create(format!("target/assert/{}_split_left.dot", prefix))
//                         .expect("create file");
//                 Tree(node.clone())
//                     .write_dot(&mut file)
//                     .expect("write dot file");
//             }
//             if let Some((ref node, _)) = split_right {
//                 let mut file =
//                     std::fs::File::create(format!("target/assert/{}_split_right.dot", prefix))
//                         .expect("create file");
//                 Tree(node.clone())
//                     .write_dot(&mut file)
//                     .expect("write dot file");
//             }
//             assert!(left_bal, "left tree unbalanced {} post split at {}", prefix, at);
//             assert!(right_bal, "right tree unbalanced {} post split at {}", prefix, at);
//         }
//     }
// }

// fn debug_assert_join_balanced(
//     prefix: &str,
//     left: &Option<(Arc<Node>, usize)>,
//     right: &Option<(Arc<Node>, usize)>,
//     joined: &Option<(Arc<Node>, usize)>,
// ) {
//     if cfg!(debug_assertions) {
//         let left_bal = if let Some((ref node, _)) = left {
//             node.is_balanced()
//         } else {
//             true
//         };
//         let right_bal = if let Some((ref node, _)) = right {
//             node.is_balanced()
//         } else {
//             true
//         };
//         let joined_bal = if let Some((ref node, _)) = joined {
//             node.is_balanced()
//         } else {
//             true
//         };

//         if !left_bal || !right_bal || !joined_bal {
//             std::fs::create_dir_all("target/assert/").expect("create directory");

//             if let Some((ref node, _)) = left {
//                 let mut file =
//                     std::fs::File::create(format!("target/assert/{}_join_left.dot", prefix))
//                         .expect("create file");
//                 Tree(node.clone())
//                     .write_dot(&mut file)
//                     .expect("write dot file");
//             }

//             if let Some((ref node, _)) = right {
//                 let mut file =
//                     std::fs::File::create(format!("target/assert/{}_join_right.dot", prefix))
//                         .expect("create file");
//                 Tree(node.clone())
//                     .write_dot(&mut file)
//                     .expect("write dot file");
//             }

//             if let Some((ref node, _)) = joined {
//                 let mut file =
//                     std::fs::File::create(format!("target/assert/{}_joined.dot", prefix))
//                         .expect("create file");
//                 Tree(node.clone())
//                     .write_dot(&mut file)
//                     .expect("write dot file");
//             }
//             assert!(left_bal, "left tree unbalanced {} pre join", prefix);
//             assert!(right_bal, "right tree unbalanced {} pre join", prefix);
//             assert!(joined_bal, "joined tree unbalanced {} post join", prefix);
//         }
//     }
// }

#[cfg(test)]
mod tests {
    use bstr::ByteSlice;
    use rand::{Rng, SeedableRng};

    use super::*;
    use crate::rope::block::BlockBuffer;
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

        let mut buffer = BlockBuffer::new();
        for (_i, (at, p)) in parts.iter().enumerate() {
            let (block, w) = buffer.append(p.as_bytes()).unwrap();
            assert_eq!(w, p.len());
            rope = rope.insert(*at, block).unwrap();

            // let mut file = std::fs::File::create(format!("target/tests/insert{:02}.dot", i))
            //     .expect("create file");
            // rope.write_dot(&mut file).expect("write dot file");

            assert!(rope.is_balanced());
        }
        assert!(rope.is_balanced());
        assert_eq!(rope.to_bstring(), contents);

        #[rustfmt::skip]
        let parts = vec![
            "This ", "is", " the", " ", "song ", "that", " never ", "ends.\n",
            "It ", "just ", "goes ", "'round ", "and", " 'round", ", my", " ", "fr", "i", "ends.\n",
            "Some ", "people ", "started ", "singing ", "it\n",
            "not ", "knowing ", "what", " it", " was;\n",
            "and ", "they", " ", "continue", " singing", " ", "i", "t", " ", "forever", " j", "us", "t ", "because...", "\n",
        ];
        for (i, actual) in rope.chunks().enumerate() {
            let expected = parts.get(i).unwrap_or(&"");
            assert_eq!(actual.as_bstr(), expected, "part={}", i);
        }
        for (i, actual) in rope.slice(11..).chunks().enumerate() {
            let expected = parts.get(i + 3).unwrap_or(&"");
            assert_eq!(actual.as_bstr(), expected, "part={}", i);
        }
        for (i, actual) in rope.slice(..172).chunks().enumerate() {
            let expected = parts.get(i).unwrap_or(&"");
            assert_eq!(actual.as_bstr(), expected, "part={}", i);
        }

        assert_eq!(rope.num_lines(), 6);
        for (i, line) in rope.lines().enumerate() {
            let line = line
                .chunks()
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
        let mut buf = BlockBuffer::new();

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
        let mut buffer = BlockBuffer::new();
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
