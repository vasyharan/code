pub mod block;
mod tree;
use bstr::BString;
use tree::Tree;

use self::block::BlockRange;

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
    use bstr::BStr;

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
        for (i, (at, p)) in parts.iter().enumerate() {
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

            assert_eq!(split_left.to_bstring(), BString::from(&contents[..at]));
            assert_eq!(split_right.to_bstring(), BString::from(&contents[at..]));

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

            assert_eq!(updated.to_bstring(), BString::from(&contents[i..]));
            assert_eq!(deleted.to_bstring(), BString::from([contents[i - 1]]));
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

            assert_eq!(updated.to_bstring(), BString::from(&contents[..(rope.len() - 1)]));
            assert_eq!(deleted.to_bstring(), BString::from([contents[rope.len() - 1]]));
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
            assert_eq!(updated_str[..middle], BString::from(&contents[..middle]));
            assert_eq!(updated_str[middle..], BString::from(&contents[(middle + i)..]));
            // assert_eq!(
            //     deleted.to_string(),
            //     String::from_utf8(vec![contents.as_bytes()[middle]]).expect("utf8 string")
            // );
            assert!(updated.is_balanced(), "unbalanced left node; delete middle {}", i);
            assert!(deleted.is_balanced(), "unbalanced right node; delete middle {}", i);
            updated
        });
    }
}
