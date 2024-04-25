use crate::{Buffer, Editor};

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
        match buffer.contents.char_at(self.cursor) {
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
        loop {
            match buffer.contents.char_at(self.cursor) {
                Some(' ') | Some('\t') => self.cursor.move_next_column(),
                _ => break,
            }
        }
    }

    pub(crate) fn cursor_jump_forward_word_end(&mut self, buffer: &Buffer) {
        self.cursor_jump_forward_skip_ws(buffer);
    }

    pub(crate) fn cursor_jump_forward_word_next(&mut self, buffer: &Buffer) {}
}
