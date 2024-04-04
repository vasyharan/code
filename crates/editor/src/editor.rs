use crate::Buffer;

#[derive(Debug, Clone, Copy)]
pub struct Point {
    pub line: usize,
    pub column: usize,
}

impl Point {
    pub fn next_column(&self) -> Self {
        Self {
            line: self.line,
            column: self.column + 1,
        }
    }

    pub fn prev_column(&self) -> Self {
        if self.column == 1 {
            self.clone()
        } else {
            Self {
                line: self.line,
                column: self.column - 1,
            }
        }
    }

    pub fn next_line(&self) -> Self {
        Self {
            line: self.line + 1,
            column: self.column,
        }
    }

    pub fn prev_line(&self) -> Self {
        if self.line == 1 {
            self.clone()
        } else {
            Self {
                line: self.line - 1,
                column: self.column,
            }
        }
    }
}

impl Default for Point {
    fn default() -> Self {
        Self { line: 1, column: 1 }
    }
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
    Insert(char),
}

#[derive(Debug)]
pub struct Editor<'a> {
    pub buffer: &'a Buffer,
    pub mode: Mode,
    pub cursor: Point,
}

impl<'a> Editor<'a> {
    pub fn new(buffer: &'a Buffer) -> Self {
        Self {
            buffer,
            mode: Default::default(),
            cursor: Default::default(),
        }
    }

    pub fn process_key(&self, key: crossterm::event::KeyEvent) -> Option<Command> {
        use crossterm::event::KeyCode;

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

    pub fn command(&mut self, command: Command) -> () {
        match command {
            Command::ModeSet(mode) => self.mode = mode,
            Command::CursorMove(direction) => match direction {
                Direction::Up => self.cursor_move_up(),
                Direction::Down => self.cursor_move_down(),
                Direction::Left => self.cursor_move_left(),
                Direction::Right => self.cursor_move_right(),
            },
            Command::CursorJump(jump) => match jump {
                CursorJump::ForwardEndWord => self.cursor_jump_forward_word_end(),
                CursorJump::ForwardNextWord => self.cursor_jump_forward_word_next(),
            },
            Command::Insert(c) => {
                // return Some(app::Command::BufferInsert(self.buffer_id, *c));
            }
        };
    }

    fn cursor_move_left(&mut self) -> () {
        self.cursor = self.cursor.prev_column();
    }

    fn cursor_move_up(&mut self) -> () {
        self.cursor = self.cursor.prev_line();
        match self.buffer.line_at(self.cursor.line) {
            None => (),
            Some(line) => self.cursor.column = std::cmp::min(line.len(), self.cursor.column),
        }
    }

    fn cursor_move_right(&mut self) -> () {
        let next_cursor = self.cursor.next_column();
        match self.buffer.char_at(next_cursor) {
            None | Some(b'\n') => (),
            _ => self.cursor = next_cursor,
        }
    }

    fn cursor_move_down(&mut self) -> () {
        let next_cursor = self.cursor.next_line();
        match self.buffer.line_at(next_cursor.line) {
            None => (),
            Some(line) => {
                self.cursor = next_cursor;
                self.cursor.column = std::cmp::min(line.len(), self.cursor.column);
            }
        }
    }

    fn cursor_jump_forward_word_end(&mut self) -> () {
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

    fn cursor_jump_forward_word_next(&mut self) -> () {
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
