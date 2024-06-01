use bstr::*;
use ratatui::prelude as tui;

use commands::Commands;
use tore::CursorPoint;

use crate::Theme;

const BORDERS: bool = false;

#[derive(Debug)]
pub struct CommandsPane<'a, T> {
    theme: &'a Theme,
    commands: &'a Commands<T>,
}

impl<'a, T> CommandsPane<'a, T> {
    pub fn new(theme: &'a Theme, commands: &'a Commands<T>) -> Self {
        Self { theme, commands }
    }

    fn layout_command_pane(
        dims: tui::Rect,
        num_results: usize,
    ) -> (tui::Rect, tui::Rect, tui::Rect) {
        const WIDTH_RATIO: u16 = 4;
        let y = 0; // dims.height / 5;
        let x0 = dims.width / WIDTH_RATIO;
        let x1 = x0 * (WIDTH_RATIO - 1);
        let xwidth = std::cmp::min(x1 - x0, 80);
        let rheight = std::cmp::min(13, num_results) as u16;

        let qborder = tui::Rect::new(x0, y, xwidth, 3 + rheight);
        let qcontent = tui::Rect::new(x0 + 1, y + 1, xwidth - 2, 1);
        let rcontent = tui::Rect::new(x0 + 1, y + 2, xwidth - 2, rheight);
        (qborder, qcontent, rcontent)
    }

    fn offset_cursor(&self, cursor: tore::Point, area: tui::Rect) -> CursorPoint {
        let x = area.left() + 2 + cursor.column as u16;
        let y = area.top();
        CursorPoint { x, y }
    }

    #[tracing::instrument(skip(self, buf))]
    pub fn render(self, dims: tui::Rect, buf: &mut tui::Buffer) -> CursorPoint {
        use ratatui::widgets::{Block, BorderType, Borders, Widget};

        let results = self.commands.results();
        let (qborder, qcontent, rcontent) = Self::layout_command_pane(dims, results.len());

        let bg0 = self.theme.palette("bg0").unwrap().0;
        let bg1 = self.theme.palette("bg1").unwrap().0;
        let fg0 = self.theme.palette("fg0").unwrap().0;
        let fg1 = self.theme.palette("yellow").unwrap().0;
        let border_style = tui::Style::reset().fg(fg0).bg(bg0);
        let content_style = border_style;

        reset_border(buf, qborder);
        Block::new()
            .borders(Borders::LEFT | Borders::RIGHT | Borders::TOP | Borders::BOTTOM)
            .border_type(BorderType::Plain)
            .border_style(border_style)
            .render(qborder, buf);

        let mut qcontent_range = qcontent.left()..qcontent.right();
        buf.get_mut(qcontent_range.next().expect("area must not be empty"), qcontent.y)
            .set_style(content_style)
            .set_char(':');
        buf.get_mut(qcontent_range.next().expect("area must not be empty"), qcontent.y)
            .set_style(content_style)
            .set_char(' ');

        let mut query = self.commands.query.chars();
        for x in qcontent_range {
            let c = query.next().unwrap_or(' ');
            buf.get_mut(x, qcontent.y)
                .set_style(content_style)
                .set_char(c);
        }

        let mut results = results.into_iter();
        for y in (rcontent.top()..rcontent.bottom()) {
            let result = results.next();
            let (content_prefix, bg) = match (self.commands.selected, &result) {
                (None, Some(_)) => unreachable!("something must be selected if results exist"),
                (Some(_), None) => unreachable!("selected entry must exist if results exist"),
                (Some(selected), Some(result)) => {
                    if selected == result.entry.id {
                        ("> ", bg1)
                    } else {
                        ("  ", bg0)
                    }
                }
                (_, None) => ("  ", bg0),
            };

            let content = result
                .as_ref()
                .map(|r| format!("{}{}", content_prefix, r.entry.command))
                .unwrap_or("".to_string());
            let mut indices = result
                .map(|r| r.indices)
                .unwrap_or(vec![])
                .into_iter()
                .peekable();
            let mut graphemes = content.as_bytes().as_bstr().graphemes();

            let maxlen = rcontent.width as usize;
            for (idx, x) in (rcontent.left()..rcontent.right()).enumerate() {
                let symbol = graphemes.next().unwrap_or(" ");
                let next_idx = *indices.peek().unwrap_or(&maxlen);
                let fg = if next_idx + 2 == idx {
                    indices.next();
                    fg1
                } else {
                    fg0
                };
                buf.get_mut(x, y)
                    .set_style(content_style)
                    .set_bg(bg)
                    .set_fg(fg)
                    .set_symbol(symbol);
            }
        }

        self.offset_cursor(self.commands.cursor, qcontent)
    }
}

fn reset_border(buf: &mut tui::Buffer, area: tui::Rect) {
    for y in area.top()..area.bottom() {
        buf.get_mut(area.left(), y).reset();
    }
    for x in area.left()..area.right() {
        buf.get_mut(x, area.top()).reset();
    }
    let x = area.right() - 1;
    for y in area.top()..area.bottom() {
        buf.get_mut(x, y).reset();
    }
    let y = area.bottom() - 1;
    for x in area.left()..area.right() {
        buf.get_mut(x, y).reset();
    }
}
