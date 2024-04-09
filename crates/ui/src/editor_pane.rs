use editor::{Buffer, Editor};
use ratatui::prelude as tui;
use ratatui::widgets::Widget;

use crate::Theme;

pub struct EditorPane<'a> {
    theme: &'a Theme,
    buffer: &'a Buffer,
    editor: &'a Editor,
}

impl<'a> EditorPane<'a> {
    pub fn new(theme: &'a Theme, buffer: &'a Buffer, editor: &'a Editor) -> Self {
        Self {
            theme,
            buffer,
            editor,
        }
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
}

impl Widget for EditorPane<'_> {
    #[tracing::instrument(skip_all)]
    fn render(self, dims: tui::Rect, buf: &mut tui::Buffer) {
        let offset = self.screen_offset(dims);
        let mut lines = self
            .buffer
            .contents
            .lines(offset.line..(offset.line + dims.height as usize))
            .enumerate();
        let x = dims.left();
        for y in dims.top()..dims.bottom() {
            if let Some((yoffset, line)) = lines.next() {
                for (xoffset, c) in line.into_iter().enumerate() {
                    let x = x + (xoffset as u16); // FIXME: downcast here!
                    let cell = buf.get_mut(x, y);
                    // let char_range = byte_offset..(byte_offset + 1);
                    let start = editor::Point {
                        line: yoffset + y as usize,
                        column: xoffset + x as usize,
                    };
                    let end = editor::Point {
                        line: yoffset + y as usize,
                        column: xoffset + 1 + x as usize,
                    };
                    if let Some((_, name)) = self.buffer.highlights.iter(start..end).next() {
                        if let Some(color) = self.theme.colour(name) {
                            cell.set_fg(color.0);
                        }
                    }
                    cell.set_char(*c as char);
                }
            } else {
                buf.get_mut(x, y).set_char('~');
            }
        }
    }
}
