use std::sync::Arc;

use crate::rope::block::BlockRange;
use bstr::{BString, ByteVec};
use Node::{Branch, Empty, Leaf};

#[derive(Debug)]
pub(crate) enum Error {
    ConsecutiveRed,
    DifferingBlackHeight,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum NodeColour {
    Red,
    Black,
}

impl NodeColour {
    pub(crate) fn black_height(&self) -> u8 {
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
        left: Arc<Node>,
        right: Arc<Node>,
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

    fn new_branch(colour: NodeColour, left: Arc<Node>, right: Arc<Node>) -> Self {
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
                write!(w, "\tn{:p}[shape=square,label=\"len={}\"];\n", self, block_ref.len())?;
            }
            Empty => {
                write!(w, "\tn{:p}[shape=square,label=\"len=0\"];\n", self)?;
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
pub(crate) struct Tree(Arc<Node>);

impl Tree {
    pub(crate) fn empty() -> Self {
        let root = Arc::new(Node::empty());
        Self(root)
    }

    // pub fn from_str(str: &str) -> Self {
    //     let root = Arc::new(Node::new_leaf(str.to_owned()));
    //     Self(root)
    // }

    pub(crate) fn len(&self) -> usize {
        self.0.len()
    }

    pub fn insert_at(&self, offset: usize, text: BlockRange) -> Self {
        fn rec(node: &Arc<Node>, offset: usize, text: BlockRange) -> Node {
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
                        let left = rec(left, offset, text);
                        let (node, _) = balance(*colour, Arc::new(left), right.clone());
                        node
                    } else {
                        let offset = offset - left_len;
                        let right = rec(right, offset, text);
                        let (node, _) = balance(*colour, left.clone(), Arc::new(right));
                        node
                    }
                }
            }
        }

        Tree(make_black(Arc::new(rec(&self.0, offset, text))))
    }

    pub(crate) fn split(&self, at: usize) -> (Option<Tree>, Option<Tree>) {
        let (left, right) = split(&self.0, at);
        debug_assert_split_balanced("split", at, &self.0, &left, &right);
        match (left, right) {
            (None, None) => (None, None),
            (None, Some((right, _))) => (None, Some(Tree(right))),
            (Some((left, _)), None) => (Some(Tree(left)), None),
            (Some((left, _)), Some((right, _))) => (Some(Tree(left)), Some(Tree(right))),
        }
    }

    pub(crate) fn delete_at(&self, offset: usize, len: usize) -> (Tree, Tree) {
        if len == 0 {
            let deleted = Self::empty();
            let updated = Self(self.0.clone());
            return (updated, deleted);
        }

        let (left, right) = split(&self.0, offset);
        debug_assert_split_balanced("delete_split1", offset, &self.0, &left, &right);

        let s2root = right.unwrap().0;
        let (deleted, right) = split(&s2root, len);
        debug_assert_split_balanced("delete_split2", len, &s2root, &deleted, &right);

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

    pub(crate) fn is_balanced(&self) -> bool {
        self.0.is_balanced()
    }

    pub(crate) fn to_bstring(&self) -> BString {
        self.0.to_bstring()
    }
}

impl From<Node> for Tree {
    fn from(node: Node) -> Self {
        Self(Arc::new(node))
    }
}

fn make_black(node: Arc<Node>) -> Arc<Node> {
    match node.as_ref() {
        Branch { colour: NodeColour::Red, left, right, len, .. } => Arc::new(Branch {
            colour: NodeColour::Black,
            left: left.clone(),
            right: right.clone(),
            len: *len,
        }),
        _ => node,
    }
}

fn balance(colour: NodeColour, left: Arc<Node>, right: Arc<Node>) -> (Node, bool) {
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
    debug_assert_join_balanced("join", &maybe_left, &maybe_right, &joined);
    joined
}

fn split(node: &Node, at: usize) -> (Option<(Arc<Node>, usize)>, Option<(Arc<Node>, usize)>) {
    fn split_rec(
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
    at: usize,
    pre_split: &Arc<Node>,
    split_left: &Option<(Arc<Node>, usize)>,
    split_right: &Option<(Arc<Node>, usize)>,
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
            assert!(left_bal, "left tree unbalanced {} post split at {}", prefix, at);
            assert!(right_bal, "right tree unbalanced {} post split at {}", prefix, at);
        }
    }
}

fn debug_assert_join_balanced(
    prefix: &str,
    left: &Option<(Arc<Node>, usize)>,
    right: &Option<(Arc<Node>, usize)>,
    joined: &Option<(Arc<Node>, usize)>,
) {
    if cfg!(debug_assertions) {
        let left_bal = if let Some((ref node, _)) = left {
            node.is_balanced()
        } else {
            true
        };
        let right_bal = if let Some((ref node, _)) = right {
            node.is_balanced()
        } else {
            true
        };
        let joined_bal = if let Some((ref node, _)) = joined {
            node.is_balanced()
        } else {
            true
        };

        if !left_bal || !right_bal || !joined_bal {
            std::fs::create_dir_all("target/assert/").expect("create directory");

            if let Some((ref node, _)) = left {
                let mut file =
                    std::fs::File::create(format!("target/assert/{}_join_left.dot", prefix))
                        .expect("create file");
                Tree(node.clone())
                    .write_dot(&mut file)
                    .expect("write dot file");
            }

            if let Some((ref node, _)) = right {
                let mut file =
                    std::fs::File::create(format!("target/assert/{}_join_right.dot", prefix))
                        .expect("create file");
                Tree(node.clone())
                    .write_dot(&mut file)
                    .expect("write dot file");
            }

            if let Some((ref node, _)) = joined {
                let mut file =
                    std::fs::File::create(format!("target/assert/{}_joined.dot", prefix))
                        .expect("create file");
                Tree(node.clone())
                    .write_dot(&mut file)
                    .expect("write dot file");
            }
            assert!(left_bal, "left tree unbalanced {} pre join", prefix);
            assert!(right_bal, "right tree unbalanced {} pre join", prefix);
            assert!(joined_bal, "joined tree unbalanced {} post join", prefix);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rope::block::BlockBuffer;

    use rand::{Rng, SeedableRng};

    enum Operation {
        Insert { at: usize, buf: Vec<u8> },
        Delete { at: usize, len: usize },
    }

    impl Operation {
        const GEN_BLOCK_SIZE: usize = 8192;

        fn gen_valid_insert(rng: &mut impl Rng, tree: &Tree) -> Self {
            let at = if tree.len() == 0 {
                0
            } else {
                rng.gen_range(0..tree.len())
            };
            let mut buf: Vec<u8> = vec![0; rng.gen_range(0..Self::GEN_BLOCK_SIZE)];
            rng.fill_bytes(&mut buf);
            Operation::Insert { at, buf }
        }

        fn gen_valid_delete(rng: &mut impl Rng, tree: &Tree) -> Self {
            let at = rng.gen_range(0..tree.len());
            let len = rng.gen_range(0..(tree.len() - at));
            Operation::Delete { at, len }
        }

        fn gen_valid(rng: &mut impl Rng, tree: &Tree) -> Self {
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

    macro_rules! branch {
        ($colour:expr, $left:expr, $right:expr $(,)?) => {{
            std::sync::Arc::new(Node::new_branch($colour, $left, $right))
        }};
    }

    macro_rules! branchr {
        ($left:expr, $right:expr $(,)?) => {{
            branch!(NodeColour::Red, $left, $right)
        }};
    }

    macro_rules! branchb {
        ($left:expr, $right:expr $(,)?) => {{
            branch!(NodeColour::Black, $left, $right)
        }};
    }

    macro_rules! leaf {
        ($buffer:ident, $size:literal) => {{
            let mut i = 0;
            loop {
                i += 1;
                let (block, written) = $buffer.append(&[0; $size]).expect("block append");
                if written == $size {
                    break Arc::new(Node::new_leaf(block));
                }
                if i == 10 {
                    unreachable!()
                }
            }
        }};
    }

    #[test]
    fn regression_1() {
        let mut buf = BlockBuffer::new();

        let tree = Tree(branchb!(
            branchb!(
                branchb!(leaf!(buf, 58), leaf!(buf, 802)),
                branchr!(
                    branchb!(branchr!(leaf!(buf, 896), leaf!(buf, 10)), leaf!(buf, 53)),
                    branchb!(leaf!(buf, 1048), leaf!(buf, 249)),
                ),
            ),
            branchb!(
                branchr!(
                    branchb!(branchr!(leaf!(buf, 927), leaf!(buf, 448)), leaf!(buf, 365)),
                    branchb!(leaf!(buf, 138), leaf!(buf, 269)),
                ),
                branchb!(leaf!(buf, 3), leaf!(buf, 7)),
            ),
        ));
        _ = tree.split(151);
    }

    #[test]
    fn random_tests() {
        let _ = std::fs::remove_dir_all("target/assert/");
        std::fs::create_dir_all("target/assert/").expect("create directory");

        let tree = Tree::empty();
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
                        rope = rope.insert_at(at, block);
                        buf = &buf[written..];
                        at += written;
                    }
                    assert!(rope.is_balanced(), "unbalanced left node; delete middle {}", i);
                    rope
                }
                Operation::Delete { at, len } => {
                    let (updated, deleted) = rope.delete_at(at, len);
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
