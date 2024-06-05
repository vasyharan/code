use bstr::*;
use ratatui::prelude as tui;

use commands::Commands;
use tore::CursorPoint;

use crate::theme::Color;

#[derive(Debug)]
pub struct Theme {
    bg: Color,
    fg: Color,
    bg_selected: Color,
    fg_highlight: Color,
}

#[derive(Debug)]
pub struct CommandsPane<'a, T> {
    theme: Theme,
    commands: &'a Commands<T>,
}

const QUERY_PREFIX: &str = ":";

impl<'a, T> CommandsPane<'a, T> {
    pub fn new(theme: &crate::Theme, commands: &'a Commands<T>) -> Self {
        let bg = theme.palette("bg0").unwrap();
        let bg_selected = theme.palette("bg1").unwrap();
        let fg = theme.palette("fg0").unwrap();
        let fg_highlight = theme.palette("yellow").unwrap();
        let theme = Theme { bg, fg, bg_selected, fg_highlight };
        Self { theme, commands }
    }

    fn layout_command_pane(
        dims: tui::Rect,
        num_results: usize,
    ) -> (tui::Rect, tui::Rect, tui::Rect) {
        use std::cmp::{max, min};

        const WIDTH_RATIO: u16 = 4;
        let y = 0; // dims.height / 5;
        let x0 = dims.width / WIDTH_RATIO;
        let x1 = x0 * (WIDTH_RATIO - 1);
        let xwidth = min(x1 - x0, 80);
        let rheight = min(13, max(1, num_results)) as u16;

        let qborder = tui::Rect::new(x0, y, xwidth, 3 + rheight);
        let qcontent = tui::Rect::new(x0 + 1, y + 1, xwidth - 2, 1);
        let rcontent = tui::Rect::new(x0 + 1, y + 2, xwidth - 2, rheight);
        (qborder, qcontent, rcontent)
    }

    fn offset_cursor(&self, cursor: tore::Point, area: tui::Rect) -> CursorPoint {
        let x = area.left() + (QUERY_PREFIX.len() as u16) + cursor.column as u16;
        let y = area.top();
        CursorPoint { x, y }
    }

    fn render_border(&self, buf: &mut tui::Buffer, area: tui::Rect) {
        use ratatui::widgets::{Block, BorderType, Borders, Widget};

        let style = tui::Style::reset()
            .fg(self.theme.fg.into())
            .bg(self.theme.bg.into());

        reset_border(buf, area);
        Block::new()
            .borders(Borders::LEFT | Borders::RIGHT | Borders::TOP | Borders::BOTTOM)
            .border_type(BorderType::Plain)
            .border_style(style)
            .render(area, buf);
    }

    fn render_query(&self, buf: &mut tui::Buffer, area: tui::Rect) {
        let range = area.left()..area.right();
        let style = tui::Style::reset()
            .fg(self.theme.fg.into())
            .bg(self.theme.bg.into());

        let query_prefix = QUERY_PREFIX.as_bytes().as_bstr().graphemes();
        let query = self.commands.query.as_bytes().as_bstr().graphemes();
        let mut query = query_prefix.chain(query);
        for x in range {
            let c = query.next().unwrap_or(" ");
            buf.get_mut(x, area.y).set_style(style).set_symbol(c);
        }
    }

    fn render_results(
        &self,
        buf: &mut tui::Buffer,
        area: tui::Rect,
        results: Vec<commands::ResultEntry<T>>,
    ) {
        let style = tui::Style::reset()
            .fg(self.theme.fg.into())
            .bg(self.theme.bg.into());

        let has_results = !results.is_empty();
        let mut results = results.into_iter();
        if !has_results {
            let mut graphemes = " No matches".as_bytes().as_bstr().graphemes();
            for x in area.left()..area.right() {
                let symbol = graphemes.next().unwrap_or(" ");
                buf.get_mut(x, area.top())
                    .set_style(style.add_modifier(tui::Modifier::ITALIC))
                    .set_symbol(symbol);
            }
        } else {
            for y in area.top()..area.bottom() {
                let result = results.next();
                let (content_prefix, bg) = match (self.commands.selected, &result) {
                    (None, Some(_)) => unreachable!("something must be selected if results exist"),
                    (Some(_), None) => unreachable!("selected entry must exist if results exist"),
                    (Some(selected), Some(result)) => {
                        if selected == result.entry.id {
                            ("", self.theme.bg_selected)
                        } else {
                            (" ", self.theme.bg)
                        }
                    }
                    (_, None) => (" ", self.theme.bg),
                };

                let content = result
                    .as_ref()
                    .map(|r| format!("{}{}", content_prefix, r.entry.name))
                    .unwrap_or("".to_string());
                let mut indices = result
                    .map(|r| r.indices)
                    .unwrap_or(vec![])
                    .into_iter()
                    .peekable();
                let maxlen = area.width as usize;
                let mut graphemes = content.as_bytes().as_bstr().graphemes();
                for (idx, x) in (area.left()..area.right()).enumerate() {
                    let symbol = graphemes.next().unwrap_or(" ");
                    let next_idx = *indices.peek().unwrap_or(&maxlen);
                    let fg = if idx == 0 {
                        self.theme.fg_highlight
                    } else if next_idx + 1 == idx {
                        indices.next();
                        self.theme.fg_highlight
                    } else {
                        self.theme.fg
                    };
                    let style = tui::Style::reset().fg(fg.into()).bg(bg.into());
                    buf.get_mut(x, y).set_style(style).set_symbol(symbol);
                }
            }
        }
    }

    #[tracing::instrument(skip(self, buf))]
    pub fn render(self, dims: tui::Rect, buf: &mut tui::Buffer) -> CursorPoint {
        let results = self.commands.query_results();
        let (qborder, qcontent, rcontent) = Self::layout_command_pane(dims, results.len());
        self.render_border(buf, qborder);
        self.render_query(buf, qcontent);
        self.render_results(buf, rcontent, results);

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
