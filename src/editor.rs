use crossterm::event::{KeyCode, KeyEvent};

use crate::app::BufferId;

#[derive(Debug)]
pub(crate) enum Command {
    SetMode(EditorMode),
    CursorUp,
    CursorDown,
    CursorRight,
    CursorLeft,
}

#[derive(Debug, Default)]
pub(crate) struct EditorPosition {
    pub(crate) row: usize,
    pub(crate) col: usize,
}

#[derive(Debug, Default)]
pub(crate) enum EditorMode {
    #[default]
    Normal,
    Insert,
}

#[derive(Debug)]
pub(crate) struct EditorContext {
    pub(crate) buffer_id: BufferId,
    pub(crate) mode: EditorMode,
    pub(crate) cursor_pos: EditorPosition,
}

impl EditorContext {
    pub(crate) fn new_buffer(buffer_id: BufferId) -> Self {
        Self { buffer_id, mode: Default::default(), cursor_pos: Default::default() }
    }

    pub(crate) fn process_key(&self, key: KeyEvent) -> Option<Command> {
        match self.mode {
            EditorMode::Normal => match key.code {
                KeyCode::Up => Some(Command::CursorUp),
                KeyCode::Down => Some(Command::CursorDown),
                KeyCode::Left => Some(Command::CursorLeft),
                KeyCode::Right => Some(Command::CursorRight),
                _ => None,
            },
            _ => todo!(),
        }
    }

    pub(crate) fn command(&mut self, command: Command) -> () {
        match command {
            Command::SetMode(mode) => {
                self.mode = mode;
            }
            Command::CursorUp => {
                if self.cursor_pos.row > 0 {
                    self.cursor_pos.row -= 1;
                }
            }
            Command::CursorDown => {
                self.cursor_pos.row += 1;
            }

            Command::CursorRight => {
                self.cursor_pos.col += 1;
            }
            Command::CursorLeft => {
                if self.cursor_pos.col > 0 {
                    self.cursor_pos.col -= 1;
                }
            }
        }
    }
}
