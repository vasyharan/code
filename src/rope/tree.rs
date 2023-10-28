use std::rc::Rc;

use crate::rope::block::BlockRange;
use bstr::{BString, ByteSlice, ByteVec};
use Node::{Branch, Empty, Leaf};

#[derive(Debug)]
pub enum Error {
    ConsecutiveRed,
    DifferingBlackHeight,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum NodeColour {
    Red,
    Black,
}

impl NodeColour {
    pub fn black_height(&self) -> u8 {
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

#[derive(Debug)]
enum Node {
    Branch {
        colour: NodeColour,
        left: Rc<Node>,
        right: Rc<Node>,
        len: usize,
    },
    Leaf {
        // val: String,
        // len: usize,
        block_ref: BlockRange,
    },
    Empty,
}

impl Node {
    fn empty() -> Self {
        Empty
    }

    fn new_branch(colour: NodeColour, left: Rc<Node>, right: Rc<Node>) -> Self {
        let len = left.len() + right.len();
        Branch { colour, left, right, len }
    }

    fn new_leaf(val: BlockRange) -> Self {
        Leaf { block_ref: val }
    }

    fn len(&self) -> usize {
        match &self {
            Branch { len, .. } => *len,
            Leaf { block_ref, .. } => block_ref.len(),
            Empty => 0,
        }
    }

    fn colour(&self) -> NodeColour {
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

    fn is_balanced(&self) -> bool {
        self.black_height().is_ok()
    }

    fn write_dot(&self, w: &mut impl std::io::Write) -> std::io::Result<()> {
        match self {
            Branch { colour, left, right, .. } => {
                write!(w, "\tn{:p}[shape=circle,color={},label=\"\"];\n", self, colour)?;

                left.write_dot(w)?;
                write!(w, "\tn{:p} -> n{:p}[label=\"{}\"];\n", self, left.as_ref(), left.len())?;

                right.write_dot(w)?;
                write!(w, "\tn{:p} -> n{:p}[label=\"{}\"];\n", self, right.as_ref(), right.len())?;
            }
            Leaf { block_ref, .. } => {
                let s = block_ref.as_bytes().as_bstr();
                write!(w, "\tn{:p}[shape=square,label=\"'{}'\"];\n", self, s)?;
            }
            Empty => {
                write!(w, "\tn{:p}[shape=square,label=\"''\"];\n", self)?;
            }
        }
        Ok(())
    }

    fn to_bstring(&self) -> BString {
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

#[derive(Debug, Clone)]
pub struct Tree(Rc<Node>);

impl Tree {
    pub fn empty() -> Self {
        let root = Rc::new(Node::empty());
        Self(root)
    }

    // pub fn from_str(str: &str) -> Self {
    //     let root = Rc::new(Node::new_leaf(str.to_owned()));
    //     Self(root)
    // }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn insert_at(&self, offset: usize, text: BlockRange) -> Self {
        fn rec(node: &Rc<Node>, offset: usize, text: BlockRange) -> Node {
            match node.as_ref() {
                Empty => Node::new_leaf(text),
                Leaf { block_ref, .. } => {
                    if node.len() == 0 {
                        Node::new_leaf(text)
                    } else if offset == 0 {
                        let left = Rc::new(Node::new_leaf(text));
                        Node::new_branch(NodeColour::Red, left, node.clone())
                    } else if offset == node.len() {
                        let right = Rc::new(Node::new_leaf(text));
                        Node::new_branch(NodeColour::Red, node.clone(), right)
                    } else {
                        let left = Rc::new(Node::new_leaf(block_ref.substr(..offset)));
                        let rl = Rc::new(Node::new_leaf(text));
                        let rr = Rc::new(Node::new_leaf(block_ref.substr(offset..)));
                        let right = Rc::new(Node::new_branch(NodeColour::Red, rl, rr));
                        Node::new_branch(NodeColour::Red, left, right)
                    }
                }
                Branch { colour, left, right, .. } => {
                    let left_len = left.len();
                    if left_len > offset {
                        let left = rec(left, offset, text);
                        let (node, _) = balance(*colour, Rc::new(left), right.clone());
                        node
                    } else {
                        let offset = offset - left_len;
                        let right = rec(right, offset, text);
                        let (node, _) = balance(*colour, left.clone(), Rc::new(right));
                        node
                    }
                }
            }
        }

        Tree(make_black(Rc::new(rec(&self.0, offset, text))))
    }

    pub fn split(&self, at: usize) -> (Option<Tree>, Option<Tree>) {
        let (left, right) = split(&self.0, at);
        debug_assert_split_balanced("split", &self.0, &left, &right);
        match (left, right) {
            (None, None) => (None, None),
            (None, Some((right, _))) => (None, Some(Tree(right))),
            (Some((left, _)), None) => (Some(Tree(left)), None),
            (Some((left, _)), Some((right, _))) => (Some(Tree(left)), Some(Tree(right))),
        }
    }

    pub fn delete_at(&self, offset: usize, len: usize) -> (Tree, Tree) {
        if len == 0 {
            let deleted = Self::empty();
            let updated = Self(self.0.clone());
            return (updated, deleted);
        }

        let (left, right) = split(&self.0, offset);
        debug_assert_split_balanced("delete_split1", &self.0, &left, &right);
        let (deleted, right) = split(&right.unwrap().0, len);
        debug_assert_split_balanced("delete_split2", &self.0, &left, &right);

        let updated = match join(left, right) {
            None => Self::empty(),
            Some((root, _)) => Self(make_black(root)),
        };
        let deleted = match deleted {
            None => Self::empty(),
            Some((root, _)) => Self(make_black(root)),
        };
        (updated, deleted)
    }

    pub fn write_dot(&self, w: &mut impl std::io::Write) -> std::io::Result<()> {
        write!(w, "digraph {{\n")?;
        self.0.write_dot(w)?;
        write!(w, "}}")?;
        Ok(())
    }

    pub fn is_balanced(&self) -> bool {
        self.0.is_balanced()
    }

    pub fn to_bstring(&self) -> BString {
        self.0.to_bstring()
    }
}

impl From<Node> for Tree {
    fn from(node: Node) -> Self {
        Self(Rc::new(node))
    }
}

fn make_black(node: Rc<Node>) -> Rc<Node> {
    match node.as_ref() {
        Branch { colour: NodeColour::Red, left, right, len, .. } => Rc::new(Branch {
            colour: NodeColour::Black,
            left: left.clone(),
            right: right.clone(),
            len: *len,
        }),
        _ => node,
    }
}

fn balance(colour: NodeColour, left: Rc<Node>, right: Rc<Node>) -> (Node, bool) {
    use NodeColour::{Black, Red};
    if colour == Red {
        return (Node::new_branch(colour, left, right), false);
    }

    if let Branch { colour: Red, left: ll, right: lr, .. } = left.as_ref() {
        if let Branch { colour: Red, left: a, right: b, .. } = ll.as_ref() {
            let c = lr;
            let d = right;
            let l = Node::new_branch(Black, a.clone(), b.clone());
            let r = Node::new_branch(Black, c.clone(), d.clone());
            return (Node::new_branch(Red, Rc::new(l), Rc::new(r)), true);
        } else if let Branch { colour: Red, left: b, right: c, .. } = lr.as_ref() {
            let a = ll;
            let d = right;
            let l = Node::new_branch(Black, a.clone(), b.clone());
            let r = Node::new_branch(Black, c.clone(), d.clone());
            return (Node::new_branch(Red, Rc::new(l), Rc::new(r)), true);
        }
    };

    if let Branch { colour: Red, left: rl, right: rr, .. } = right.as_ref() {
        if let Branch { colour: Red, left: b, right: c, .. } = rl.as_ref() {
            let a = left;
            let d = rr;
            let l = Node::new_branch(Black, a.clone(), b.clone());
            let r = Node::new_branch(Black, c.clone(), d.clone());
            return (Node::new_branch(Red, Rc::new(l), Rc::new(r)), true);
        } else if let Branch { colour: Red, left: c, right: d, .. } = rr.as_ref() {
            let a = left;
            let b = rl.clone();
            let l = Node::new_branch(Black, a.clone(), b.clone());
            let r = Node::new_branch(Black, c.clone(), d.clone());
            return (Node::new_branch(Red, Rc::new(l), Rc::new(r)), true);
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
fn join_right(left: (Rc<Node>, usize), right: (Rc<Node>, usize)) -> (Rc<Node>, usize) {
    let (left, lheight) = left;
    let (right, rheight) = right;
    debug_assert_eq!(lheight, black_height(left.as_ref()));
    debug_assert_eq!(rheight, black_height(right.as_ref()));
    if lheight == rheight {
        if let Branch { colour, .. } = left.as_ref() {
            if *colour == NodeColour::Black {
                let node = Node::new_branch(NodeColour::Red, left.clone(), right.clone());
                return (Rc::new(node), lheight);
            }
        } else {
            let node = Node::new_branch(NodeColour::Red, left.clone(), right.clone());
            return (Rc::new(node), lheight);
        }
    }
    match (left.as_ref(), right.as_ref()) {
        (Branch { colour, left: ll, right: lr, .. }, _) => {
            let lrheight = lheight - (colour.black_height() as usize);
            let (right, jrheight) = join_right((lr.clone(), lrheight), (right, rheight));
            let (node, _) = balance(*colour, ll.clone(), right);
            (Rc::new(node), jrheight + (colour.black_height() as usize))
        }
        _ => unreachable!(),
    }
}

fn join_left(left: (Rc<Node>, usize), right: (Rc<Node>, usize)) -> (Rc<Node>, usize) {
    let (left, lheight) = left;
    let (right, rheight) = right;
    debug_assert_eq!(lheight, black_height(left.as_ref()));
    debug_assert_eq!(rheight, black_height(right.as_ref()));
    if lheight == rheight {
        if let Branch { colour, .. } = right.as_ref() {
            if *colour == NodeColour::Black {
                let node = Node::new_branch(NodeColour::Red, left.clone(), right.clone());
                return (Rc::new(node), lheight);
            }
        } else {
            let node = Node::new_branch(NodeColour::Red, left.clone(), right.clone());
            return (Rc::new(node), lheight);
        }
    }
    match (left.as_ref(), right.as_ref()) {
        (_, Branch { colour, left: rl, right: rr, .. }) => {
            let rlheight = rheight - (colour.black_height() as usize);
            let (left, jlheight) = join_left((left, lheight), (rl.clone(), rlheight));
            let (node, _) = balance(*colour, left, rr.clone());
            (Rc::new(node), jlheight + (colour.black_height() as usize))
        }
        _ => unreachable!(),
    }
}

fn join(
    maybe_left: Option<(Rc<Node>, usize)>,
    maybe_right: Option<(Rc<Node>, usize)>,
) -> Option<(Rc<Node>, usize)> {
    match (maybe_left, maybe_right) {
        (None, None) => None,
        (None, Some(right)) => Some(right.clone()),
        (Some(left), None) => Some(left.clone()),
        (Some((left, lheight)), Some((right, rheight))) => {
            debug_assert_eq!(lheight, black_height(left.as_ref()));
            debug_assert_eq!(rheight, black_height(right.as_ref()));
            // let lheight = black_height(left.as_ref());
            // let rheight = black_height(right.as_ref());
            if rheight > lheight {
                Some(join_left((left, lheight), (right, rheight)))
            } else if lheight > rheight {
                Some(join_right((left, lheight), (right, rheight)))
            } else {
                let colour =
                    if left.colour() == NodeColour::Black && right.colour() == NodeColour::Black {
                        NodeColour::Red
                    } else {
                        NodeColour::Black
                    };
                let node = Node::new_branch(colour, left.clone(), right.clone());
                Some((Rc::new(node), lheight + (colour.black_height() as usize)))
            }
        }
    }
}

fn split(node: &Node, at: usize) -> (Option<(Rc<Node>, usize)>, Option<(Rc<Node>, usize)>) {
    fn split_rec(
        node: &Node,
        at: usize,
    ) -> (Option<(Rc<Node>, usize)>, Option<(Rc<Node>, usize)>, usize) {
        match node {
            Empty => (None, None, 0),
            Leaf { block_ref, .. } => {
                // TODO: stop making copies if possible
                let split_left = if at == 0 {
                    None
                } else {
                    Some((Rc::new(Node::new_leaf(block_ref.substr(..at))), 0))
                };
                let split_right = if at == block_ref.len() {
                    None
                } else {
                    Some((Rc::new(Node::new_leaf(block_ref.substr(at..))), 0))
                };
                (split_left, split_right, 0)
            }
            Branch { colour, left, right, .. } => {
                if at <= left.len() {
                    let (split_left, split_right, lheight) = split_rec(left, at);
                    let join_right = Some((right.clone(), lheight));
                    let split_right = join(split_right, join_right);
                    let height = lheight + (colour.black_height() as usize);
                    (split_left, split_right, height)
                } else {
                    let (split_left, split_right, rheight) = split_rec(right, at - left.len());
                    let join_left = Some((left.clone(), rheight));
                    let split_left = join(join_left, split_left);
                    let height = rheight + (colour.black_height() as usize);
                    (split_left, split_right, height)
                }
            }
        }
    }

    let (left, right, _) = split_rec(node, at);
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

fn debug_assert_split_balanced(
    prefix: &str,
    pre_split: &Rc<Node>,
    split_left: &Option<(Rc<Node>, usize)>,
    split_right: &Option<(Rc<Node>, usize)>,
) {
    if cfg!(debug_assertions) {
        let left_bal = if let Some((ref node, _)) = split_left {
            node.is_balanced()
        } else {
            true
        };
        let right_bal = if let Some((ref node, _)) = split_right {
            node.is_balanced()
        } else {
            true
        };

        if !left_bal || !right_bal {
            std::fs::create_dir_all("target/assert/").expect("create directory");
            let mut file = std::fs::File::create(format!("target/assert/{}_pre_split.dot", prefix))
                .expect("create file");
            Tree(pre_split.clone())
                .write_dot(&mut file)
                .expect("write dot file");

            if let Some((ref node, _)) = split_left {
                let mut file =
                    std::fs::File::create(format!("target/assert/{}_split_left.dot", prefix))
                        .expect("create file");
                Tree(node.clone())
                    .write_dot(&mut file)
                    .expect("write dot file");
            }
            if let Some((ref node, _)) = split_right {
                let mut file =
                    std::fs::File::create(format!("target/assert/{}_split_right.dot", prefix))
                        .expect("create file");
                Tree(node.clone())
                    .write_dot(&mut file)
                    .expect("write dot file");
            }
            assert!(false, "left/right tree unbalanced post split");
        }
    }
}
