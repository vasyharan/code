use bstr::ByteSlice;
use ratatui::prelude as tui;
use ratatui::widgets::Widget;

use crate::app::AppContext;

#[derive(Debug)]
pub(crate) struct EditorPane<'a> {
    context: &'a AppContext,
}

impl<'a> EditorPane<'a> {
    pub(crate) fn new(context: &'a AppContext) -> Self {
        Self { context }
    }
}

impl Widget for EditorPane<'_> {
    #[tracing::instrument(name = "editor_pane::render", skip_all)]
    fn render(self, area: tui::Rect, buf: &mut tui::Buffer) {
        let buffer = &self.context.buffer;
        let theme = &self.context.theme;
        let mut lines = buffer.contents.lines();
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
                        if let Some((_, name)) = buffer.highlights.iter(char_range).next() {
                            if let Some(color) = theme.colour(name) {
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
