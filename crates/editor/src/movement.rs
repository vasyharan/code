use tore::Point;

use crate::{Buffer, Editor};

impl Editor {
    pub fn cursor_move_left(&mut self, _buffer: &Buffer) {
        self.cursor.move_prev_column();
    }

    pub fn cursor_move_up(&mut self, buffer: &Buffer) {
        self.cursor.move_prev_line();
        // match buffer.contents.line(self.cursor.line) {
        //     None => (),
        //     Some(line) => {
        //         let len = if !line.is_empty() { line.len() - 1 } else { 0 };
        //         self.cursor.column = std::cmp::min(len, self.cursor.column)
        //     }
        // }
        let line = buffer.contents.line(self.cursor.line);
        let len = line.len_chars();
        let len = if len == 0 { 0 } else { len - 1 };
        self.cursor.column = std::cmp::min(len, self.cursor.column);
    }

    pub fn cursor_move_right(&mut self, buffer: &Buffer) {
        self.cursor.move_next_column();
        // match buffer.contents.char_at(self.cursor) {
        //     None | Some('\n') => self.cursor.move_prev_column(),
        //     _ => (),
        // }
        let line = buffer.contents.line(self.cursor.line);
        match line.chars().nth(self.cursor.column) {
            None | Some('\n') => self.cursor.move_prev_column(),
            _ => (),
        }
    }

    pub fn cursor_move_down(&mut self, buffer: &Buffer) {
        self.cursor.move_next_line();
        // match buffer.contents.line(self.cursor.line) {
        //     None => self.cursor.move_prev_line(),
        //     Some(line) => {
        //         let len = if !line.is_empty() { line.len() - 1 } else { 0 };
        //         self.cursor.column = std::cmp::min(len, self.cursor.column);
        //     }
        // }
        let line = buffer.contents.line(self.cursor.line);
        let len = line.len_chars();
        let len = if len == 0 { 0 } else { len - 1 };
        self.cursor.column = std::cmp::min(len, self.cursor.column);
    }

    pub fn cursor_jump_line_zero(&mut self, _buffer: &Buffer) {
        self.cursor.column = 0;
    }

    pub fn cursor_jump_start_of_nearest_word(&mut self, buffer: &Buffer) {
        let line_offset = buffer.contents.line_to_char(self.cursor.line);
        let mut offset = line_offset + self.cursor.column;

        #[derive(Debug, Clone, Copy, PartialEq, Eq)]
        enum State {
            Init,
            SkipWord,
            SkipPunctuation,
            SkipWhitespace,
            Done,
        }

        let mut state = State::Init;
        let mut chars = buffer.contents.chars_at(offset);
        loop {
            match state {
                State::Done => break,
                _ => match chars.prev() {
                    None => break,
                    Some(char) => {
                        offset -= 1;
                        match state {
                            State::Done => unreachable!("invalid state"),
                            State::Init | State::SkipWhitespace => {
                                if char.is_alphanumeric() {
                                    state = State::SkipWord;
                                } else if char.is_ascii_punctuation() {
                                    state = State::SkipPunctuation;
                                } else if is_whitespace(char) {
                                    state = State::SkipWhitespace;
                                } else {
                                    state = State::Done;
                                }
                            }
                            State::SkipWord => {
                                if char.is_alphanumeric() {
                                    state = State::SkipWord;
                                } else {
                                    offset += 1;
                                    state = State::Done;
                                }
                            }
                            State::SkipPunctuation => {
                                if char.is_ascii_punctuation() {
                                    state = State::SkipPunctuation;
                                } else {
                                    offset += 1;
                                    state = State::Done;
                                }
                            }
                        }
                    }
                },
            }
        }

        let line = buffer.contents.char_to_line(offset);
        let column = offset - buffer.contents.line_to_char(line);
        self.cursor = Point { line, column };
    }

    pub fn cursor_jump_start_of_last_word(&mut self, _buffer: &Buffer) {
        // ge
        todo!()
    }

    pub fn cursor_jump_end_of_nearest_word(&mut self, buffer: &Buffer) {
        let line_offset = buffer.contents.line_to_char(self.cursor.line);
        let mut offset = line_offset + self.cursor.column + 1;

        #[derive(Debug, Clone, Copy, PartialEq, Eq)]
        enum State {
            Init,
            SkipWord,
            SkipPunctuation,
            SkipWhitespace,
            Done,
        }

        let mut state = State::Init;
        let mut chars = buffer.contents.chars_at(offset);
        loop {
            match state {
                State::Done => break,
                _ => match chars.next() {
                    None => break,
                    Some(char) => {
                        offset += 1;
                        match state {
                            State::Done => unreachable!("invalid state"),
                            State::Init | State::SkipWhitespace => {
                                if char.is_alphanumeric() {
                                    state = State::SkipWord;
                                } else if char.is_ascii_punctuation() {
                                    state = State::SkipPunctuation;
                                } else if is_whitespace(char) {
                                    state = State::SkipWhitespace;
                                } else {
                                    offset -= 1;
                                    state = State::Done;
                                }
                            }
                            State::SkipWord => {
                                if char.is_alphanumeric() {
                                    state = State::SkipWord;
                                } else {
                                    offset -= 2;
                                    state = State::Done;
                                }
                            }
                            State::SkipPunctuation => {
                                if char.is_ascii_punctuation() {
                                    state = State::SkipPunctuation;
                                } else {
                                    offset -= 2;
                                    state = State::Done;
                                }
                            }
                        }
                    }
                },
            }
        }

        let line = buffer.contents.char_to_line(offset);
        let column = offset - buffer.contents.line_to_char(line);
        self.cursor = Point { line, column };
    }

    pub fn cursor_jump_start_of_next_word(&mut self, buffer: &Buffer) {
        let line_offset = buffer.contents.line_to_char(self.cursor.line);
        let mut offset = line_offset + self.cursor.column;

        #[derive(Debug, Clone, Copy, PartialEq, Eq)]
        enum State {
            Init,
            SkipWord,
            SkipPunctuation,
            SkipWhitespace,
            Done,
        }

        let mut chars = buffer.contents.chars_at(offset);
        let mut state = State::Init;
        loop {
            match state {
                State::Done => break,
                _ => match chars.next() {
                    None => break,
                    Some(char) => {
                        offset += 1;
                        match state {
                            State::Done => unreachable!("invalid state"),
                            State::Init => {
                                if char.is_alphanumeric() {
                                    state = State::SkipWord;
                                } else if char.is_ascii_punctuation() {
                                    state = State::SkipPunctuation;
                                } else if is_whitespace(char) {
                                    state = State::SkipWhitespace;
                                } else {
                                    state = State::Done;
                                }
                            }
                            State::SkipWhitespace => {
                                if char == ' ' || char == '\t' || char == '\r' || char == '\n' {
                                    state = State::SkipWhitespace;
                                } else {
                                    offset -= 1;
                                    state = State::Done;
                                }
                            }
                            State::SkipWord => {
                                if char.is_alphanumeric() {
                                    state = State::SkipWord;
                                } else {
                                    chars.prev();
                                    offset -= 1;
                                    state = State::SkipWhitespace;
                                }
                            }
                            State::SkipPunctuation => {
                                if char.is_ascii_punctuation() {
                                    state = State::SkipPunctuation;
                                } else {
                                    chars.prev();
                                    offset -= 1;
                                    state = State::SkipWhitespace;
                                }
                            }
                        }
                    }
                },
            }
        }

        let line = buffer.contents.char_to_line(offset);
        let column = offset - buffer.contents.line_to_char(line);
        self.cursor = Point { line, column };
    }
}

fn is_whitespace(char: char) -> bool {
    char == ' ' || char == '\t' || char == '\r' || char == '\n'
}
