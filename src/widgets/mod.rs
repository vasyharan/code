use ratatui::prelude as tui;

mod editor_pane;

type Dimensions = tui::Rect;

#[derive(Debug, Clone)]
pub(crate) struct Point {
    pub(crate) row: u16,
    pub(crate) column: u16,
}

pub(crate) use editor_pane::EditorPane;
