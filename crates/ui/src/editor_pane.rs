use editor::{Buffer, Editor};
use ratatui::prelude as tui;
use tore::CursorPoint;

use crate::Theme;

pub struct EditorPane<'a> {
    theme: &'a Theme,
    buffer: &'a Buffer,
    editor: &'a Editor,
}

impl<'a> EditorPane<'a> {
    pub fn new(theme: &'a Theme, buffer: &'a Buffer, editor: &'a Editor) -> Self {
        Self { theme, buffer, editor }
    }

    fn screen_offset(&self, dims: tui::Rect) -> editor::Point {
        let cursor = self.editor.cursor;
        let width: usize = dims.width.into();
        let height: usize = dims.height.into();
        let column = if cursor.column >= width {
            cursor.column - width
        } else {
            0
        };
        let line = if cursor.line >= height {
            cursor.line - height
        } else {
            0
        };
        editor::Point { line, column }
    }

    fn offset_cursor(&self, _area: tui::Rect, cursor: tore::Point) -> CursorPoint {
        CursorPoint { x: cursor.column as u16, y: cursor.line as u16 }
    }

    #[tracing::instrument(skip(self, buf))]
    pub fn render(self, dims: tui::Rect, buf: &mut tui::Buffer) -> CursorPoint {
        use bstr::ByteSlice;

        let offset = self.screen_offset(dims);
        let mut lines = self
            .buffer
            .contents
            .lines(offset.line..(offset.line + dims.height as usize));
        let x = dims.left();
        for y in dims.top()..dims.bottom() {
            if let Some(line) = lines.next() {
                let chunk_and_ranges = line.chunk_and_ranges(0);
                let mut xoffset = 0;
                'row_loop: for (chunk, chunk_range) in chunk_and_ranges {
                    let mut byte_offset = chunk_range.start;
                    for c in chunk.chars() {
                        let x = x + xoffset;
                        if x >= dims.width {
                            break 'row_loop;
                        }
                        let char_len = c.len_utf8();
                        let char_range = byte_offset..(byte_offset + char_len);
                        let cell = buf.get_mut(x, y);
                        // cell.set_bg(self.theme.bg().0);
                        if let Some((_, name)) = self.buffer.highlights.iter(char_range).next() {
                            if let Some(color) = self.theme.scheme(name) {
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

        self.offset_cursor(dims, self.editor.cursor)
    }
}
