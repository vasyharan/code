use std::fmt;
use std::sync::Arc;

mod cursor;
pub trait Summary: Default + Clone + fmt::Debug {
    fn combine(&self, other: &Self) -> Self;

    fn empty() -> Self {
        Default::default()
    }
}

pub trait Item: Clone {
    type Summary: Summary;

    fn summary(&self) -> Self::Summary;
}

#[derive(Debug, Clone)]
pub struct Tree<T: Item>(Arc<Node<T>>);

impl<T: Item> Tree<T> {
    pub fn summary(&self) -> &T::Summary {
        self.0.summary()
    }
}

#[derive(Debug)]
pub enum Node<T: Item> {
    Branch {
        colour: Colour,
        left: Tree<T>,
        right: Tree<T>,
        summary: T::Summary,
    },
    Leaf {
        item: T,
        summary: T::Summary,
    },
}

impl<T: Item> Node<T> {
    fn new_branch(colour: Colour, left: Tree<T>, right: Tree<T>) -> Self {
        let summary = left.0.summary().combine(right.0.summary());
        Node::Branch { colour, left, right, summary }
    }

    fn new_leaf(item: T) -> Self {
        let summary = item.summary();
        Node::Leaf { item, summary }
    }

    fn unwrap_item(&self) -> &T {
        match self {
            Node::Branch { .. } => panic!("called `Node::unwrap_item()` on a `Branch` node"),
            Node::Leaf { ref item, .. } => item,
        }
    }

    fn summary(&self) -> &T::Summary {
        match self {
            Node::Branch { ref summary, .. } => summary,
            Node::Leaf { ref summary, .. } => summary,
        }
    }

    fn colour(&self) -> Colour {
        match self {
            Node::Branch { colour, .. } => *colour,
            Node::Leaf { .. } => Colour::Black,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum Colour {
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
