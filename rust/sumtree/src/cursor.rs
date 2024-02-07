use std::sync::Arc;

use crate::{Item, Node, Tree};

#[derive(Debug, Clone)]
struct Cursor<T: Item> {
    tree: Tree<T>,
    ancestors: Vec<Arc<Node<T>>>,
    leaf: Option<Arc<Node<T>>>,
}

impl<T: Item> Cursor<T> {
    fn new(tree: Tree<T>) -> Self {
        Self { tree, ancestors: vec![], leaf: None }
    }

    fn reset(&mut self) {
        self.ancestors.clear();
        self.leaf = None;
    }

    fn seek_first(&mut self) -> &T {
        self.reset();
        let from_node = Some(&self.tree.0);
        self.leaf = Some(self.leftmost_leaf(from_node).clone());
        self.leaf.as_ref().unwrap().unwrap_item()
    }

    fn leftmost_leaf<'a>(&mut self, from_node: Option<&'a Arc<Node<T>>>) -> &'a Arc<Node<T>> {
        let mut maybe_node = from_node;
        while let Some(node) = maybe_node {
            match node.as_ref() {
                Node::Leaf { .. } => {
                    return node;
                }
                Node::Branch { left, .. } => {
                    self.ancestors.push(node.clone());
                    maybe_node = Some(&left.0);
                }
            }
        }
        unreachable!("tree must always end with leaf nodes")
    }
}
