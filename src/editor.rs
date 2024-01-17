use crossterm::event::{KeyCode, KeyEvent};
use slotmap::new_key_type;

use crate::app::{self};
use crate::buffer::{self};
use crate::rope::{self, Rope};

#[derive(Debug, Clone)]
pub(crate) enum Direction {
    Up,
    Down,
    Left,
    Right,
}

#[derive(Debug, Clone)]
pub(crate) enum CursorJump {
    ForwardEndWord,
    ForwardNextWord,
}

#[derive(Debug, Clone)]
pub(crate) enum Command {
    ModeSet(Mode),
    CursorMove(Direction),
    CursorJump(CursorJump),
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
                KeyCode::Up => Some(Command::CursorMove(Direction::Up)),
                KeyCode::Down => Some(Command::CursorMove(Direction::Down)),
                KeyCode::Left => Some(Command::CursorMove(Direction::Left)),
                KeyCode::Right => Some(Command::CursorMove(Direction::Right)),
                KeyCode::Char('w') => Some(Command::CursorJump(CursorJump::ForwardNextWord)),
                KeyCode::Char('e') => Some(Command::CursorJump(CursorJump::ForwardEndWord)),
                KeyCode::Char('i') => Some(Command::ModeSet(Mode::Insert)),
                _ => None,
            },
            Mode::Insert => match key.code {
                KeyCode::Esc => Some(Command::ModeSet(Mode::Normal)),
                KeyCode::Up => Some(Command::CursorMove(Direction::Up)),
                KeyCode::Down => Some(Command::CursorMove(Direction::Down)),
                KeyCode::Left => Some(Command::CursorMove(Direction::Left)),
                KeyCode::Right => Some(Command::CursorMove(Direction::Right)),
                KeyCode::Char(c) => Some(Command::Insert(c)),
                _ => None,
            },
        }
    }

    pub(crate) fn command(&mut self, command: &Command) -> Option<app::Command> {
        match command {
            Command::ModeSet(mode) => self.mode = *mode,
            Command::CursorMove(direction) => match direction {
                Direction::Up => todo!(),
                Direction::Down => todo!(),
                Direction::Left => self.cursor_move_left(),
                Direction::Right => self.cursor_move_right(),
            },
            Command::CursorJump(jump) => match jump {
                CursorJump::ForwardEndWord => self.cursor_jump_forward_word_end(),
                CursorJump::ForwardNextWord => self.cursor_jump_forward_word_next(),
            },
            Command::Insert(c) => {
                return Some(app::Command::BufferInsert(self.buffer_id, *c));
            }
        };
        None
    }

    fn cursor_move_left(&mut self) -> () {
        if let Some(b'\n') = self.cursor.prev() {
            self.cursor.next();
        }
    }

    fn cursor_move_right(&mut self) -> () {
        if let Some(b'\n') = self.cursor.next() {
            self.cursor.prev();
        }
    }

    fn cursor_jump_forward_word_end(&mut self) -> () {
        loop {
            match self.cursor.next() {
                None => break,
                Some(b' ') | Some(b'\n') => {
                    self.cursor.prev();
                    break;
                }
                _ => { /* continue */ }
            }
        }
    }

    fn cursor_jump_forward_word_next(&mut self) -> () {
        self.cursor_jump_forward_word_end();
        loop {
            match self.cursor.next() {
                None => break,
                Some(b' ') => { /* continue */ }
                _ => {
                    self.cursor.prev();
                    break;
                }
            }
        }
    }
}
