mod client;
mod highlighter;
mod language;

pub use client::{Client, Command, Event};
pub use language::Language;

use editor::BufferContents;
use tree_sitter as ts;

#[derive(Debug)]
struct BufferContentsTextProvider<'a>(&'a BufferContents);

impl<'a> BufferContentsTextProvider<'a> {
    fn parse_callback(&self) -> impl Fn(usize, ts::Point) -> &'a [u8] {
        |_byte, pos| -> &[u8] {
            self.0
                .line_at(pos.row + 1)
                .map(|line| {
                    let column = pos.column as usize;
                    if column < line.as_bytes().len() {
                        &line.as_bytes()[column..]
                    } else {
                        b"\n"
                    }
                })
                .unwrap_or(&[])
        }
    }
}

impl<'a> ts::TextProvider<'a> for BufferContentsTextProvider<'a> {
    type I = editor::Lines<'a>;

    fn text(&mut self, node: ts::Node) -> Self::I {
        let range = node.range();
        let start = editor::Point { line: range.start_point.row, column: range.start_point.column };
        let end = editor::Point { line: range.end_point.row, column: range.end_point.column };
        self.0.range(start, end)
    }
}
