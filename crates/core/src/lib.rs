#[derive(Debug, Clone, Copy, Default, PartialOrd, PartialEq)]
pub struct Point {
    pub line: usize,
    pub column: usize,
}

impl Point {
    pub fn next_column(&self) -> Self {
        Self { line: self.line, column: self.column + 1 }
    }

    pub fn prev_column(&self) -> Self {
        if self.column == 0 {
            *self
        } else {
            Self { line: self.line, column: self.column - 1 }
        }
    }

    pub fn next_line(&self) -> Self {
        Self { line: self.line + 1, column: self.column }
    }

    pub fn prev_line(&self) -> Self {
        if self.line == 0 {
            *self
        } else {
            Self { line: self.line - 1, column: self.column }
        }
    }
}