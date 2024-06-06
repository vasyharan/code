use crossterm::cursor::SetCursorStyle;
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
    pub fn render(self, dims: tui::Rect, buf: &mut tui::Buffer) -> (CursorPoint, SetCursorStyle) {
        use bstr::ByteSlice;

        let offset = self.screen_offset(dims);
        let mut lines = self.buffer.contents.lines_at(offset.line);
        let x = dims.left();
        for (yoffset, y) in (dims.top()..dims.bottom()).enumerate() {
            if let Some(line) = lines.next() {
                let line_offset = self.buffer.contents.line_to_byte(offset.line + yoffset);
                let mut xoffset = 0;
                'row_loop: for chunk in line.chunks() {
                    for (start, end, grapheme) in chunk.as_bytes().as_bstr().grapheme_indices() {
                        if x + xoffset >= dims.width || grapheme == "\n" {
                            break 'row_loop;
                        }

                        let cell = buf.get_mut(x + xoffset, y);
                        let char_range = line_offset + start..line_offset + end;
                        if let Some((_, name)) = self.buffer.highlights.iter(char_range).next() {
                            if let Some(color) = self.theme.scheme(name) {
                                cell.set_fg(color.0);
                            }
                        }

                        cell.set_symbol(grapheme);
                        xoffset += 1;
                    }
                }
            } else {
                buf.get_mut(x, y).set_char('~');
            }
        }

        let cursor_pos = self.offset_cursor(dims, self.editor.cursor);
        let cursor_style = match self.editor.mode {
            editor::Mode::Normal => SetCursorStyle::BlinkingBlock,
            editor::Mode::Insert => SetCursorStyle::BlinkingBar,
        };
        (cursor_pos, cursor_style)
    }
}
