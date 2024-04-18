mod client;
mod highlighter;
mod language;

pub use client::{Client, Command, Event};
pub use language::Language;

use editor::BufferContents;
use tree_sitter as ts;
use rope::Rope;

#[derive(Debug)]
struct BufferContentsTextProvider<'a>(&'a BufferContents);

impl<'a> BufferContentsTextProvider<'a> {
    fn parse_callback(&self) -> impl Fn(usize, ts::Point) -> &'a [u8] {
        |byte, _pos| -> &[u8] { self.0.chunks(byte..).next().unwrap_or(&[]) }
    }
}

impl<'a> ts::TextProvider<'a> for BufferContentsTextProvider<'a> {
    type I = rope::Chunks<'a>;

    fn text(&mut self, node: ts::Node) -> Self::I {
        self.0.chunks(node.byte_range())
    }
}
