use crate::{Buffer, BufferId, Point};
use crossterm::event::KeyEvent;
use slotmap::new_key_type;

new_key_type! {
    pub struct Id;
}

#[derive(Clone, Copy, Debug, Default)]
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
    ForwardEndWord,
    ForwardNextWord,
}

#[derive(Debug, Clone)]
pub enum Command {
    ModeSet(Mode),
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
        Self {
            id,
            buffer_id,
            mode: Default::default(),
            cursor: Default::default(),
        }
    }

    pub fn process_key(&mut self, key: KeyEvent, buffer: &Buffer) -> Option<crate::Command> {
        use crossterm::event::KeyCode;
        debug_assert!(buffer.id == self.buffer_id);

        match self.mode {
            Mode::Normal => match key.code {
                KeyCode::Up => self.cursor_move_up(buffer),
                KeyCode::Down => self.cursor_move_down(buffer),
                KeyCode::Left => self.cursor_move_left(buffer),
                KeyCode::Right => self.cursor_move_right(buffer),
                KeyCode::Char('w') => self.cursor_jump_forward_word_next(buffer),
                KeyCode::Char('e') => self.cursor_jump_forward_word_end(buffer),
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

    pub fn command(&mut self, buffer: &Buffer, command: Command) -> () {
        debug_assert!(buffer.id == self.buffer_id);
        match command {
            Command::ModeSet(mode) => self.mode = mode,
            Command::CursorMove(direction) => match direction {
                Direction::Up => self.cursor_move_up(buffer),
                Direction::Down => self.cursor_move_down(buffer),
                Direction::Left => self.cursor_move_left(buffer),
                Direction::Right => self.cursor_move_right(buffer),
            },
            Command::CursorJump(jump) => match jump {
                CursorJump::ForwardEndWord => self.cursor_jump_forward_word_end(buffer),
                CursorJump::ForwardNextWord => self.cursor_jump_forward_word_next(buffer),
            },
        };
    }

    fn cursor_move_left(&mut self, _buffer: &Buffer) -> () {
        self.cursor = self.cursor.prev_column();
    }

    fn cursor_move_up(&mut self, buffer: &Buffer) -> () {
        self.cursor = self.cursor.prev_line();
        match buffer.line_at(self.cursor.line) {
            None => (),
            Some(line) => self.cursor.column = std::cmp::min(line.len(), self.cursor.column),
        }
    }

    fn cursor_move_right(&mut self, buffer: &Buffer) -> () {
        let next_cursor = self.cursor.next_column();
        match buffer.char_at(next_cursor) {
            None | Some(b'\n') => (),
            _ => self.cursor = next_cursor,
        }
    }

    fn cursor_move_down(&mut self, buffer: &Buffer) -> () {
        let next_cursor = self.cursor.next_line();
        match buffer.line_at(next_cursor.line) {
            None => (),
            Some(line) => {
                self.cursor = next_cursor;
                self.cursor.column = std::cmp::min(line.len(), self.cursor.column);
            }
        }
    }

    fn cursor_jump_forward_word_end(&mut self, _buffer: &Buffer) -> () {
        // loop {
        //     match self.cursor.next() {
        //         None => break,
        //         Some(b' ') | Some(b'\n') => {
        //             self.cursor.prev();
        //             break;
        //         }
        //         _ => { /* continue */ }
        //     }
        // }
    }

    fn cursor_jump_forward_word_next(&mut self, _buffer: &Buffer) -> () {
        // self.cursor_jump_forward_word_end();
        // loop {
        //     match self.cursor.next() {
        //         None => break,
        //         Some(b' ') => { /* continue */ }
        //         _ => {
        //             self.cursor.prev();
        //             break;
        //         }
        //     }
        // }
    }
}
