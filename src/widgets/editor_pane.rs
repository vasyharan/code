use anyhow::Result;
use bstr::ByteSlice;
use ratatui::prelude as tui;
use ratatui::widgets::Widget;

use crate::buffer::Buffer;
use crate::editor::Editor;
use crate::rope::Rope;
use crate::theme::Theme;

#[derive(Debug)]
pub(crate) struct EditorPane<'ed, 'buf: 'ed, 'app: 'buf> {
    theme: &'app Theme,
    buffer: &'buf Buffer,
    editor: &'ed Editor,
}

pub(crate) struct Position {
    pub(crate) row: u16,
    pub(crate) col: u16,
}

impl Position {
    fn from_byte_offset(rope: &Rope, offset: usize) -> Result<Self> {
        let (line, line_start) = rope.line_at_offset(offset)?;
        let row = line as u16;
        let col = (offset - line_start) as u16;
        Ok(Position { row, col })
    }
}

impl<'ed, 'buf: 'ed, 'app: 'buf> EditorPane<'app, 'buf, 'ed> {
    pub(crate) fn new(theme: &'app Theme, buffer: &'buf Buffer, editor: &'ed Editor) -> Self {
        Self { theme, buffer, editor }
    }

    pub(crate) fn screen_cursor_position(&self, size: (u16, u16)) -> Result<(u16, u16)> {
        let pos =
            Position::from_byte_offset(&self.buffer.contents, self.editor.cursor.byte_offset())?;
        let col = std::cmp::min(pos.col, size.0);
        let row = std::cmp::min(pos.row, size.1);
        Ok((col, row))
    }

    fn screen_offset(&self, size: (u16, u16)) -> Result<Position> {
        let pos =
            Position::from_byte_offset(&self.buffer.contents, self.editor.cursor.byte_offset())?;
        let size = ((size.0 - 1), (size.1 - 1));
        let col = if pos.col >= size.0 {
            pos.col - size.0
        } else {
            0
        };
        let row = if pos.row >= size.1 {
            pos.row - size.1
        } else {
            0
        };
        Ok(Position { col, row })
    }
}

impl Widget for EditorPane<'_, '_, '_> {
    #[tracing::instrument(skip_all)]
    fn render(self, area: tui::Rect, buf: &mut tui::Buffer) {
        let offset = self.screen_offset((area.width, area.height)).unwrap();
        // let buffer = &self.ctx.buffers[self.editor.buffer_id];
        let mut lines = self
            .buffer
            .contents
            .lines((offset.row as usize)..((offset.row + area.height) as usize));
        let x = area.left();
        for y in area.top()..area.bottom() {
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
