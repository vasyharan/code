use std::sync::Arc;

use super::tree::Node;
use super::Rope;

#[derive(Debug, Clone, Copy)]
pub(crate) struct Point {
    pub(crate) line: usize,
    pub(crate) column: usize,
}

impl Default for Point {
    fn default() -> Self {
        Self { line: 1, column: 1 }
    }
}

#[derive(Debug)]
pub(crate) struct Cursor {
    rope: Rope,
    byte_offset: usize,
    point: Point,
    ancestors: Vec<Arc<Node>>,
    leaf: Option<(Arc<Node>, usize)>,
}

impl Cursor {
    pub(crate) fn new(rope: Rope) -> Self {
        Self { rope, byte_offset: 0, point: Point::default(), ancestors: vec![], leaf: None }
    }

    pub(crate) fn byte_offset(&self) -> usize {
        self.byte_offset
    }

    pub(crate) fn point(&self) -> Point {
        self.point
    }

    // TODO: make this iterate over graphemes
    pub(crate) fn next(&mut self) -> Option<u8> {
        self.forward_byte().and_then(|_| self.peek_byte())
    }

    // TODO: make this iterator over graphemes
    pub(crate) fn prev(&mut self) -> Option<u8> {
        self.backward_byte().and_then(|_| self.peek_byte())
    }

    fn peek_byte(&self) -> Option<u8> {
        let bs = self.peek();
        if bs.is_empty() {
            None
        } else {
            Some(bs[0])
        }
    }

    fn peek(&self) -> &[u8] {
        match self.leaf {
            None => &[],
            Some((ref leaf, offset)) => match leaf.as_ref() {
                Node::Branch { .. } => unreachable!(),
                Node::Empty => &[],
                Node::Leaf { slab, .. } => &slab.as_bytes()[offset..],
            },
        }
    }

    fn forward_byte(&mut self) -> Option<()> {
        if self.byte_offset == 0 && self.leaf.is_none() {
            assert!(self.ancestors.is_empty());
            self.leaf = Some(leftmost_leaf(&mut self.ancestors, &self.rope.0));
        }

        loop {
            match self.leaf {
                None => return None,
                Some((ref leaf, ref mut offset)) => match leaf.as_ref() {
                    Node::Branch { .. } => unreachable!(),
                    Node::Empty => return None,
                    Node::Leaf { slab, .. } => {
                        let bs = &slab.as_bytes()[*offset..];
                        if bs.len() > 0 {
                            if bs[0] == b'\n' {
                                self.point.line += 1;
                                self.point.column = 1;
                            } else {
                                self.point.column += 1;
                            }
                            *offset += 1;
                            self.byte_offset += 1;
                            return Some(());
                        } else {
                            self.leaf = next_leaf(&mut self.ancestors, leaf);
                        }
                    }
                },
            }
        }
    }

    fn backward_byte(&mut self) -> Option<()> {
        if self.byte_offset == self.rope.len() && self.leaf.is_none() {
            assert!(self.ancestors.is_empty());
            self.leaf = Some(rightmost_leaf(&mut self.ancestors, &self.rope.0));
        }

        loop {
            match self.leaf {
                None => return None,
                Some((ref leaf, ref mut offset)) => match leaf.as_ref() {
                    Node::Branch { .. } => unreachable!(),
                    Node::Empty => return None,
                    Node::Leaf { slab, .. } => {
                        let bs = &slab.as_bytes()[..*offset];
                        if bs.len() >= 1 {
                            if bs[bs.len() - 1] == b'\n' {
                                self.point.line -= 1;
                                // TODO: fix me count columns in line
                                self.point.column = 1;
                            } else {
                                self.point.column -= 1;
                            }
                            *offset -= 1;
                            self.byte_offset -= 1;
                            return Some(());
                        } else {
                            self.leaf = prev_leaf(&mut self.ancestors, leaf);
                        }
                    }
                },
            }
        }
    }
}

// #[derive(Debug, Clone)]
// struct LeafNodes {
//     ancestors: Vec<Arc<Node>>,
//     curr: Option<Arc<Node>>,
// }

// impl LeafNodes {
//     fn new(rope: Rope) -> Self {
//         Self { ancestors: vec![], curr: None }
//     }

//     fn next(&mut self) -> Option<Arc<Node>> {
//         if self.byte_offset == 0 && self.curr.is_none() {
//             assert!(self.ancestors.is_empty());
//             self.curr = Some(leftmost_leaf2(&mut self.ancestors, &self.rope.0));
//         }
//         if self.curr.is_none() {
//             return None;
//         }

//         let next = self.curr.take().unwrap();
//         self.curr = next_leaf2(&mut self.ancestors, &next);
//         Some(next)
//     }

//     fn prev(&mut self) -> Option<Arc<Node>> {
//         if self.curr.is_none() {
//             return None;
//         }

//         let next = self.curr.take().unwrap();
//         self.curr = next_leaf2(&mut self.ancestors, &next);
//         Some(next)
//     }
// }

// fn leftmost_leaf2(ancestors: &mut Vec<Arc<Node>>, from_node: &Arc<Node>) -> Arc<Node> {
//     leftmost_leaf(ancestors, from_node).0
// }

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

// fn next_leaf2(ancestors: &mut Vec<Arc<Node>>, from_leaf: &Arc<Node>) -> Option<Arc<Node>> {
//     next_leaf(ancestors, from_leaf).map(|r| r.0)
// }

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

// fn prev_leaf2(ancestors: &mut Vec<Arc<Node>>, from_leaf: &Arc<Node>) -> Option<Arc<Node>> {
//     prev_leaf(ancestors, from_leaf).map(|r| r.0)
// }

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
    use crate::rope::{Rope, SlabAllocator};

    #[test]
    fn cursor_next_prev() {
        let variants = vec![
            build_rope(vec!["0123456789\n123"]),
            build_rope(vec!["01", "2345", "6", "789\n1", "23"]),
            build_rope(vec!["01", "2345", "6", "789", "\n", "123"]),
        ];

        const LINE_LEN: usize = 14;
        for (_variant, rope) in variants.iter().enumerate() {
            let mut cursor = rope.cursor();
            let bstr = rope.to_bstring();
            for i in 0..LINE_LEN {
                let next = cursor.next();
                assert_eq!(next, Some(bstr[i]));

                assert_eq!(cursor.prev(), next);
                assert_eq!(cursor.next(), next);
            }
            assert_eq!(cursor.next(), None);

            for i in 0..LINE_LEN {
                let i = LINE_LEN - i - 1;
                let prev = cursor.prev();
                assert_eq!(prev, Some(bstr[i]));

                assert_eq!(cursor.next(), prev);
                assert_eq!(cursor.prev(), prev);
            }
            assert_eq!(cursor.prev(), None);
        }
    }

    fn build_rope(parts: Vec<&str>) -> Rope {
        let mut rope = Rope::empty();
        let mut buffer = SlabAllocator::new();
        for (_i, p) in parts.iter().enumerate() {
            let (block, w) = buffer.append(p.as_bytes()).unwrap();
            assert_eq!(w, p.len());
            rope = rope.insert(rope.len(), block).unwrap();
            assert!(rope.is_balanced());
        }
        rope
    }
}
