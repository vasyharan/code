use bstr::ByteSlice;
use crossterm::cursor::SetCursorStyle;
use ratatui::prelude as tui;

use selector::Selector;
use tore::CursorPoint;

use crate::theme::Color;

#[derive(Debug)]
pub struct Theme {
    bg: Color,
    fg: Color,
    bg_selected: Color,
    fg_highlight: Color,
}

// pub trait Renderer<Id> {
//     fn render(dims: tui::Rect, buf: &mut tui::Buffer, result: selector::Result<Id>);
// }

#[derive(Debug)]
pub struct SelectorPane<'a, Id: Eq + Copy> {
    theme: Theme,
    // renderer: R,
    selector: &'a Selector<Id>,
}

impl<'a, Id: Eq + Copy> SelectorPane<'a, Id> {
    pub fn new(theme: &crate::Theme, selector: &'a Selector<Id>) -> Self {
        let bg = theme.palette("bg0").unwrap();
        let bg_selected = theme.palette("bg1").unwrap();
        let fg = theme.palette("fg0").unwrap();
        let fg_highlight = theme.palette("yellow").unwrap();
        let theme = Theme { bg, fg, bg_selected, fg_highlight };
        Self { theme, selector }
    }

    #[tracing::instrument(skip(self, buf, results, render))]
    pub fn render<R>(
        self,
        buf: &mut tui::Buffer,
        area: tui::Rect,
        results: &Vec<Id>,
        render: R,
    ) -> (CursorPoint, SetCursorStyle)
    where
        R: Fn(tui::Rect, &mut tui::Buffer, Id) -> (),
    {
        let area = self.layout(area, results.len());
        let (query_area, results_area) = Self::split_sections(area);
        self.render_borders(buf, area);
        self.render_query(buf, query_area);
        if results_area.is_some() {
            self.render_results(buf, results_area.unwrap(), results, render);
        }

        let cursor_pos = self.cursor_pos(self.selector.cursor, query_area);
        (cursor_pos, SetCursorStyle::BlinkingBlock)
    }

    fn cursor_pos(&self, cursor: tore::Point, area: tui::Rect) -> CursorPoint {
        let x = area.left() + (self.selector.query_prefix.len() as u16) + cursor.column as u16;
        let y = area.top();
        CursorPoint { x, y }
    }

    fn layout(&self, dims: tui::Rect, num_results: usize) -> tui::Rect {
        use std::cmp::min;

        const WIDTH_RATIO: u16 = 4;
        let y = 0; // dims.height / 5;
        let x0 = dims.width / WIDTH_RATIO;
        let x1 = x0 * (WIDTH_RATIO - 1);
        let width = min(x1 - x0, 80) + (self.selector.query_prefix.len() as u16) + 2;
        let height = if num_results > 0 {
            (num_results.clamp(1, 13) as u16) + 4
        } else {
            3
        };

        tui::Rect::new(x0, y, width, height)
    }

    fn split_sections(area: tui::Rect) -> (tui::Rect, Option<tui::Rect>) {
        let query = tui::Rect::new(area.x + 1, area.y + 1, area.width - 2, 1);
        let results = if area.height > 4 {
            Some(tui::Rect::new(area.x + 1, area.y + 3, area.width - 2, area.height - 4))
        } else {
            None
        };
        (query, results)
    }

    fn render_borders(&self, buf: &mut tui::Buffer, area: tui::Rect) {
        use ratatui::symbols;

        let style = tui::Style::reset()
            .fg(self.theme.fg.into())
            .bg(self.theme.bg.into());

        for y in area.top()..area.bottom() {
            // left-vertical border
            let symbol = symbols::line::NORMAL.vertical;
            buf.get_mut(area.left(), y)
                .set_style(style)
                .set_symbol(symbol);

            // right-vertical border
            buf.get_mut(area.right() - 1, y)
                .set_style(style)
                .set_symbol(symbol);
        }
        for x in area.left()..area.right() {
            let (is_left, is_right) = (x == area.left(), x == area.right() - 1);
            // top-horizontal border
            let symbol = if is_left {
                symbols::line::NORMAL.top_left
            } else if is_right {
                symbols::line::NORMAL.top_right
            } else {
                symbols::line::NORMAL.horizontal
            };
            buf.get_mut(x, area.top())
                .set_style(style)
                .set_symbol(symbol);

            // separator border
            let symbol = if is_left {
                symbols::line::NORMAL.vertical_right
            } else if is_right {
                symbols::line::NORMAL.vertical_left
            } else {
                symbols::line::NORMAL.horizontal
            };
            buf.get_mut(x, area.top() + 2)
                .set_style(style)
                .set_symbol(symbol);

            // bottom-horizontal border
            let symbol = if is_left {
                symbols::line::NORMAL.bottom_left
            } else if is_right {
                symbols::line::NORMAL.bottom_right
            } else {
                symbols::line::NORMAL.horizontal
            };
            buf.get_mut(x, area.bottom() - 1)
                .set_style(style)
                .set_symbol(symbol);
        }
    }

    fn render_query(&self, buf: &mut tui::Buffer, area: tui::Rect) {
        let range = area.left()..area.right();
        let style = tui::Style::reset()
            .fg(self.theme.fg.into())
            .bg(self.theme.bg.into());

        let query_prefix = self.selector.query_prefix.as_bytes().as_bstr().graphemes();
        let query = self.selector.query.as_bytes().as_bstr().graphemes();
        let mut query = query_prefix.chain(query);
        for x in range {
            let c = query.next().unwrap_or(" ");
            buf.get_mut(x, area.y).set_style(style).set_symbol(c);
        }
    }

    fn render_results<R>(
        &self,
        buf: &mut tui::Buffer,
        area: tui::Rect,
        results: &Vec<Id>,
        render: R,
    ) where
        R: Fn(tui::Rect, &mut tui::Buffer, Id) -> (),
    {
        let style = tui::Style::reset()
            .fg(self.theme.fg.into())
            .bg(self.theme.bg.into());

        let has_results = !results.is_empty();
        if !has_results {
            return;
        }

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
                let result: Option<&Id> = results.next();
                let (prefix, bg) = match (self.selector.focused, result) {
                    (None, Some(_)) => unreachable!("something must be focused if results exist"),
                    (Some(selected), Some(result)) => {
                        if selected == *result {
                            ("", self.theme.bg_selected)
                        } else {
                            (" ", self.theme.bg)
                        }
                    }
                    (_, None) => (" ", self.theme.bg),
                };
                if let Some(result) = result {
                    let graphemes = prefix.as_bytes().as_bstr().graphemes();
                    let mut len = 0;
                    for (i, symbol) in graphemes.enumerate() {
                        len += 1;
                        buf.get_mut(area.left() + i as u16, y).set_symbol(symbol);
                    }
                    render(tui::Rect::new(area.left() + len, y, area.width, 1), buf, *result);
                }

                // let (content_prefix, bg) = match (self.selector.selected, &result) {
                //     (None, Some(_)) => unreachable!("something must be selected if results exist"),
                //     (Some(_), None) => unreachable!("selected entry must exist if results exist"),
                //     (Some(selected), Some(result)) => {
                //         if selected == result.id {
                //             ("", self.theme.bg_selected)
                //         } else {
                //             (" ", self.theme.bg)
                //         }
                //     }
                //     (_, None) => (" ", self.theme.bg),
                // };

                // // let content = result
                // //     .as_ref()
                // //     .map(|r| format!("{}{}", content_prefix, r.))
                // //     .unwrap_or("".to_string());
                // let content_prefix = content_prefix.as_bytes().as_bstr().graphemes();
                // let content: bstr::Graphemes = todo!(); //
                // let content = content_prefix.chain(content);
                // let mut indices = result
                //     .map(|r| r.indices)
                //     .unwrap_or_default()
                //     .into_iter()
                //     .peekable();
                // let maxlen = area.width as usize;
                // let mut graphemes = content.as_bytes().as_bstr().graphemes();
                // for (idx, x) in (area.left()..area.right()).enumerate() {
                //     let symbol = graphemes.next().unwrap_or(" ");
                //     let next_idx = *indices.peek().unwrap_or(&maxlen);
                //     let fg = if idx == 0 {
                //         self.theme.fg_highlight
                //     } else if next_idx + 1 == idx {
                //         indices.next();
                //         self.theme.fg_highlight
                //     } else {
                //         self.theme.fg
                //     };
                //     let style = tui::Style::reset().fg(fg.into()).bg(bg.into());
                //     buf.get_mut(x, y).set_style(style).set_symbol(symbol);
                // }
            }
        }
    }
}
