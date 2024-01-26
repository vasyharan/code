use anyhow::Result;
use bstr::ByteSlice;
use ratatui::prelude as tui;
use ratatui::widgets::Widget;

use crate::buffer::Buffer;
use crate::editor::Editor;
use crate::rope;
use crate::theme::Theme;

use super::{Dimensions, Point};

#[derive(Debug)]
pub(crate) struct EditorPane<'ed, 'buf: 'ed, 'app: 'buf> {
    theme: &'app Theme,
    buffer: &'buf Buffer,
    editor: &'ed Editor,
}

impl<'ed, 'buf: 'ed, 'app: 'buf> EditorPane<'app, 'buf, 'ed> {
    pub(crate) fn new(theme: &'app Theme, buffer: &'buf Buffer, editor: &'ed Editor) -> Self {
        Self { theme, buffer, editor }
    }

    pub(crate) fn screen_cursor_position(&self, dims: Dimensions) -> Result<Point> {
        let point = self.editor.cursor.point();
        // TODO: fix these casts!
        let column = std::cmp::min(point.column, dims.width as usize) as u16 - 1;
        let row = std::cmp::min(point.line, dims.height as usize) as u16 - 1;
        Ok(Point { column, row })
    }

    fn screen_offset(&self, dims: Dimensions) -> Result<Point> {
        let point = self.editor.cursor.point();
        // TODO: fix these casts!
        let width = dims.width as usize;
        let column = if point.column >= width {
            (point.column - width) as u16
        } else {
            0
        };
        let height = dims.height as usize;
        let row = if point.line >= height {
            (point.line - height) as u16
        } else {
            0
        };
        Ok(Point { row, column })
    }
}

impl Widget for EditorPane<'_, '_, '_> {
    #[tracing::instrument(skip_all)]
    fn render(self, dims: tui::Rect, buf: &mut tui::Buffer) {
        let offset = self.screen_offset(dims).unwrap();
        // let buffer = &self.ctx.buffers[self.editor.buffer_id];
        let mut lines = self
            .buffer
            .contents
            .lines((offset.row as usize)..((offset.row + dims.height) as usize));
        let x = dims.left();
        for y in dims.top()..dims.bottom() {
            if let Some(line) = lines.next() {
                let chunk_and_ranges = line.chunk_and_ranges(..);
                let mut xoffset = 0;
                // TODO: grapheme boundaries
                for (chunk, chunk_range) in chunk_and_ranges {
                    let mut byte_offset = chunk_range.start;
                    for c in chunk.chars() {
                        let x = x + xoffset;
                        let char_len = c.len_utf8();
                        let char_range = byte_offset..(byte_offset + char_len);
                        let cell = buf.get_mut(x, y);
                        // cell.set_bg(self.theme.bg().0);
                        if let Some((_, name)) = self.buffer.highlights.iter(char_range).next() {
                            if let Some(color) = self.theme.colour(name) {
                                cell.set_fg(color.0);
                            }
                        }
                        cell.set_char(c);

                        xoffset += 1; // TODO: this should check wcwidth
                        byte_offset += char_len;
                    }
                }
            } else {
                buf.get_mut(x, y).set_char('~');
            }
        }
    }
}
