use anyhow::Result;
use core::ops::Range;
use slotmap::new_key_type;
use std::path::PathBuf;

use crate::Point;

new_key_type! {
    pub struct Id;
}

#[derive(Debug)]
pub struct Buffer {
    pub id: Id,
    lines: Vec<String>,
}

impl Buffer {
    pub fn empty(id: Id) -> Self {
        Self { id, lines: vec![] }
    }

    pub fn open(id: Id, filename: &PathBuf) -> Result<Self> {
        use std::fs::File;
        use std::io::BufRead;
        use std::io::BufReader;

        let file = File::open(filename)?;
        let rd = BufReader::new(&file);
        let lines: std::io::Result<Vec<String>> = rd.lines().collect();
        let lines = lines?;
        Ok(Self { id, lines })
    }

    pub fn lines(&self, range: Range<usize>) -> &[String] {
        let range = range.start..std::cmp::min(range.end, self.lines.len());
        &self.lines[range]
    }

    pub fn line_at(&self, line: usize) -> Option<&str> {
        let line = line - 1;
        if line < self.lines.len() {
            Some(&self.lines[line])
        } else {
            None
        }
    }

    pub fn char_at(&self, point: Point) -> Option<u8> {
        self.line_at(point.line).and_then(|line| {
            let column = point.column - 1;
            if column < line.len() {
                Some(line.as_bytes()[column])
            } else {
                None
            }
        })
    }
}
