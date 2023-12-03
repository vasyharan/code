use bstr::ByteSlice;
use iset::IntervalMap;
use ratatui::prelude as tui;
use ratatui::widgets::StatefulWidget;

use crate::buffer::Buffer;
// use crate::rope::Rope;
use crate::theme::Theme;

#[derive(Debug)]
pub(crate) struct EditorPane {
    theme: Theme,
}

impl EditorPane {
    pub(crate) fn new() -> Self {
        Self { theme: Theme::default() }
    }
}

impl StatefulWidget for &EditorPane {
    type State = Buffer;

    #[tracing::instrument(name = "editor_pane::render", skip_all)]
    fn render(self, area: tui::Rect, buf: &mut tui::Buffer, state: &mut Self::State) {
        let mut lines = state.contents.lines();
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
                        if let Some((_, name)) = state.highlights.iter(char_range).next() {
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
