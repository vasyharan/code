mod block;
mod tree;
pub use self::block::BlockRange;
pub use self::block::Buffer;
use self::tree::Tree;
use bstr::BString;

#[derive(Debug)]
pub enum Error {
    EOS,
}

#[derive(Debug, Clone)]
pub struct Rope {
    root: Tree,
}

impl Rope {
    pub fn empty() -> Self {
        Self { root: Tree::empty() }
    }

    pub fn len(&self) -> usize {
        self.root.len()
    }

    pub fn insert_at(&self, offset: usize, text: BlockRange) -> Result<Self, Error> {
        if text.len() == 0 {
            return Ok(Self { root: self.root.clone() });
        }
        if offset > self.root.len() {
            return Err(Error::EOS);
        }
        let root = self.root.insert_at(offset, text);
        Ok(Self { root })
    }

    pub fn delete_at(&self, offset: usize, len: usize) -> Result<(Self, Self), Error> {
        if offset > self.root.len() {
            return Err(Error::EOS);
        }
        match len.checked_add(offset) {
            None => return Err(Error::EOS),
            Some(x) if x > self.root.len() => return Err(Error::EOS),
            _ => match self.root.delete_at(offset, len) {
                (left, right) => Ok((Self { root: (left) }, Self { root: (right) })),
            },
        }
    }

    fn split(&self, offset: usize) -> Result<(Self, Self), Error> {
        if offset > self.root.len() {
            return Err(Error::EOS);
        }
        match self.root.split(offset) {
            (None, None) => Ok((Self::empty(), Self::empty())),
            (None, Some(right)) => Ok((Self::empty(), Self { root: (right) })),
            (Some(left), None) => Ok((Self { root: (left) }, Self::empty())),
            (Some(left), Some(right)) => Ok((Self { root: (left) }, Self { root: (right) })),
        }
    }

    pub fn is_balanced(&self) -> bool {
        return self.root.is_balanced();
    }

    pub fn to_bstring(&self) -> BString {
        self.root.to_bstring()
    }

    fn write_dot(&self, w: &mut impl std::io::Write) -> std::io::Result<()> {
        self.root.write_dot(w)
    }
}

#[cfg(test)]
mod tests {
    use bstr::ByteSlice;

    use super::*;
    use crate::rope::block::Buffer;

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
        let contents: BString = "This is the song that never ends.\n\
            It just goes 'round and 'round, my friends.\n\
            Some people started singing it\n\
            not knowing what it was;\n\
            and they continue singing it forever just because...\n\
        "
        .into();

        let mut rope = Rope::empty();
        assert!(rope.is_balanced());

        let mut buffer = Buffer::new();
        for (_i, (at, p)) in parts.iter().enumerate() {
            let (block, w) = buffer.append(p.as_bytes()).unwrap();
            assert_eq!(w, p.len());
            rope = rope.insert_at(*at, block).unwrap();

            // let mut file = std::fs::File::create(format!("target/tests/insert{:02}.dot", i))
            //     .expect("create file");
            // rope.write_dot(&mut file).expect("write dot file");

            assert!(rope.is_balanced());
        }
        assert!(rope.is_balanced());
        assert_eq!(rope.to_bstring(), contents);

        for at in 0..rope.len() {
            let (split_left, split_right) = rope.split(at).expect("split rope");

            // let mut file = std::fs::File::create(format!("target/tests/split_left{:02}.dot", at))
            //     .expect("create file");
            // split_left.write_dot(&mut file).expect("write dot file");
            // let mut file = std::fs::File::create(format!("target/tests/split_right{:02}.dot", at))
            //     .expect("create file");
            // split_right.write_dot(&mut file).expect("write dot file");

            assert_eq!(split_left.to_bstring(), contents[..at].as_bstr());
            assert_eq!(split_right.to_bstring(), contents[at..].as_bstr());

            assert!(split_left.is_balanced(), "unbalanced left; split at {}", at);
            assert!(split_right.is_balanced(), "unbalaced right; split at {}", at);
        }

        // delete from start of rope
        (1..=rope.len()).fold(rope.clone(), |rope, i| {
            let (updated, deleted) = rope.delete_at(0, 1).expect("delete rope");

            // let mut file =
            //     std::fs::File::create(format!("target/tests/delete_updated{:02}.dot", i))
            //         .expect("create file");
            // updated.write_dot(&mut file).expect("write dot file");
            // let mut file =
            //     std::fs::File::create(format!("target/tests/delete_deleted{:02}.dot", i))
            //         .expect("create file");
            // deleted.write_dot(&mut file).expect("write dot file");

            assert_eq!(updated.to_bstring(), contents[i..].as_bstr());
            assert_eq!(deleted.to_bstring(), [contents[i - 1]].as_bstr());
            assert!(updated.is_balanced());
            assert!(deleted.is_balanced());
            updated
        });

        // delete from end of string
        (1..=rope.len()).fold(rope.clone(), |rope, i| {
            let (updated, deleted) = rope.delete_at(rope.len() - 1, 1).expect("delete rope");

            // let mut file =
            //     std::fs::File::create(format!("target/tests/delete_updated{:02}.dot", i))
            //         .expect("create file");
            // updated.write_dot(&mut file).expect("write dot file");
            // let mut file =
            //     std::fs::File::create(format!("target/tests/delete_deleted{:02}.dot", i))
            //         .expect("create file");
            // deleted.write_dot(&mut file).expect("write dot file");

            assert_eq!(updated.to_bstring(), contents[..(rope.len() - 1)].as_bstr());
            assert_eq!(deleted.to_bstring(), [contents[rope.len() - 1]].as_bstr());
            assert!(updated.is_balanced(), "unbalanced left node; delete end {}", i);
            assert!(deleted.is_balanced(), "unbalanced right node; delete end {}", i);
            updated
        });

        // delete from middle of string
        (1..=rope.len()).fold(rope.clone(), |rope, i| {
            let middle = rope.len() / 2;
            let (updated, deleted) = rope.delete_at(middle, 1).expect("delete rope");

            // let mut file =
            //     std::fs::File::create(format!("target/tests/delete_updated{:02}.dot", i))
            //         .expect("create file");
            // updated.write_dot(&mut file).expect("write dot file");
            // let mut file =
            //     std::fs::File::create(format!("target/tests/delete_deleted{:02}.dot", i))
            //         .expect("create file");
            // deleted.write_dot(&mut file).expect("write dot file");

            let updated_str = updated.to_bstring();
            assert_eq!(updated_str[..middle].as_bstr(), contents[..middle].as_bstr());
            assert_eq!(updated_str[middle..].as_bstr(), contents[(middle + i)..].as_bstr());
            // assert_eq!(
            //     deleted.to_string(),
            //     String::from_utf8(vec![contents.as_bytes()[middle]]).expect("utf8 string")
            // );
            assert!(updated.is_balanced(), "unbalanced left node; delete middle {}", i);
            assert!(deleted.is_balanced(), "unbalanced right node; delete middle {}", i);
            updated
        });
    }

    #[test]
    fn random_tests() {
        use rand::{distributions::Standard, prelude::Distribution, Rng, RngCore, SeedableRng};
        let rope = Rope::empty();
        let mut bytes = [0; 8192];
        let mut buffer = Buffer::new();
        let mut rng = rand::rngs::SmallRng::from_entropy();

        enum Operation {
            Insert,
            Delete,
        }

        impl Distribution<Operation> for Standard {
            fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> Operation {
                match rng.gen_range(0..2) {
                    0 => Operation::Insert,
                    1 => Operation::Delete,
                    _ => unreachable!(),
                }
            }
        }

        (0..1_000).fold(rope.clone(), |rope, i| {
            let op: Operation = rng.gen();
            if rope.len() == 0 {
                let len = bytes.len();
                let mut buf = &mut bytes[0..(rng.gen_range(0..len))];
                rng.fill_bytes(buf);

                let mut at = 0;
                let mut rope = rope;
                while buf.len() > 0 {
                    let (block, written) = buffer.append(buf).expect("buffer append");
                    buf = &mut buf[0..written];
                    rope = rope.insert_at(at, block).expect("rope insert");
                    at += written;
                }
                return rope;
            }
            match op {
                Operation::Insert => {
                    let mut at = rng.gen_range(0..rope.len());
                    let len = bytes.len();
                    let mut buf = &mut bytes[0..(rng.gen_range(0..len))];
                    rng.fill_bytes(buf);

                    let mut rope = rope;
                    while buf.len() > 0 {
                        let (block, written) = buffer.append(buf).expect("buffer append");
                        buf = &mut buf[0..written];
                        rope = rope.insert_at(at, block).expect("rope insert");
                        at += written;
                    }
                    assert!(rope.is_balanced(), "unbalanced left node; delete middle {}", i);
                    rope
                }
                Operation::Delete => {
                    let at = rng.gen_range(0..rope.len());
                    let len = rng.gen_range(0..(rope.len() - at));
                    let (updated, deleted) = rope.delete_at(at, len).expect("rope delete");
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
