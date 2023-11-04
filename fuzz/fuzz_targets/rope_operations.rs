#![no_main]

extern crate toku;
use arbitrary::Arbitrary;
use libfuzzer_sys::fuzz_target;
use toku::rope::BlockBuffer;
use toku::rope::Rope;

#[derive(Arbitrary, Clone, Debug)]
pub enum Operation {
    Insert { at: usize, contents: Vec<u8> },
    Delete { at: usize, len: usize },
}

fuzz_target!(|ops: Vec<Operation>| {
    use Operation::{Delete, Insert};
    let mut buffer = BlockBuffer::new();
    ops.into_iter().fold(Rope::empty(), |rope, op| {
        println!("{:?}", op);
        match op {
            Insert { at, contents } => {
                let (block, written) = buffer.append(&contents).unwrap();
                assert_eq!(written, contents.len());
                let rope = rope.insert_at(at, block).unwrap_or(rope);
                assert!(rope.is_balanced());
                rope
            }
            Delete { at, len } => {
                let (updated, deleted) = rope.delete_at(at, len).unwrap_or((rope, Rope::empty()));
                assert!(updated.is_balanced());
                assert!(deleted.is_balanced());
                updated
            }
        }
    });
});
