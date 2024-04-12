use crate::{Colour, Item, Node, SumTree};

#[derive(Debug)]
pub enum Direction {
    Left,
    Right,
}

#[derive(Debug)]
pub struct Cursor<'a, T: Item> {
    tree: &'a SumTree<T>,
    ancestors: Vec<&'a SumTree<T>>,
    curr: Option<&'a SumTree<T>>,
}

impl<'a, T: Item> Cursor<'a, T> {
    pub fn new(tree: &'a SumTree<T>) -> Self {
        Self { tree, ancestors: vec![], curr: None }
    }

    pub fn into_position(mut self) -> CursorPosition<'a, T> {
        if let None = self.curr {
            self.goto_leftmost_leaf_from(self.tree);
        }
        match self.curr {
            None => unreachable!("empty sum tree"),
            Some(curr) => CursorPosition::new(curr, self.ancestors),
        }
    }

    pub fn reset(&mut self) {
        self.ancestors.clear();
        self.curr = None;
    }

    pub fn next(&mut self) -> Option<&'a SumTree<T>> {
        self.goto_next_leaf();
        self.curr
    }

    pub fn seek(
        &mut self,
        mut seek_fn: impl FnMut(&'a SumTree<T>) -> Direction,
    ) -> Option<&'a SumTree<T>> {
        if let None = self.curr {
            self.curr = Some(self.tree);
        }
        while let Some(next) = self.curr {
            match next.0.as_ref() {
                Node::Leaf { .. } => break,
                Node::Branch { .. } => match seek_fn(next) {
                    Direction::Left => self.goto_next_left_node_from(next),
                    Direction::Right => self.goto_next_right_node_from(next),
                },
            }
        }
        self.curr
    }

    fn goto_next_left_node_from(&mut self, from: &'a SumTree<T>) {
        match from.0.as_ref() {
            Node::Leaf { .. } => self.goto_next_right_node_from(from),
            Node::Branch { left, .. } => {
                self.ancestors.push(from);
                self.curr = Some(left);
            }
        }
    }

    fn goto_next_right_node_from(&mut self, from: &'a SumTree<T>) {
        match from.0.as_ref() {
            Node::Leaf { .. } => {
                let mut search_node = Some(from);
                while search_node.is_some() && !self.ancestors.is_empty() {
                    let node = search_node.unwrap();
                    let parent = self.ancestors[self.ancestors.len() - 1];
                    match parent.0.as_ref() {
                        Node::Leaf { .. } => unreachable!("leaf node on ancestors stack"),
                        Node::Branch { right, .. } => {
                            if node == right {
                                _ = self.ancestors.pop();
                                search_node = Some(parent);
                            } else {
                                self.curr = Some(right);
                                return;
                            }
                        }
                    }
                }
                self.curr = None
            }
            Node::Branch { right, .. } => {
                self.ancestors.push(from);
                self.curr = Some(right);
            }
        }
    }

    fn goto_next_leaf(&mut self) {
        match self.curr {
            None => self.goto_leftmost_leaf_from(self.tree),
            Some(curr) => self.goto_next_leaf_from(curr),
        }
    }

    fn goto_leftmost_leaf_from(&mut self, from: &'a SumTree<T>) {
        let mut maybe_from = Some(from);
        while let Some(from) = maybe_from {
            match from.0.as_ref() {
                Node::Leaf { .. } => {
                    self.curr = Some(from);
                    return;
                }
                Node::Branch { left, .. } => {
                    self.ancestors.push(from);
                    maybe_from = Some(left);
                }
            }
        }
        self.curr = None;
    }

    fn goto_next_leaf_from(&mut self, from: &'a SumTree<T>) {
        let mut maybe_from = Some(from);
        while maybe_from.is_some() && !self.ancestors.is_empty() {
            let from = maybe_from.unwrap();
            let parent = self.ancestors[self.ancestors.len() - 1];
            match parent.0.as_ref() {
                Node::Leaf { .. } => unreachable!("leaf node on ancestors stack"),
                Node::Branch { left, right, .. } => {
                    if from == left {
                        self.goto_leftmost_leaf_from(right);
                        return;
                    } else if from == right {
                        _ = self.ancestors.pop();
                        maybe_from = Some(parent);
                    } else {
                        unreachable!("node must be left/right child of parent")
                    }
                }
            }
        }
        self.curr = None;
    }
}

pub struct CursorPosition<'a, T: Item> {
    ancestors: Vec<&'a SumTree<T>>,
    curr: &'a SumTree<T>,
}

impl<'a, T: Item> CursorPosition<'a, T> {
    pub fn new(curr: &'a SumTree<T>, ancestors: Vec<&'a SumTree<T>>) -> Self {
        Self { ancestors, curr }
    }

    pub fn insert_left(self, item: T) -> SumTree<T> {
        let right = self.curr;
        let left = SumTree::new_leaf(item);
        let tree = SumTree::new_branch(Colour::Red, left, right.clone());
        self.balance(right, tree)
    }

    pub fn insert_right(self, item: T) -> SumTree<T> {
        let left = self.curr;
        let right = SumTree::new_leaf(item);
        let tree = SumTree::new_branch(Colour::Red, left.clone(), right);
        self.balance(left, tree)
    }

    fn balance(mut self, old: &SumTree<T>, new: SumTree<T>) -> SumTree<T> {
        let mut old = old;
        let mut new = new;
        while !self.ancestors.is_empty() {
            let parent = self.ancestors.pop().unwrap();
            match parent.0.as_ref() {
                Node::Leaf { .. } => unreachable!("leaf node on ancestors stack"),
                Node::Branch { colour, left, right, .. } => {
                    if old == left {
                        let (t, _) = balance(*colour, new, right.clone());
                        new = t;
                    } else if old == right {
                        let (t, _) = balance(*colour, left.clone(), new);
                        new = t;
                    } else {
                        unreachable!("parent is not left or right");
                    }
                }
            }

            old = parent;
        }

        make_black(new)
    }
}

fn make_black<T: Item>(tree: SumTree<T>) -> SumTree<T> {
    match tree.0.as_ref() {
        Node::Branch { colour: Colour::Red, left, right, .. } => {
            SumTree::new_branch(Colour::Black, left.clone(), right.clone())
        }
        _ => tree,
    }
}

fn balance<T: Item>(colour: Colour, left: SumTree<T>, right: SumTree<T>) -> (SumTree<T>, bool) {
    use Colour::*;

    if colour == Red {
        return (SumTree::new_branch(colour, left, right), false);
    }

    if let Node::Branch { colour: Red, left: ll, right: lr, .. } = left.0.as_ref() {
        if let Node::Branch { colour: Red, left: a, right: b, .. } = ll.0.as_ref() {
            let c = lr;
            let d = right;
            let l = SumTree::new_branch(Black, a.clone(), b.clone());
            let r = SumTree::new_branch(Black, c.clone(), d.clone());
            return (SumTree::new_branch(Red, l, r), true);
        } else if let Node::Branch { colour: Red, left: b, right: c, .. } = lr.0.as_ref() {
            let a = ll;
            let d = right;
            let l = SumTree::new_branch(Black, a.clone(), b.clone());
            let r = SumTree::new_branch(Black, c.clone(), d.clone());
            return (SumTree::new_branch(Red, l, r), true);
        }
    };

    if let Node::Branch { colour: Red, left: rl, right: rr, .. } = right.0.as_ref() {
        if let Node::Branch { colour: Red, left: b, right: c, .. } = rl.0.as_ref() {
            let a = left;
            let d = rr;
            let l = SumTree::new_branch(Black, a.clone(), b.clone());
            let r = SumTree::new_branch(Black, c.clone(), d.clone());
            return (SumTree::new_branch(Red, l, r), true);
        } else if let Node::Branch { colour: Red, left: c, right: d, .. } = rr.0.as_ref() {
            let a = left;
            let b = rl.clone();
            let l = SumTree::new_branch(Black, a.clone(), b.clone());
            let r = SumTree::new_branch(Black, c.clone(), d.clone());
            return (SumTree::new_branch(Red, l, r), true);
        }
    }

    (SumTree::new_branch(colour, left, right), false)
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use super::*;
    use crate::macros::*;
    use crate::tests::*;

    #[test]
    fn iterate_tests() {
        // single node tree
        let v1 = leaf!(V(1));
        let tree = v1.clone();
        let mut cursor = tree.cursor();
        assert_eq!(cursor.next(), Some(&v1));
        assert_eq!(cursor.next(), None);

        let mut cursor = tree.cursor();
        assert_eq!(cursor.next(), Some(&v1));
        assert_eq!(cursor.next(), None);

        // complex tree
        let v2 = leaf!(V(2));
        let b1 = branch_b!(v1.clone(), v2.clone());
        let v3 = leaf!(V(3));
        let v4 = leaf!(V(4));
        let b2 = branch_b!(v3.clone(), v4.clone());
        let b3 = branch_b!(b1.clone(), b2.clone());
        let v5 = leaf!(V(5));
        let v6 = leaf!(V(6));
        let b4 = branch_b!(v5.clone(), v6.clone());
        let v7 = leaf!(V(7));
        let b5 = branch_b!(b4.clone(), v7.clone());
        let b6 = branch_b!(b3.clone(), b5.clone());
        let tree = b6.clone();
        /*
         *                 b6
         *               /   \
         *             b3     b5
         *            /  \   /  \
         *           b1  b2 b4  7
         *          /\  /\ /\
         *         1 2 3 4 5 6
         */
        {
            let mut cursor = tree.cursor();
            assert_eq!(cursor.next(), Some(&v1));
            assert_eq!(cursor.next(), Some(&v2));
            assert_eq!(cursor.next(), Some(&v3));
            assert_eq!(cursor.next(), Some(&v4));
            assert_eq!(cursor.next(), Some(&v5));
            assert_eq!(cursor.next(), Some(&v6));
            assert_eq!(cursor.next(), Some(&v7));
            assert_eq!(cursor.next(), None);
        }
    }

    #[test]
    fn seek_tests() {
        // complex tree
        let v1 = leaf!(V(1));
        let v2 = leaf!(V(2));
        let b1 = branch_b!(v1.clone(), v2.clone());
        let v3 = leaf!(V(3));
        let v4 = leaf!(V(4));
        let b2 = branch_b!(v3.clone(), v4.clone());
        let b3 = branch_b!(b1.clone(), b2.clone());
        let v5 = leaf!(V(5));
        let v6 = leaf!(V(6));
        let b4 = branch_b!(v5.clone(), v6.clone());
        let v7 = leaf!(V(7));
        let b5 = branch_b!(b4.clone(), v7.clone());
        let b6 = branch_b!(b3.clone(), b5.clone());
        let tree = b6.clone();
        /*
         *                 b6
         *               /   \
         *             b3     b5
         *            /  \   /  \
         *           b1  b2 b4  7
         *          /\  /\ /\
         *         1 2 3 4 5 6
         */
        {
            let mut directions = vec![
                (&b2, Direction::Right),
                (&b3, Direction::Right),
                (&b6, Direction::Left),
            ];
            let mut cursor = tree.cursor();
            assert_eq!(
                cursor.seek(|node| {
                    let (expected, direction) = directions.pop().unwrap();
                    assert_eq!(node, expected);
                    direction
                }),
                Some(&v4)
            );
        }
        {
            let mut directions = vec![(&b5, Direction::Right), (&b6, Direction::Right)];
            let mut cursor = tree.cursor();
            assert_eq!(
                cursor.seek(|node| {
                    let (expected, direction) = directions.pop().unwrap();
                    assert_eq!(node, expected);
                    direction
                }),
                Some(&v7)
            );
        }
    }
}
