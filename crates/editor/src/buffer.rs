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
        Self::new(id, vec![])
    }

    pub fn new(id: Id, lines: Vec<String>) -> Self {
        Self { id, lines }
    }

    pub async fn read(filename: &PathBuf) -> Result<Vec<String>> {
        use tokio::fs::File;
        use tokio::io::{AsyncBufReadExt, BufReader};
        use tokio_stream::wrappers::LinesStream;
        use tokio_stream::StreamExt;

        let mut file = File::open(filename).await?;
        let rd = BufReader::new(&mut file);
        let lines = LinesStream::new(rd.lines());
        let lines: std::io::Result<Vec<String>> = lines.collect().await;
        let lines = lines?;
        Ok(lines)
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
