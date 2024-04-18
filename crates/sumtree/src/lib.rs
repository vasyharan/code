use std::sync::Arc;
use std::{fmt, ops::Deref};

pub mod cursor;
mod macros;

pub use cursor::{Cursor, Direction as CursorDirection};

pub trait Summary: Default + Clone + Copy + fmt::Debug {
    fn combine(&self, rhs: &Self) -> Self;

    fn scan_leaf(&mut self, lhs: &Self);
    fn scan_branch(&mut self, lhs: &Self);

    fn empty() -> Self {
        Default::default()
    }
}

pub trait Item: Clone + fmt::Debug {
    type Summary: Summary;

    fn summary(&self) -> Self::Summary;
}

#[derive(Debug)]
pub enum Error {
    ConsecutiveRed,
    DifferingBlackHeight,
}

#[derive(Debug, Clone)]
pub struct SumTree<T: Item>(Arc<Node<T>>);

impl<T: Item> Deref for SumTree<T> {
    type Target = Arc<Node<T>>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T: Item> SumTree<T> {
    pub fn new_leaf(item: T) -> Self {
        Self(Arc::new(Node::new_leaf(item)))
    }

    pub fn new_branch(colour: Colour, left: SumTree<T>, right: SumTree<T>) -> Self {
        Self(Arc::new(Node::new_branch(colour, left, right)))
    }

    pub fn deref_item(&self) -> &T {
        self.0.deref_item()
    }

    pub fn summary(&self) -> T::Summary {
        self.0.summary()
    }

    pub fn cursor(&self) -> Cursor<'_, T> {
        Cursor::new(self)
    }

    pub fn cursor_with_summary(&self) -> Cursor<'_, T> {
        Cursor::with_summary(self)
    }

    pub fn is_balanced(&self) -> bool {
        self.0.black_height().is_ok()
    }

    pub fn write_dot(&self, w: &mut impl std::io::Write) -> std::io::Result<()> {
        writeln!(w, "digraph G {{")?;
        self.0.write_dot(w)?;
        writeln!(w, "}}")?;
        Ok(())
    }
}

impl<T: Item> PartialEq for &SumTree<T> {
    fn eq(&self, other: &&SumTree<T>) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
}

#[derive(Debug)]
pub enum Node<T: Item> {
    Branch {
        colour: Colour,
        left: SumTree<T>,
        right: SumTree<T>,
        summary: T::Summary,
    },
    Leaf {
        item: T,
        summary: T::Summary,
    },
}

impl<T: Item> Node<T> {
    fn new_branch(colour: Colour, left: SumTree<T>, right: SumTree<T>) -> Self {
        let summary = left.0.summary().combine(&right.0.summary());
        Node::Branch { colour, left, right, summary }
    }

    fn new_leaf(item: T) -> Self {
        let summary = item.summary();
        Node::Leaf { item, summary }
    }

    pub fn deref_item(&self) -> &T {
        match self {
            Node::Branch { .. } => unreachable!("called `Node::deref_item()` on a `Branch` node"),
            Node::Leaf { ref item, .. } => item,
        }
    }

    fn summary(&self) -> T::Summary {
        match self {
            Node::Branch { summary, .. } => *summary,
            Node::Leaf { summary, .. } => *summary,
        }
    }

    fn colour(&self) -> Colour {
        match self {
            Node::Branch { colour, .. } => *colour,
            Node::Leaf { .. } => Colour::Black,
        }
    }

    fn black_height(&self) -> Result<usize, Error> {
        match self {
            Node::Leaf { .. } => Ok(0),
            Node::Branch { colour, left, right, .. } => {
                if *colour == Colour::Red {
                    if let Node::Branch { colour: Colour::Red, .. } = left.0.as_ref() {
                        return Err(Error::ConsecutiveRed);
                    }
                    if let Node::Branch { colour: Colour::Red, .. } = right.0.as_ref() {
                        return Err(Error::ConsecutiveRed);
                    }
                }

                let lheight = &left.0.black_height()?;
                let rheight = &right.0.black_height()?;
                if lheight != rheight {
                    return Err(Error::DifferingBlackHeight);
                }
                Ok(lheight + (colour.black_height() as usize))
            }
        }
    }

    fn write_dot(&self, w: &mut impl std::io::Write) -> std::io::Result<()> {
        match self {
            Node::Branch { colour, left, right, summary, .. } => {
                writeln!(
                    w,
                    "\tn{:p}[shape=circle,color={},label=\"{:?}\"];",
                    self, colour, summary
                )?;

                left.0.write_dot(w)?;
                writeln!(w, "\tn{:p} -> n{:p};", self, left.0.as_ref())?;

                right.0.write_dot(w)?;
                writeln!(w, "\tn{:p} -> n{:p};", self, right.0.as_ref())?;
            }
            Node::Leaf { item, .. } => {
                writeln!(w, "\tn{:p}[shape=square,label=\"{:?}\"];", self, item)?;
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Colour {
    Red,
    Black,
}

impl Colour {
    pub fn black_height(&self) -> u8 {
        match self {
            Colour::Red => 0,
            Colour::Black => 1,
        }
    }
}

impl fmt::Display for Colour {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Colour::Red => write!(f, "red")?,
            Colour::Black => write!(f, "black")?,
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::macros::*;

    #[test]
    fn build_tree() {
        let _ = std::fs::remove_dir_all("target/tests/");
        std::fs::create_dir_all("target/tests/").expect("create directory");

        let tree = leaf!(V(5));
        assert!(tree.is_balanced());
        assert_eq!(tree.summary(), Sum(5));

        let mut cursor = tree.cursor();
        assert_eq!(cursor.next().map(|n| n.0.deref_item()), Some(&V(5)));

        let tree = cursor.into_position().insert_left(V(4));
        assert!(tree.is_balanced());
        assert_eq!(tree.summary(), Sum(9));

        let mut cursor = tree.cursor();
        assert_eq!(cursor.next().map(|n| n.0.deref_item()), Some(&V(4)));
        assert_eq!(cursor.next().map(|n| n.0.deref_item()), Some(&V(5)));

        cursor.reset();
        let tree = cursor.into_position().insert_left(V(1));
        let mut file = std::fs::File::create("target/tests/insert01.dot").expect("create file");
        tree.write_dot(&mut file).expect("write dot file");
        assert!(tree.is_balanced());
        assert_eq!(tree.summary(), Sum(10));

        let mut cursor = tree.cursor();
        assert_eq!(cursor.next().map(|n| n.0.deref_item()), Some(&V(1)));
        assert_eq!(cursor.next().map(|n| n.0.deref_item()), Some(&V(4)));
        assert_eq!(cursor.next().map(|n| n.0.deref_item()), Some(&V(5)));

        let tree = cursor.into_position().insert_right(V(9));
        let mut file = std::fs::File::create("target/tests/insert09.dot").expect("create file");
        tree.write_dot(&mut file).expect("write dot file");
        assert!(tree.is_balanced());
        assert_eq!(tree.summary(), Sum(19));

        let mut cursor = tree.cursor();
        assert_eq!(cursor.next().map(|n| n.0.deref_item()), Some(&V(1)));
        assert_eq!(cursor.next().map(|n| n.0.deref_item()), Some(&V(4)));
        assert_eq!(cursor.next().map(|n| n.0.deref_item()), Some(&V(5)));
        assert_eq!(cursor.next().map(|n| n.0.deref_item()), Some(&V(9)));
    }

    #[derive(Debug, Clone, PartialEq)]
    pub(crate) struct V(pub(crate) u32);

    impl Item for V {
        type Summary = Sum;

        fn summary(&self) -> Self::Summary {
            Sum(self.0)
        }
    }

    #[derive(Debug, Default, Clone, Copy, PartialEq)]
    pub(crate) struct Sum(pub(crate) u32);

    impl Summary for Sum {
        fn combine(&self, rhs: &Self) -> Self {
            Sum(self.0 + rhs.0)
        }

        fn scan_branch(&mut self, lhs: &Self) {
            self.0 += lhs.0;
        }

        fn scan_leaf(&mut self, lhs: &Self) {
            self.0 += lhs.0;
        }
    }
}
