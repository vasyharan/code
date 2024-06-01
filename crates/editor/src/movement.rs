use rope::Chars;
use tracing::{event, Level};

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

    pub fn cursor_jump_line_zero(&mut self, buffer: &Buffer) {
        self.cursor.column = 0;
    }

    pub fn cursor_jump_forward_skip_ws(&mut self, buffer: &Buffer) {
        let offset = buffer
            .contents
            .point_to_offset(self.cursor)
            .expect("invalid cursor");

        let mut machine = ForwardJumpWhitespace::new();
        let chars = buffer.contents.chars(.., offset);
        let chars = machine.run(chars);
        let offset = chars.offset();

        self.cursor = buffer
            .contents
            .offset_to_point(offset)
            .expect("invalid offset");
    }
    pub fn cursor_jump_backward_word_start(&mut self, buffer: &Buffer) {
        // b
        todo!()
    }

    pub fn cursor_jump_backward_prev_word_end(&mut self, buffer: &Buffer) {
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

#[derive(Debug, PartialEq, Eq)]
enum ForwardJumpWhitespaceState {
    Whitespace,
    Reverse,
    Done,
}

struct ForwardJumpWhitespace(ForwardJumpWhitespaceState);

impl ForwardJumpWhitespace {
    fn new() -> Self {
        Self(ForwardJumpWhitespaceState::Whitespace)
    }

    fn run<'a>(&mut self, mut chars: Chars<'a>) -> Chars<'a> {
        use ForwardJumpWhitespaceState::*;
        loop {
            match self.0 {
                Done => break chars,
                _ => *self = self.apply(&mut chars),
            }
        }
    }

    fn apply(&self, chars: &mut Chars<'_>) -> Self {
        use ForwardJumpWhitespaceState::*;
        let next_state = match self.0 {
            Done => Done,
            Whitespace => match chars.next() {
                None => Done,
                Some(' ') | Some('\t') => Whitespace,
                Some('\r') | Some('\n') => Whitespace,
                Some(char) => {
                    event!(
                        Level::INFO,
                        target = "ForwardJumpWhitespace",
                        char = format!("{}", char)
                    );
                    Reverse
                }
            },
            Reverse => {
                chars.prev();
                Done
            }
        };
        event!(
            Level::INFO,
            target = "ForwardJumpWhitespace",
            state = format!("{:?}", self.0),
            next_state = format!("{:?}", next_state)
        );
        Self(next_state)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ForwardJumpWordEndState {
    Init,
    Word1,
    Word2,
    Punctuation1,
    Punctuation2,
    Reverse2,
    Reverse1,
    Done,
}

struct ForwardJumpWordEnd(ForwardJumpWordEndState);

impl ForwardJumpWordEnd {
    fn new() -> Self {
        Self(ForwardJumpWordEndState::Init)
    }

    fn run<'a>(&mut self, mut chars: Chars<'a>) -> Chars<'a> {
        use ForwardJumpWordEndState::*;
        loop {
            match self.0 {
                Done => break chars,
                _ => *self = self.apply(&mut chars),
            }
        }
    }

    fn apply(&self, chars: &mut Chars) -> Self {
        use ForwardJumpWordEndState::*;
        let next_state = match self.0 {
            Done => Done,
            Init | Word1 | Word2 | Punctuation1 | Punctuation2 => match chars.next() {
                None => Done,
                Some(char) => match self.0 {
                    Init => {
                        if char.is_ascii_punctuation() {
                            Punctuation1
                        } else if char.is_alphanumeric() {
                            Word1
                        } else {
                            Reverse1
                        }
                    }
                    Word1 => {
                        if char.is_alphanumeric() {
                            Word2
                        } else {
                            Reverse2
                        }
                    }
                    Word2 => {
                        if char.is_alphanumeric() {
                            Word2
                        } else {
                            Reverse2
                        }
                    }
                    Punctuation1 => {
                        if char.is_ascii_punctuation() {
                            Punctuation2
                        } else {
                            Reverse2
                        }
                    }
                    Punctuation2 => {
                        if char.is_ascii_punctuation() {
                            Punctuation2
                        } else {
                            Reverse2
                        }
                    }
                    state => state,
                },
            },
            Reverse2 => {
                chars.prev();
                Reverse1
            }
            Reverse1 => {
                chars.prev();
                Done
            }
        };

        event!(
            Level::INFO,
            target = "ForwardJumpWordEnd",
            state = format!("{:?}", self.0),
            next_state = format!("{:?}", next_state)
        );
        Self(next_state)
    }
}
