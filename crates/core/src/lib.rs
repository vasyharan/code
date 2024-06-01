#[derive(Debug, Clone, Copy, Default, PartialOrd, PartialEq)]
pub struct Point {
    pub line: usize,
    pub column: usize,
}

impl Point {
    pub fn move_next_column(&mut self) {
        self.column += 1;
    }

    pub fn move_prev_column(&mut self) {
        if self.column > 0 {
            self.column -= 1;
        }
    }

    pub fn move_next_line(&mut self) {
        self.line += 1;
    }

    pub fn move_prev_line(&mut self) {
        if self.line > 0 {
            self.line -= 1;
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialOrd, PartialEq)]
pub struct CursorPoint {
    pub x: u16,
    pub y: u16,
}

impl CursorPoint {}
