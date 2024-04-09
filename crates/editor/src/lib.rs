mod buffer;
mod editor;

pub use buffer::{Buffer, Contents as BufferContents, Id as BufferId};
pub use editor::{Command as EditorCommand, Editor, Id as EditorId};

#[derive(Debug, Clone)]
pub enum Command {
    Editor(EditorId, EditorCommand),
}

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
