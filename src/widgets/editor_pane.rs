use bstr::ByteSlice;
use ratatui::prelude as tui;
use ratatui::widgets::Widget;

use crate::buffer::Buffer;
use crate::editor::{EditorContext, EditorPosition};
use crate::theme::Theme;

#[derive(Debug)]
pub(crate) struct EditorPane<'a> {
    pub(crate) theme: &'a Theme,
    pub(crate) buffer: &'a Buffer,
    pub(crate) context: &'a EditorContext,
}

impl<'a> EditorPane<'a> {
    pub(crate) fn screen_cursor_position(&self, size: (u16, u16)) -> (u16, u16) {
        let col = std::cmp::min(self.context.cursor_pos.col, size.0.into());
        let row = std::cmp::min(self.context.cursor_pos.row, size.1.into());
        (col as u16, row as u16)
    }

    fn screen_offset(&self, size: (u16, u16)) -> EditorPosition {
        let size = ((size.0 as usize - 1), (size.1 as usize - 1));
        let col = if self.context.cursor_pos.col >= size.0 {
            self.context.cursor_pos.col - size.0
        } else {
            0
        };
        let row = if self.context.cursor_pos.row >= size.1 {
            self.context.cursor_pos.row - size.1
        } else {
            0
        };
        EditorPosition { col, row }
    }
}

impl Widget for EditorPane<'_> {
    #[tracing::instrument(name = "editor_pane::render", skip_all)]
    fn render(self, area: tui::Rect, buf: &mut tui::Buffer) {
        let offset = self.screen_offset((area.width, area.height));
        let mut lines = self
            .buffer
            .contents
            .lines(offset.row..(offset.row + area.height as usize));
        let x = area.left();
        for y in area.top()..area.bottom() {
            if let Some(line) = lines.next() {
                let chunk_and_ranges = line.chunk_and_ranges(..);
                let mut xoffset = 0;
                // TODO: grapheme boundaries
                for (chunk, chunk_range) in chunk_and_ranges {
                    let mut byte_offset = chunk_range.start;
                    for c in chunk.as_bstr().chars() {
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
