use rope::Chars;

use crate::{Buffer, Editor};

#[derive(Debug)]
enum CursorJumpState {
    Jumping,
    Done,
}

struct ForwardJumpWhitespace(CursorJumpState);

impl ForwardJumpWhitespace {
    fn new() -> Self {
        ForwardJumpWhitespace(CursorJumpState::Jumping)
    }

    fn run(&mut self, mut chars: Chars<'_>) -> usize {
        loop {
            match self.0 {
                CursorJumpState::Done => break chars.offset(),
                CursorJumpState::Jumping => {
                    *self = self.apply(chars.next());
                }
            };
        }
    }

    fn apply(&self, maybe_char: Option<char>) -> Self {
        use CursorJumpState::*;
        let next_state = match (&self.0, maybe_char) {
            (Jumping, Some(' ') | Some('\t')) => Jumping,
            (Jumping, _) => Done,
            (Done, _) => Done,
        };
        ForwardJumpWhitespace(next_state)
    }
}

impl Editor {
    pub(crate) fn cursor_move_left(&mut self, _buffer: &Buffer) {
        self.cursor.move_prev_column();
    }

    pub(crate) fn cursor_move_up(&mut self, buffer: &Buffer) {
        self.cursor.move_prev_line();
        match buffer.contents.line(self.cursor.line) {
            None => (),
            Some(line) => {
                let len = if !line.is_empty() { line.len() - 1 } else { 0 };
                self.cursor.column = std::cmp::min(len, self.cursor.column)
            }
        }
    }

    pub(crate) fn cursor_move_right(&mut self, buffer: &Buffer) {
        self.cursor.move_next_column();
        match buffer.contents.char_at(self.cursor.clone()) {
            None | Some('\n') => self.cursor.move_prev_column(),
            _ => (),
        }
    }

    pub(crate) fn cursor_move_down(&mut self, buffer: &Buffer) {
        self.cursor.move_next_line();
        match buffer.contents.line(self.cursor.line) {
            None => self.cursor.move_prev_line(),
            Some(line) => {
                let len = if !line.is_empty() { line.len() - 1 } else { 0 };
                self.cursor.column = std::cmp::min(len, self.cursor.column);
            }
        }
    }

    pub(crate) fn cursor_jump_forward_skip_ws(&mut self, buffer: &Buffer) {
        let offset = buffer
            .contents
            .point_to_offset(self.cursor)
            .expect("invalid cursor");

        let mut machine = ForwardJumpWhitespace::new();
        let chars = buffer.contents.chars(.., offset);
        let offset = machine.run(chars);
        let offset = if offset > 0 { offset - 1 } else { 0 };

        self.cursor = buffer
            .contents
            .offset_to_point(offset)
            .expect("invalid offset");
    }

    pub(crate) fn cursor_jump_forward_word_end(&mut self, buffer: &Buffer) {
        self.cursor_jump_forward_skip_ws(buffer);
    }

    pub(crate) fn cursor_jump_forward_word_next(&mut self, buffer: &Buffer) {}
}
