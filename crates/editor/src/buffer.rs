use anyhow::Result;
use core::ops::Range;
use slotmap::new_key_type;
use std::path::PathBuf;

use crate::Point;

pub type Highlights = iset::IntervalMap<Point, String>;

new_key_type! {
    pub struct Id;
}

#[derive(Debug)]
pub enum Command {
    Highlight(Highlights),
}

#[derive(Debug)]
pub struct Buffer {
    pub id: Id,
    pub contents: Contents,
    pub highlights: Highlights,
}

impl Buffer {
    pub fn empty(id: Id) -> Self {
        Self::new(id, Contents(vec![]))
    }

    pub fn new(id: Id, contents: Contents) -> Self {
        Self { id, contents, highlights: Default::default() }
    }

    pub async fn read(filename: &PathBuf) -> Result<Contents> {
        use tokio::fs::File;
        use tokio::io::{AsyncBufReadExt, BufReader};
        use tokio_stream::wrappers::LinesStream;
        use tokio_stream::StreamExt;

        let mut file = File::open(filename).await?;
        let rd = BufReader::new(&mut file);
        let lines = LinesStream::new(rd.lines());
        let lines: std::io::Result<Vec<String>> = lines.collect().await;
        let lines = lines?;
        Ok(Contents(lines))
    }

    pub fn command(&mut self, command: Command) -> () {
        match command {
            Command::Highlight(hls) => self.highlights = hls,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Contents(Vec<String>);

impl Contents {
    pub fn range(&self, start: Point, end: Point) -> Lines {
        Lines { contents: self, start, end }
    }

    pub fn lines(&self, range: Range<usize>) -> Lines {
        let start = Point { line: range.start, column: 0 };
        let end = Point { line: range.end, column: 0 };
        self.range(start, end)
    }

    pub fn line_at(&self, line: usize) -> Option<&str> {
        if line < self.0.len() {
            Some(&self.0[line])
        } else {
            None
        }
    }

    pub fn char_at(&self, point: Point) -> Option<u8> {
        self.line_at(point.line).and_then(|line| {
            let column = point.column;
            if column < line.len() {
                Some(line.as_bytes()[column])
            } else {
                None
            }
        })
    }
}

pub struct Lines<'a> {
    contents: &'a Contents,
    start: Point,
    end: Point,
}

impl<'a> Iterator for Lines<'a> {
    type Item = &'a [u8];

    fn next(&mut self) -> Option<Self::Item> {
        if self.start.line > self.end.line {
            return None;
        }

        let start = self.start;
        match self.contents.line_at(start.line) {
            None => None,
            Some(line) => {
                self.start = Point { line: start.line + 1, column: 0 };
                if start.column < line.as_bytes().len() {
                    if start.line == self.end.line {
                        if self.end.column < line.as_bytes().len() {
                            Some(&line.as_bytes()[start.column..self.end.column])
                        } else {
                            Some(&line.as_bytes()[start.column..])
                        }
                    } else {
                        Some(&line.as_bytes()[start.column..])
                    }
                } else {
                    Some(&[])
                }
            }
        }
    }
}
