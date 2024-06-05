use crate::{Buffer, BufferId};
use crossterm::event::KeyEvent;
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
    ModeSet(Mode),
    SwapBuffer(BufferId),
    CursorMove(Direction),
    CursorJump(CursorJump),
}

#[derive(Debug)]
pub struct Editor {
    pub id: Id,
    pub buffer_id: BufferId,
    pub mode: Mode,
    pub cursor: Point,
}

impl Editor {
    pub fn new(id: Id, buffer_id: BufferId) -> Self {
        Self { id, buffer_id, mode: Default::default(), cursor: Default::default() }
    }

    pub fn process_key(&mut self, key: KeyEvent, buffer: &Buffer) -> Option<Command> {
        use crossterm::event::KeyCode;
        debug_assert!(buffer.id == self.buffer_id);

        match self.mode {
            Mode::Normal => match key.code {
                KeyCode::Up | KeyCode::Char('k') => self.cursor_move_up(buffer),
                KeyCode::Down | KeyCode::Char('j') => self.cursor_move_down(buffer),
                KeyCode::Left | KeyCode::Char('h') => self.cursor_move_left(buffer),
                KeyCode::Right | KeyCode::Char('l') => self.cursor_move_right(buffer),
                KeyCode::Char('w') => self.cursor_jump_start_of_next_word(buffer),
                KeyCode::Char('e') => self.cursor_jump_end_of_nearest_word(buffer),
                KeyCode::Char('b') => self.cursor_jump_start_of_nearest_word(buffer),
                KeyCode::Char('0') => self.cursor_jump_line_zero(buffer),
                // KeyCode::Char('i') => Some(Command::ModeSet(Mode::Insert)),
                _ => (),
            },
            Mode::Insert => match key.code {
                // KeyCode::Esc => Some(Command::ModeSet(Mode::Normal)),
                KeyCode::Up => self.cursor_move_up(buffer),
                KeyCode::Down => self.cursor_move_down(buffer),
                KeyCode::Left => self.cursor_move_left(buffer),
                KeyCode::Right => self.cursor_move_right(buffer),
                // KeyCode::Char(c) => Some(Command::Insert(c)),
                _ => (),
            },
        }
        None
    }

    pub fn swap_buffer(&mut self, buffer_id: BufferId) {
        self.buffer_id = buffer_id;
    }

    pub fn command(&mut self, buffer: &Buffer, command: Command) {
        debug_assert!(buffer.id == self.buffer_id);
        match command {
            Command::SwapBuffer(buffer_id) => self.swap_buffer(buffer_id),
            Command::ModeSet(mode) => self.mode = mode,
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
}
