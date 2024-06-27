mod client;
mod highlighter;
mod language;

pub use client::{Command, Event, Syntax};
pub use language::Language;

use editor::BufferContents;
use rope::iter::Chunks;
use tree_sitter as ts;

#[derive(Debug)]
struct BufferContentsTextProvider<'a>(&'a BufferContents);

impl<'a> BufferContentsTextProvider<'a> {
    fn parse_callback(&self) -> impl Fn(usize, ts::Point) -> &'a [u8] {
        |byte_offset, _pos| -> &[u8] {
            let chunk = self.0.get_chunk_at_byte(byte_offset);
            chunk.map_or(&[], |(chunk, byte_idx, ..)| {
                let chunk_offset = byte_offset - byte_idx;
                &chunk.as_bytes()[chunk_offset..]
            })
        }
    }
}

struct ByteSliceChunks<'a>(Chunks<'a>);

impl<'a> Iterator for ByteSliceChunks<'a> {
    type Item = &'a [u8];

    fn next(&mut self) -> Option<Self::Item> {
        self.0.next().map(|chunk| chunk.as_bytes())
    }
}

impl<'a> ts::TextProvider<'a> for BufferContentsTextProvider<'a> {
    type I = ByteSliceChunks<'a>;

    fn text(&mut self, node: ts::Node) -> Self::I {
        let text = self.0.byte_slice(node.byte_range());
        let chunks = text.chunks();
        ByteSliceChunks(chunks)
    }
}
