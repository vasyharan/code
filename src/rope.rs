use std::rc::Rc;

#[derive(Debug)]
enum Error {
    EOS,
}

#[derive(Debug)]
enum RedBlackTreeError {
    ConsecutiveRed,
    DifferingBlackHeight,
}

// #[derive(Debug, Clone, Copy, PartialEq)]
// enum Position {
//     ByteOffset(usize),
//     // LineAndColumn((u32, u32)),
// }

#[derive(Debug, Clone, Copy, PartialEq)]
enum NodeColour {
    Red,
    Black,
}

impl NodeColour {
    fn black_height(&self) -> u8 {
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
        val: String,
        len: usize,
    },
}

use Node::{Branch, Leaf};

impl Node {
    fn new_branch(colour: NodeColour, left: Rc<Node>, right: Rc<Node>) -> Self {
        let len = left.len() + right.len();
        Branch { colour, left, right, len }
    }

    fn new_leaf(val: String) -> Self {
        let len = val.len();
        Leaf { val, len }
    }

    fn len(&self) -> usize {
        match &self {
            Branch { len, .. } => *len,
            Leaf { len, .. } => *len,
        }
    }

    fn colour(&self) -> NodeColour {
        match &self {
            Branch { colour, .. } => *colour,
            Leaf { .. } => NodeColour::Black,
        }
    }

    fn is_balanced(&self) -> Result<usize, RedBlackTreeError> {
        match &self {
            Node::Leaf { .. } => Ok(0),
            Node::Branch { colour, left, right, .. } => {
                if *colour == NodeColour::Red {
                    if let Branch { colour: NodeColour::Red, .. } = left.as_ref() {
                        return Err(RedBlackTreeError::ConsecutiveRed);
                    }
                    if let Branch { colour: NodeColour::Red, .. } = right.as_ref() {
                        return Err(RedBlackTreeError::ConsecutiveRed);
                    }
                }

                let lheight = &left.is_balanced()?;
                let rheight = &right.is_balanced()?;
                if lheight != rheight {
                    return Err(RedBlackTreeError::DifferingBlackHeight);
                }
                Ok(lheight + (colour.black_height() as usize))
            }
        }
    }

    fn write_dot(&self, w: &mut impl std::io::Write) -> std::io::Result<()> {
        match self {
            Branch { colour, left, right, .. } => {
                write!(
                    w,
                    "\tn{:p}[shape=circle,color={},label=\"\"];\n",
                    self, colour
                )?;

                left.write_dot(w)?;
                write!(
                    w,
                    "\tn{:p} -> n{:p}[label=\"{}\"];\n",
                    self,
                    left.as_ref(),
                    left.len()
                )?;

                right.write_dot(w)?;
                write!(
                    w,
                    "\tn{:p} -> n{:p}[label=\"{}\"];\n",
                    self,
                    right.as_ref(),
                    right.len()
                )?;
            }
            Leaf { val, .. } => {
                write!(w, "\tn{:p}[shape=square,label=\"'{}'\"];\n", self, val)?;
            }
        }
        Ok(())
    }
}

impl ToString for Node {
    fn to_string(&self) -> String {
        match self {
            Leaf { val, .. } => val.clone(),
            Branch { left, right, .. } => {
                let mut s = left.to_string();
                s.push_str(&right.to_string());
                s
            }
        }
    }
}

#[derive(Debug, Clone)]
struct Rope {
    root: Rc<Node>,
}

impl Rope {
    fn empty() -> Self {
        let root = Rc::new(Node::new_leaf("".to_string()));
        Self { root }
    }

    fn len(&self) -> usize {
        self.root.len()
    }

    fn insert_at(&self, offset: usize, text: String) -> Result<Self, Error> {
        if text.len() == 0 {
            return Ok(Self { root: self.root.clone() });
        }
        if offset > self.root.len() {
            return Err(Error::EOS);
        }
        let root = insert(&self.root, offset, text);
        let root = make_black(Rc::new(root));
        Ok(Self { root })
    }

    fn delete_at(&self, offset: usize, len: usize) -> Result<(Self, Self), Error> {
        if offset > self.root.len() || len + offset > self.root.len() {
            return Err(Error::EOS);
        }
        if len == 0 {
            let deleted = Self::empty();
            let updated = Self { root: self.root.clone() };
            return Ok((updated, deleted));
        }

        let (left, right, _) = split(&self.root, offset);
        let (deleted, right, _) = split(&right.unwrap().0, len);
        let updated = match join(left, right) {
            None => Self::empty(),
            Some((root, _)) => Self { root: make_black(root) },
        };
        let deleted = match deleted {
            None => Self::empty(),
            Some((root, _)) => Self { root: make_black(root) },
        };
        Ok((updated, deleted))
    }

    fn split(&self, offset: usize) -> Result<(Self, Self), Error> {
        if offset > self.root.len() {
            return Err(Error::EOS);
        }
        match split(&self.root, offset) {
            (Some((left, _)), Some((right, _)), _) => Ok((
                Self { root: make_black(left) },
                Self { root: make_black(right) },
            )),
            (None, Some((right, _)), _) => Ok((Self::empty(), Self { root: make_black(right) })),
            (Some((left, _)), None, _) => Ok((Self { root: make_black(left) }, Self::empty())),
            (None, None, _) => Ok((Self::empty(), Self::empty())),
        }
    }

    fn is_balanced(&self) -> bool {
        match &self.root.is_balanced() {
            Ok(_) => true,
            _ => false,
        }
    }

    fn write_dot(&self, w: &mut impl std::io::Write) -> std::io::Result<()> {
        write!(w, "digraph {{\n")?;
        self.root.write_dot(w)?;
        write!(w, "}}")?;
        Ok(())
    }
}

impl ToString for Rope {
    fn to_string(&self) -> String {
        self.root.to_string()
    }
}

fn make_black(node: Rc<Node>) -> Rc<Node> {
    match node.as_ref() {
        Branch { colour: NodeColour::Red, left, right, len, .. } => {
            let node = Branch {
                colour: NodeColour::Black,
                left: left.clone(),
                right: right.clone(),
                len: *len,
            };
            Rc::new(node)
        }
        _ => node,
    }
}
fn insert(node: &Rc<Node>, offset: usize, text: String) -> Node {
    match node.as_ref() {
        Leaf { val, .. } => {
            if node.len() == 0 {
                Node::new_leaf(text)
            } else if offset == 0 {
                let left = Rc::new(Node::new_leaf(text));
                Node::new_branch(NodeColour::Red, left, node.clone())
            } else if offset == node.len() {
                let right = Rc::new(Node::new_leaf(text));
                Node::new_branch(NodeColour::Red, node.clone(), right)
            } else {
                let left = Rc::new(Node::new_leaf(val[..offset].to_string()));
                let rl = Rc::new(Node::new_leaf(text));
                let rr = Rc::new(Node::new_leaf(val[offset..].to_string()));
                let right = Rc::new(Node::new_branch(NodeColour::Red, rl, rr));
                Node::new_branch(NodeColour::Red, left, right)
            }
        }
        Branch { colour, left, right, .. } => {
            let left_len = left.len();
            if left_len > offset {
                let left = insert(left, offset, text);
                let (node, _) = balance(*colour, Rc::new(left), right.clone());
                node
            } else {
                let offset = offset - left_len;
                let right = insert(right, offset, text);
                let (node, _) = balance(*colour, left.clone(), Rc::new(right));
                node
            }
        }
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

fn join_left(left: (Rc<Node>, usize), right: (Rc<Node>, usize)) -> (Rc<Node>, usize) {
    let (left, lheight) = left;
    let (right, rheight) = right;
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

fn join(
    maybe_left: Option<(Rc<Node>, usize)>,
    maybe_right: Option<(Rc<Node>, usize)>,
) -> Option<(Rc<Node>, usize)> {
    match (maybe_left, maybe_right) {
        (None, None) => None,
        (None, Some(right)) => Some(right.clone()),
        (Some(left), None) => Some(left.clone()),
        (Some((left, lheight)), Some((right, rheight))) => {
            // let lheight = black_height(&left);
            // let rheight = black_height(&right);
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
                // let colour = NodeColour::Black;
                let node = Node::new_branch(colour, left.clone(), right.clone());
                // let (node, _) = balance(colour, left.clone(), right.clone());
                Some((Rc::new(node), lheight + (colour.black_height() as usize)))
            }
        }
    }
}

fn split(node: &Node, at: usize) -> (Option<(Rc<Node>, usize)>, Option<(Rc<Node>, usize)>, usize) {
    match node {
        Node::Leaf { val, .. } => {
            // TODO: stop making copies if possible
            let split_left = if at == 0 {
                None
            } else {
                Some((Rc::new(Node::new_leaf(val[..at].to_string())), 0))
            };
            let split_right = if at == val.len() {
                None
            } else {
                Some((Rc::new(Node::new_leaf(val[at..].to_string())), 0))
            };
            (split_left, split_right, 0)
        }
        Node::Branch { colour, left, right, .. } => {
            if at <= left.len() {
                let (split_left, split_right, lheight) = split(left, at);
                let join_right = Some((right.clone(), lheight));
                let split_right = join(split_right, join_right);
                let height = lheight + (colour.black_height() as usize);
                (split_left, split_right, height)
            } else {
                let (split_left, split_right, rheight) = split(right, at - left.len());
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
        Node::Leaf { .. } => 0,
        Node::Branch { colour, left, right, .. } => {
            let lheight = black_height(left);
            let rheight = black_height(right);
            if lheight != rheight {
                panic!("unbalanced")
            }
            lheight + (colour.black_height() as usize)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
        let contents = "This is the song that never ends.\n\
            It just goes 'round and 'round, my friends.\n\
            Some people started singing it\n\
            not knowing what it was;\n\
            and they continue singing it forever just because...\n\
        ";

        let mut rope = Rope::empty();
        assert!(rope.is_balanced());

        for (i, (at, p)) in parts.iter().enumerate() {
            rope = rope.insert_at(*at, p.to_string()).unwrap();

            let mut file = std::fs::File::create(format!("target/tests/insert{:02}.dot", i))
                .expect("create file");
            rope.write_dot(&mut file).expect("write dot file");
            assert!(
                rope.is_balanced(),
                "unbalanced when inserting {:?} at {}",
                p,
                at
            );
        }
        assert!(rope.is_balanced());
        assert_eq!(rope.to_string(), contents);

        for at in 0..rope.len() {
            let (split_left, split_right) = rope.split(at).expect("split rope");

            let mut file = std::fs::File::create(format!("target/tests/split_left{:02}.dot", at))
                .expect("create file");
            split_left.write_dot(&mut file).expect("write dot file");
            let mut file = std::fs::File::create(format!("target/tests/split_right{:02}.dot", at))
                .expect("create file");
            split_right.write_dot(&mut file).expect("write dot file");

            assert_eq!(split_left.to_string(), contents[..at]);
            assert_eq!(split_right.to_string(), contents[at..]);

            assert!(split_left.is_balanced());
            assert!(split_right.is_balanced());
        }

        (1..=rope.len()).fold(rope.clone(), |rope, i| {
            let (updated, deleted) = rope.delete_at(0, 1).expect("delete rope");

            let mut file =
                std::fs::File::create(format!("target/tests/delete_updated{:02}.dot", i))
                    .expect("create file");
            updated.write_dot(&mut file).expect("write dot file");
            let mut file =
                std::fs::File::create(format!("target/tests/delete_deleted{:02}.dot", i))
                    .expect("create file");
            deleted.write_dot(&mut file).expect("write dot file");

            assert_eq!(updated.to_string(), contents[i..]);
            assert_eq!(deleted.to_string().as_bytes(), [contents.as_bytes()[i - 1]]);
            assert!(updated.is_balanced());
            assert!(deleted.is_balanced());
            updated
        });

        (1..=rope.len()).fold(rope.clone(), |rope, i| {
            let (updated, deleted) = rope.delete_at(rope.len() - 1, 1).expect("delete rope");

            let mut file =
                std::fs::File::create(format!("target/tests/delete_updated{:02}.dot", i))
                    .expect("create file");
            updated.write_dot(&mut file).expect("write dot file");
            let mut file =
                std::fs::File::create(format!("target/tests/delete_deleted{:02}.dot", i))
                    .expect("create file");
            deleted.write_dot(&mut file).expect("write dot file");

            assert_eq!(updated.to_string(), contents[..(rope.len() - 1)]);
            assert_eq!(
                deleted.to_string(),
                String::from_utf8(vec![contents.as_bytes()[rope.len() - 1]]).expect("utf8 string")
            );
            assert!(updated.is_balanced());
            assert!(deleted.is_balanced());
            updated
        });

        // rope = rope.delete_at(Position::ByteOffset(2), 2).unwrap();
        // let mut file = std::fs::File::create("target/tests/delete00.dot").expect("create file");
        // rope.write_dot(&mut file).expect("write dot file");
        // assert_eq!(rope.to_string(), "Lom ipsum dolor sit amet consectetur adipiscing elit sed do eiusmod tempor incididunt ut labore et dolore magna aliqua.");
        // assert!(rope.is_balanced());

        // rope = rope.delete_at(Position::ByteOffset(0), 1).unwrap();
        // let mut file = std::fs::File::create("target/tests/delete01.dot").expect("create file");
        // rope.write_dot(&mut file).expect("write dot file");
        // assert_eq!(rope.to_string(), "om ipsum dolor sit amet consectetur adipiscing elit sed do eiusmod tempor incididunt ut labore et dolore magna aliqua.");
        // assert!(rope.is_balanced());

        // rope = rope.delete_at(Position::ByteOffset(2), 1).unwrap();
        // let mut file = std::fs::File::create("target/tests/delete02.dot").expect("create file");
        // rope.write_dot(&mut file).expect("write dot file");
        // assert_eq!(rope.to_string(), "omipsum dolor sit amet consectetur adipiscing elit sed do eiusmod tempor incididunt ut labore et dolore magna aliqua.");
        // assert!(rope.is_balanced());

        // rope = rope.delete_at(Position::ByteOffset(10), 22).unwrap();
        // let mut file = std::fs::File::create("target/tests/delete03.dot").expect("create file");
        // rope.write_dot(&mut file).expect("write dot file");
        // assert_eq!(rope.to_string(), "omipsum dour adipiscing elit sed do eiusmod tempor incididunt ut labore et dolore magna aliqua.");
        // assert!(rope.is_balanced());
    }
}
