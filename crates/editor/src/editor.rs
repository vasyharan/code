use std::path::PathBuf;

use crate::{Buffer, BufferId};
use slotmap::new_key_type;
use tore::Point;

new_key_type! {
    pub struct Id;
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum Mode {
    #[default]
    Normal,
    Insert,
}

#[derive(Debug, Clone)]
pub enum Direction {
    Up,
    Down,
    Left,
    Right,
}

#[derive(Debug, Clone)]
pub enum CursorJump {
    StartOfNextWord,
    StartOfLastWord,
    EndOfNearestWord,
    StartOfNearestWord,
}

#[derive(Debug, Clone)]
pub enum Command {
    SetMode(Mode),
    SwapBuffer(BufferId),
    CursorMove(Direction),
    CursorJump(CursorJump),
    InsertChar(char),
}

#[derive(Debug)]
pub struct Editor {
    pub mode: Mode,
    pub id: Id,
    pub buffer_id: BufferId,
    pub cursor: Point,
}

impl Editor {
    pub fn new(id: Id, buffer_id: BufferId) -> Self {
        Self { id, mode: Mode::default(), buffer_id, cursor: Default::default() }
    }

    pub fn swap_buffer(&mut self, buffer_id: BufferId) {
        self.buffer_id = buffer_id;
    }

    pub fn command(&mut self, buffer: &mut Buffer, command: Command) {
        debug_assert!(buffer.id == self.buffer_id);
        match command {
            Command::SwapBuffer(buffer_id) => self.swap_buffer(buffer_id),
            Command::InsertChar(c) => self.insert_char(buffer, c),
            Command::SetMode(mode) => self.mode = mode,
            Command::CursorMove(direction) => match direction {
                Direction::Up => self.cursor_move_up(buffer),
                Direction::Down => self.cursor_move_down(buffer),
                Direction::Left => self.cursor_move_left(buffer),
                Direction::Right => self.cursor_move_right(buffer),
            },
            Command::CursorJump(jump) => match jump {
                CursorJump::StartOfNextWord => self.cursor_jump_start_of_next_word(buffer),
                CursorJump::StartOfLastWord => self.cursor_jump_start_of_last_word(buffer),
                CursorJump::EndOfNearestWord => self.cursor_jump_end_of_nearest_word(buffer),
                CursorJump::StartOfNearestWord => self.cursor_jump_start_of_nearest_word(buffer),
            },
        };
    }

    pub fn insert_char(&mut self, buffer: &mut Buffer, c: char) {
        let offset = buffer.contents.point_to_char_offset(self.cursor);
        self.cursor.move_next_column();
        buffer.contents.insert_char(offset, c);
    }
}
