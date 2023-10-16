#![no_main]

extern crate toku;
use arbitrary::Arbitrary;
use libfuzzer_sys::fuzz_target;
use toku::rope::Rope;

#[derive(Arbitrary, Clone, Debug)]
pub enum Operation {
    Insert { at: usize, contents: String },
    Delete { at: usize, len: usize },
}

fuzz_target!(|ops: Vec<Operation>| {
    use Operation::{Delete, Insert};
    ops.into_iter().fold(Rope::empty(), |rope, op| match op {
        Insert { at, contents } => {
            let rope = rope.insert_at(at, contents).unwrap_or(rope);
            assert!(rope.is_balanced());
            rope
        }
        Delete { at, len } => {
            let (updated, deleted) = rope.delete_at(at, len).unwrap_or((rope, Rope::empty()));
            assert!(updated.is_balanced());
            assert!(deleted.is_balanced());
            updated
        }
    });
});
