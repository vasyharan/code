use crate::Buffer;

#[derive(Clone, Copy, Debug, Default)]
pub enum Mode {
    #[default]
    Normal,
    Insert,
}

#[derive(Debug, Clone, Copy)]
pub struct Point {
    pub line: usize,
    pub column: usize,
}

impl Default for Point {
    fn default() -> Self {
        Self { line: 1, column: 1 }
    }
}

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
}
