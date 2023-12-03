use crate::app::BufferId;

#[derive(Debug, Default)]
pub(crate) struct EditorPosition {
    pub(crate) row: usize,
    pub(crate) col: usize,
}

#[derive(Debug)]
pub(crate) struct EditorContext {
    pub(crate) buffer_id: BufferId,
    pub(crate) cursor_pos: EditorPosition,
}

impl EditorContext {
    pub(crate) fn new_buffer(buffer_id: BufferId) -> Self {
        Self { buffer_id, cursor_pos: Default::default() }
    }

    pub(crate) fn cursor_up(&mut self) -> () {
        if self.cursor_pos.row > 0 {
            self.cursor_pos.row -= 1;
        }
    }

    pub(crate) fn cursor_down(&mut self) -> () {
        self.cursor_pos.row += 1;
    }

    pub(crate) fn cursor_right(&mut self) -> () {
        self.cursor_pos.col += 1;
    }

    pub(crate) fn cursor_left(&mut self) {
        if self.cursor_pos.col > 0 {
            self.cursor_pos.col -= 1;
        }
    }
}
