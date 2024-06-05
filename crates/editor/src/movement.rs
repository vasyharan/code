use crate::{Buffer, Editor};

impl Editor {
    pub fn cursor_move_left(&mut self, _buffer: &Buffer) {
        self.cursor.move_prev_column();
    }

    pub fn cursor_move_up(&mut self, buffer: &Buffer) {
        self.cursor.move_prev_line();
        match buffer.contents.line(self.cursor.line) {
            None => (),
            Some(line) => {
                let len = if !line.is_empty() { line.len() - 1 } else { 0 };
                self.cursor.column = std::cmp::min(len, self.cursor.column)
            }
        }
    }

    pub fn cursor_move_right(&mut self, buffer: &Buffer) {
        self.cursor.move_next_column();
        match buffer.contents.char_at(self.cursor) {
            None | Some('\n') => self.cursor.move_prev_column(),
            _ => (),
        }
    }

    pub fn cursor_move_down(&mut self, buffer: &Buffer) {
        self.cursor.move_next_line();
        match buffer.contents.line(self.cursor.line) {
            None => self.cursor.move_prev_line(),
            Some(line) => {
                let len = if !line.is_empty() { line.len() - 1 } else { 0 };
                self.cursor.column = std::cmp::min(len, self.cursor.column);
            }
        }
    }

    pub fn cursor_jump_line_zero(&mut self, _buffer: &Buffer) {
        self.cursor.column = 0;
    }

    pub fn cursor_jump_backward_word_start(&mut self, _buffer: &Buffer) {
        // b
        todo!()
    }

    pub fn cursor_jump_backward_prev_word_end(&mut self, _buffer: &Buffer) {
        // ge
        todo!()
    }

    pub fn cursor_jump_forward_word_end(&mut self, buffer: &Buffer) {
        let offset = buffer
            .contents
            .point_to_offset(self.cursor)
            .expect("editor cursor should be a valid point");

        #[derive(Debug, Clone, Copy, PartialEq, Eq)]
        enum State {
            Init,
            SkipWord,
            SkipPunctuation,
            SkipWhitespace,
            Done,
        }

        let mut state = State::Init;
        let mut chars = buffer.contents.chars(.., offset);
        _ = chars.next();
        let chars = loop {
            match state {
                State::Done => break chars,
                _ => match chars.next() {
                    None => break chars,
                    Some(char) => match state {
                        State::Done => unreachable!("invalid state"),
                        State::Init | State::SkipWhitespace => {
                            if char.is_alphanumeric() {
                                state = State::SkipWord;
                            } else if char.is_ascii_punctuation() {
                                state = State::SkipPunctuation;
                            } else if char == ' ' || char == '\t' || char == '\r' || char == '\n' {
                                state = State::SkipWhitespace;
                            } else {
                                chars.prev();
                                state = State::Done;
                            }
                        }
                        State::SkipWord => {
                            if char.is_alphanumeric() {
                                state = State::SkipWord;
                            } else {
                                chars.prev();
                                chars.prev();
                                state = State::Done;
                            }
                        }
                        State::SkipPunctuation => {
                            if char.is_ascii_punctuation() {
                                state = State::SkipPunctuation;
                            } else {
                                chars.prev();
                                chars.prev();
                                state = State::Done;
                            }
                        }
                    },
                },
            }
        };

        let offset = chars.offset();
        self.cursor = buffer
            .contents
            .offset_to_point(offset)
            .expect("invalid offset");
    }

    pub fn cursor_jump_forward_next_word_start(&mut self, buffer: &Buffer) {
        let offset = buffer
            .contents
            .point_to_offset(self.cursor)
            .expect("editor cursor should be a valid point");

        #[derive(Debug, Clone, Copy, PartialEq, Eq)]
        enum State {
            Init,
            SkipWord,
            SkipPunctuation,
            SkipWhitespace,
            Done,
        }

        let mut chars = buffer.contents.chars(.., offset);
        let mut state = State::Init;
        let chars = loop {
            match state {
                State::Done => break chars,
                _ => match chars.next() {
                    None => break chars,
                    Some(char) => match state {
                        State::Done => unreachable!("invalid state"),
                        State::Init => {
                            if char.is_alphanumeric() {
                                state = State::SkipWord;
                            } else if char.is_ascii_punctuation() {
                                state = State::SkipPunctuation;
                            } else if char == ' ' || char == '\t' || char == '\r' || char == '\n' {
                                state = State::SkipWhitespace;
                            } else {
                                state = State::Done;
                            }
                        }
                        State::SkipWhitespace => {
                            if char == ' ' || char == '\t' || char == '\r' || char == '\n' {
                                state = State::SkipWhitespace;
                            } else {
                                chars.prev();
                                state = State::Done;
                            }
                        }
                        State::SkipWord => {
                            if char.is_alphanumeric() {
                                state = State::SkipWord;
                            } else {
                                chars.prev();
                                state = State::SkipWhitespace;
                            }
                        }
                        State::SkipPunctuation => {
                            if char.is_alphanumeric() {
                                state = State::SkipPunctuation;
                            } else {
                                chars.prev();
                                state = State::SkipWhitespace;
                            }
                        }
                    },
                },
            }
        };

        let offset = chars.offset();
        self.cursor = buffer
            .contents
            .offset_to_point(offset)
            .expect("invalid offset");
    }
}
