use editor::{Buffer, Editor};
use ratatui::prelude as tui;
use ratatui::widgets::Widget;

pub struct EditorPane<'a> {
    buffer: &'a Buffer,
    editor: &'a Editor,
}

impl<'a> EditorPane<'a> {
    pub fn new(buffer: &'a Buffer, editor: &'a Editor) -> Self {
        Self { buffer, editor }
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
            .lines(offset.line..(offset.line + dims.height as usize));
        let x = dims.left();
        for y in dims.top()..dims.bottom() {
            if let Some(line) = lines.next() {
                for (xoffset, c) in line.into_iter().enumerate() {
                    let x = x + (xoffset as u16); // FIXME: downcast here!
                    let cell = buf.get_mut(x, y);
                    cell.set_char(*c as char);
                }
            } else {
                buf.get_mut(x, y).set_char('~');
            }
        }
    }
}
