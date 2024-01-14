use crossterm::event::{KeyCode, KeyEvent};
use slotmap::new_key_type;

use crate::app::{self};
use crate::buffer::{self};
use crate::rope::{self, Rope};

#[derive(Debug)]
pub(crate) enum Direction {
    Up,
    Down,
    Left,
    Right,
}

#[derive(Debug)]
pub(crate) enum Command {
    SetMode(Mode),
    MoveCursor(Direction),
    Insert(char),
}

#[derive(Debug, Default)]
pub(crate) struct Position(pub(crate) usize);

impl From<usize> for Position {
    fn from(value: usize) -> Self {
        Position(value)
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub(crate) enum Mode {
    #[default]
    Normal,
    Insert,
}

new_key_type! {
    pub(crate) struct Id;
}

#[derive(Debug)]
pub(crate) struct Editor {
    // pub(crate) id: Id,
    pub(crate) buffer_id: buffer::Id,
    pub(crate) mode: Mode,
    pub(crate) cursor: rope::Cursor,
}

impl Editor {
    pub(crate) fn new_contents(_id: Id, buffer_id: buffer::Id, contents: Rope) -> Self {
        let cursor = contents.cursor();
        Self { buffer_id, mode: Default::default(), cursor }
    }

    pub(crate) fn process_key(&self, key: KeyEvent) -> Option<Command> {
        match self.mode {
            Mode::Normal => match key.code {
                KeyCode::Up => Some(Command::MoveCursor(Direction::Up)),
                KeyCode::Down => Some(Command::MoveCursor(Direction::Down)),
                KeyCode::Left => Some(Command::MoveCursor(Direction::Left)),
                KeyCode::Right => Some(Command::MoveCursor(Direction::Right)),
                KeyCode::Char('i') => Some(Command::SetMode(Mode::Insert)),
                _ => None,
            },
            Mode::Insert => match key.code {
                KeyCode::Esc => Some(Command::SetMode(Mode::Normal)),
                KeyCode::Up => Some(Command::MoveCursor(Direction::Up)),
                KeyCode::Down => Some(Command::MoveCursor(Direction::Down)),
                KeyCode::Left => Some(Command::MoveCursor(Direction::Left)),
                KeyCode::Right => Some(Command::MoveCursor(Direction::Right)),
                KeyCode::Char(c) => Some(Command::Insert(c)),
                _ => None,
            },
        }
    }

    pub(crate) fn command(&mut self, command: &Command) -> Option<app::Command> {
        match command {
            Command::SetMode(mode) => self.mode = *mode,
            Command::MoveCursor(direction) => match direction {
                Direction::Up => todo!(),
                Direction::Down => todo!(),
                Direction::Left => {
                    self.cursor.prev();
                    if let Some(b'\n') = self.cursor.peek_byte() {
                        self.cursor.next();
                    }
                }
                Direction::Right => {
                    self.cursor.next();
                    if let Some(b'\n') = self.cursor.peek_byte() {
                        self.cursor.prev();
                    }
                }
            },
            Command::Insert(c) => {
                return Some(app::Command::BufferInsert(self.buffer_id, *c));
            }
        };
        None
    }
}
