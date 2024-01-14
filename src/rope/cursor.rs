use std::sync::Arc;

use super::tree::Node;
use super::Rope;

#[derive(Debug)]
pub(crate) struct Cursor {
    rope: Rope,
    index: usize,
    parents: Vec<Arc<Node>>,
    pos: Option<(Arc<Node>, usize)>,
}

impl Cursor {
    pub(crate) fn new(rope: Rope) -> Self {
        let index = 0;
        let mut parents = vec![];
        let pos = None;
        Self { rope, index, parents, pos }
    }

    pub(crate) fn byte_offset(&self) -> usize {
        self.index
    }

    pub(crate) fn next(&mut self) -> Option<(usize, usize)> {
        let size = 1;
        match self.forward_by(size) {
            0 => None,
            res => {
                let index = self.index;
                self.index += res;
                Some((index, self.index))
            }
        }
    }

    pub(crate) fn prev(&mut self) -> Option<(usize, usize)> {
        let size = 1;
        match self.backward_by(size) {
            0 => None,
            res => {
                let index = self.index;
                self.index -= res;
                Some((self.index, index))
            }
        }
    }

    pub fn peek_byte(&self) -> Option<u8> {
        let bs = self.peek();
        if bs.is_empty() {
            None
        } else {
            Some(bs[0])
        }
    }

    fn peek(&self) -> &[u8] {
        match self.pos {
            None => &[],
            Some((ref leaf, offset)) => match leaf.as_ref() {
                Node::Branch { .. } => unreachable!(),
                Node::Empty => &[],
                Node::Leaf { block_ref, .. } => &block_ref.as_bytes()[offset..],
            },
        }
    }

    fn forward_by(&mut self, len: usize) -> usize {
        if self.index == 0 {
            if let None = self.pos {
                assert!(self.parents.is_empty());
                self.pos = Some(leftmost_leaf(&mut self.parents, &self.rope.0));
            }
        }

        let mut result = 0;
        while result < len {
            match self.pos {
                None => break,
                Some((ref leaf, ref mut offset)) => match leaf.as_ref() {
                    Node::Branch { .. } => unreachable!(),
                    Node::Empty => return 0,
                    Node::Leaf { block_ref, .. } => {
                        let bs = &block_ref.as_bytes()[*offset..];
                        if bs.len() > len {
                            result += len;
                            *offset += len;
                        } else {
                            result += bs.len();
                            self.pos = next_leaf(&mut self.parents, leaf);
                            // match next_leaf(&mut self.parents, leaf) {
                            //     None => {
                            //         self.pos = None;
                            //         return result;
                            //     }
                            //     next => self.pos = next,
                            // }
                        }
                    }
                },
            }
        }

        result
    }

    fn backward_by(&mut self, len: usize) -> usize {
        if self.index == self.rope.len() {
            if let None = self.pos {
                assert!(self.parents.is_empty());
                self.pos = Some(rightmost_leaf(&mut self.parents, &self.rope.0));
            }
        }

        let mut result = 0;
        while result < len {
            match self.pos {
                None => break,
                Some((ref leaf, ref mut offset)) => match leaf.as_ref() {
                    Node::Branch { .. } => unreachable!(),
                    Node::Empty => return 0,
                    Node::Leaf { block_ref, .. } => {
                        let bs = &block_ref.as_bytes()[..*offset];
                        if bs.len() >= len {
                            result += len;
                            *offset -= len;
                        } else {
                            result += bs.len();
                            self.pos = prev_leaf(&mut self.parents, leaf);
                            // match prev_leaf(&mut self.parents, leaf) {
                            //     Some(prev) => self.curr_leaf = prev,
                            //     None => {
                            //         assert!(self.parents.is_empty());
                            //         self.curr_leaf = leftmost_leaf(&mut self.parents, &self.rope.0);
                            //         return result;
                            //     }
                            // }
                        }
                    }
                },
            }
        }

        result
    }
}

fn leaf_at_byte_offset(
    parents: &mut Vec<Arc<Node>>,
    node: &Arc<Node>,
    offset: usize,
) -> Option<(Arc<Node>, usize)> {
    let mut node = node;
    let mut offset = offset;
    while offset < node.len() {
        match node.as_ref() {
            Node::Empty { .. } => unreachable!(),
            Node::Leaf { .. } => {
                return Some((node.clone(), offset));
            }
            Node::Branch { left, right, .. } => {
                parents.push(node.clone());
                if offset < left.len() {
                    node = left;
                } else {
                    offset -= left.len();
                    node = right;
                }
            }
        }
    }

    None
}

fn leftmost_leaf(parents: &mut Vec<Arc<Node>>, from_node: &Arc<Node>) -> (Arc<Node>, usize) {
    let mut maybe_node = Some(from_node);
    while let Some(node) = maybe_node {
        match node.as_ref() {
            Node::Empty => unreachable!(),
            Node::Leaf { .. } => {
                return (node.clone(), 0);
            }
            Node::Branch { left, .. } => {
                parents.push(node.clone());
                maybe_node = Some(left);
            }
        }
    }
    unreachable!()
}

fn rightmost_leaf(parents: &mut Vec<Arc<Node>>, from_node: &Arc<Node>) -> (Arc<Node>, usize) {
    let mut maybe_node = Some(from_node);
    while let Some(node) = maybe_node {
        match node.as_ref() {
            Node::Empty => unreachable!(),
            Node::Leaf { metrics, .. } => {
                return (node.clone(), metrics.len);
            }
            Node::Branch { right, .. } => {
                parents.push(node.clone());
                maybe_node = Some(right);
            }
        }
    }
    unreachable!()
}

fn next_leaf(parents: &mut Vec<Arc<Node>>, from_leaf: &Arc<Node>) -> Option<(Arc<Node>, usize)> {
    let mut search_node: Option<Arc<Node>> = Some(from_leaf.clone());
    while search_node.is_some() && !parents.is_empty() {
        let node = search_node.unwrap();
        let parent = parents[parents.len() - 1].clone();
        match parent.as_ref() {
            Node::Leaf { .. } | Node::Empty => unreachable!(),
            Node::Branch { left, right, .. } => {
                if std::ptr::eq(left.as_ref(), node.as_ref()) {
                    return Some(leftmost_leaf(parents, right));
                } else if std::ptr::eq(right.as_ref(), node.as_ref()) {
                    _ = parents.pop();
                    search_node = Some(parent);
                } else {
                    unreachable!()
                }
            }
        }
    }

    None
}

fn prev_leaf(parents: &mut Vec<Arc<Node>>, from_leaf: &Arc<Node>) -> Option<(Arc<Node>, usize)> {
    let mut search_node: Option<Arc<Node>> = Some(from_leaf.clone());
    while search_node.is_some() && !parents.is_empty() {
        let node = search_node.unwrap();
        let parent = parents[parents.len() - 1].clone();
        match parent.as_ref() {
            Node::Leaf { .. } | Node::Empty => unreachable!(),
            Node::Branch { left, right, .. } => {
                if std::ptr::eq(right.as_ref(), node.as_ref()) {
                    return Some(rightmost_leaf(parents, left));
                } else if std::ptr::eq(left.as_ref(), node.as_ref()) {
                    _ = parents.pop();
                    search_node = Some(parent);
                } else {
                    unreachable!()
                }
            }
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use crate::rope::{BlockBuffer, Rope};

    #[test]
    fn cursor_next_prev() {
        let variants = vec![
            build_rope(vec!["0123456789\n"]),
            build_rope(vec!["01", "2345", "6", "789\n"]),
            // build_rope(vec!["01", "2345", "6", "789", "\n"]),
            // build_rope(vec!["01", "2345", "6", "789\n0"]),
        ];

        const LINE_LEN: usize = 11;
        let line = "0123456789\n".as_bytes();
        for (_variant, rope) in variants.iter().enumerate() {
            let mut cursor = rope.cursor();
            let bstr = rope.to_bstring();
            for i in 0..LINE_LEN {
                let next = cursor.next();
                assert_eq!(next, Some((i, i + 1)));
                assert_eq!(bstr[i], line[i]);

                assert_eq!(cursor.prev(), next);
                assert_eq!(cursor.next(), next);
            }
            assert_eq!(cursor.next(), None);

            for i in 0..LINE_LEN {
                let i = LINE_LEN - i - 1;
                let prev = cursor.prev();
                assert_eq!(prev, Some((i, i + 1)));
                assert_eq!(bstr[i], line[i]);

                assert_eq!(cursor.next(), prev);
                assert_eq!(cursor.prev(), prev);
            }
            assert_eq!(cursor.prev(), None);
        }
    }

    fn build_rope(parts: Vec<&str>) -> Rope {
        let mut rope = Rope::empty();
        let mut buffer = BlockBuffer::new();
        for (_i, p) in parts.iter().enumerate() {
            let (block, w) = buffer.append(p.as_bytes()).unwrap();
            assert_eq!(w, p.len());
            rope = rope.insert(rope.len(), block).unwrap();
            assert!(rope.is_balanced());
        }
        rope
    }
}
