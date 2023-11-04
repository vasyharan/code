use bstr::{BString, ByteVec};
use std::{
    ops::{Add, AddAssign, Sub, SubAssign},
    sync::Arc,
};

use crate::rope::block::BlockRange;

use Node::{Branch, Empty, Leaf};

#[derive(Debug)]
pub(super) enum Error {
    ConsecutiveRed,
    DifferingBlackHeight,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) enum NodeColour {
    Red,
    Black,
}

impl NodeColour {
    pub(super) fn black_height(&self) -> u8 {
        match self {
            NodeColour::Red => 0,
            NodeColour::Black => 1,
        }
    }
}

impl std::fmt::Display for NodeColour {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            NodeColour::Red => write!(f, "red")?,
            NodeColour::Black => write!(f, "black")?,
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy)]
pub(super) struct NodeMetrics {
    pub(super) len: usize,
    pub(super) num_lines: usize,
}

impl NodeMetrics {
    pub(super) const EMPTY: NodeMetrics = NodeMetrics { len: 0, num_lines: 0 };
}

impl Add<&NodeMetrics> for NodeMetrics {
    type Output = NodeMetrics;

    fn add(self, rhs: &NodeMetrics) -> Self::Output {
        NodeMetrics { len: self.len + rhs.len, num_lines: self.num_lines + rhs.num_lines }
    }
}

impl AddAssign<&NodeMetrics> for NodeMetrics {
    fn add_assign(&mut self, rhs: &NodeMetrics) {
        *self = NodeMetrics { len: self.len + rhs.len, num_lines: self.num_lines + rhs.num_lines };
    }
}

impl Sub<&NodeMetrics> for NodeMetrics {
    type Output = NodeMetrics;

    fn sub(self, rhs: &NodeMetrics) -> Self::Output {
        NodeMetrics { len: self.len - rhs.len, num_lines: self.num_lines - rhs.num_lines }
    }
}

impl SubAssign<&NodeMetrics> for NodeMetrics {
    fn sub_assign(&mut self, rhs: &NodeMetrics) {
        *self = NodeMetrics { len: self.len - rhs.len, num_lines: self.num_lines - rhs.num_lines };
    }
}

#[derive(Debug)]
pub(super) enum Node {
    Branch {
        colour: NodeColour,
        left: Arc<Node>,
        right: Arc<Node>,
        metrics: NodeMetrics,
    },
    Leaf {
        // val: String,
        // len: usize,
        block_ref: BlockRange,
        metrics: NodeMetrics,
    },
    Empty,
}

impl Node {
    pub(super) fn empty() -> Self {
        Empty
    }

    pub(super) fn new_branch(colour: NodeColour, left: Arc<Node>, right: Arc<Node>) -> Self {
        let len = left.len() + right.len();
        let num_lines = left.num_lines() + right.num_lines();
        let metrics = NodeMetrics { len, num_lines };
        Branch { colour, left, right, metrics }
    }

    pub(super) fn new_leaf(val: BlockRange) -> Self {
        let num_lines = bytecount::count(val.as_bytes(), b'\n');
        let metrics = NodeMetrics { len: val.len(), num_lines };
        Leaf { block_ref: val, metrics }
    }

    pub(super) fn metrics(&self) -> NodeMetrics {
        match &self {
            Branch { metrics, .. } => *metrics,
            Leaf { metrics, .. } => *metrics,
            Empty => NodeMetrics::EMPTY,
        }
    }

    pub(super) fn len(&self) -> usize {
        self.metrics().len
    }

    pub(super) fn num_lines(&self) -> usize {
        self.metrics().num_lines
    }

    pub(super) fn colour(&self) -> NodeColour {
        match &self {
            Branch { colour, .. } => *colour,
            Empty | Leaf { .. } => NodeColour::Black,
        }
    }

    fn black_height(&self) -> Result<usize, Error> {
        match &self {
            Empty | Leaf { .. } => Ok(0),
            Branch { colour, left, right, .. } => {
                if *colour == NodeColour::Red {
                    if let Branch { colour: NodeColour::Red, .. } = left.as_ref() {
                        return Err(Error::ConsecutiveRed);
                    }
                    if let Branch { colour: NodeColour::Red, .. } = right.as_ref() {
                        return Err(Error::ConsecutiveRed);
                    }
                }

                let lheight = &left.black_height()?;
                let rheight = &right.black_height()?;
                if lheight != rheight {
                    return Err(Error::DifferingBlackHeight);
                }
                Ok(lheight + (colour.black_height() as usize))
            }
        }
    }

    pub(super) fn is_balanced(&self) -> bool {
        self.black_height().is_ok()
    }

    pub(super) fn write_dot(&self, w: &mut impl std::io::Write) -> std::io::Result<()> {
        match self {
            Branch { colour, left, right, .. } => {
                write!(w, "\tn{:p}[shape=circle,color={},label=\"\"];\n", self, colour)?;

                left.write_dot(w)?;
                write!(w, "\tn{:p} -> n{:p}[label=\"{}\"];\n", self, left.as_ref(), left.len())?;

                right.write_dot(w)?;
                write!(w, "\tn{:p} -> n{:p}[label=\"{}\"];\n", self, right.as_ref(), right.len())?;
            }
            Leaf { block_ref, .. } => {
                write!(w, "\tn{:p}[shape=square,label=\"len={}\"];\n", self, block_ref.len())?;
            }
            Empty => {
                write!(w, "\tn{:p}[shape=square,label=\"len=0\"];\n", self)?;
            }
        }
        Ok(())
    }

    pub(super) fn to_bstring(&self) -> BString {
        match self {
            Empty => b"".into(),
            Leaf { block_ref, .. } => block_ref.as_bytes().into(),
            Branch { left, right, .. } => {
                let mut bstr = left.to_bstring();
                bstr.push_str(right.to_bstring());
                bstr
            }
        }
    }
}

// impl From<Node> for Tree {
//     fn from(node: Node) -> Self {
//         Self(Arc::new(node))
//     }
// }

// pub(crate) struct TreeView<'a> {
//     tree: &'a Tree,
//     range: Range<usize>,
// }

// impl<'a> TreeView<'a> {
//     fn new(tree: &'a Tree, range: Range<usize>) -> Self {
//         Self { tree, range }
//     }
// }

// impl<'a> Iterator for TreeView<'a> {
//     type Item = &'a [u8];

//     fn next(&mut self) -> Option<Self::Item> {
//         if self.range.is_empty() {
//             return None;
//         }
//         match self.tree.leaf_node_at(self.range.start) {
//             None => None,
//             Some((leaf, leaf_start)) => {
//                 if let Leaf { block_ref, .. } = leaf {
//                     let len = self.range.len() - 1;
//                     self.range = (self.range.start + block_ref.len())..self.range.end;
//                     if self.range.is_empty() {
//                         Some(&block_ref.as_bytes()[leaf_start..(leaf_start + len)])
//                     } else {
//                         Some(&block_ref.as_bytes()[leaf_start..])
//                     }
//                 } else {
//                     unreachable!()
//                 }
//             }
//         }
//     }
// }

// fn byte_offset_for_line(node: &Node, linenum: usize) -> Option<usize> {
//     match node {
//         Empty => None,
//         Leaf { block_ref, .. } => {
//             if linenum == 1 {
//                 memchr::memchr(b'\n', block_ref.as_bytes())
//             } else {
//                 memchr::memchr_iter(b'\n', block_ref.as_bytes())
//                     .enumerate()
//                     .find(|(i, _)| *i == linenum - 1)
//                     .map(|(_, p)| p)
//             }
//         }
//         Branch { left, right, .. } => {
//             if linenum <= left.num_lines() {
//                 byte_offset_for_line(left.as_ref(), linenum)
//             } else {
//                 byte_offset_for_line(right.as_ref(), linenum - left.num_lines())
//                     .map(|o| o + left.len())
//             }
//         }
//     }
// }

// fn leaf_node_at<'a>(node: &'a Node, at: usize) -> (&'a Node, usize) {
//     match node {
//         Empty => {
//             debug_assert!(at == 0);
//             (node, at)
//         }
//         Leaf { block_ref, .. } => {
//             debug_assert!(at <= block_ref.len());
//             (node, at)
//         }
//         Branch { left, right, .. } => {
//             if at < left.len() {
//                 leaf_node_at(left.as_ref(), at)
//             } else {
//                 leaf_node_at(right.as_ref(), at - left.len())
//             }
//         }
//     }
// }
