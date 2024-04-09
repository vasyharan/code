use anyhow::Result;
use core::ops::Range;
use slotmap::new_key_type;
use std::path::PathBuf;

use crate::Point;

new_key_type! {
    pub struct Id;
}

#[derive(Debug)]
pub struct Contents(Vec<String>);

#[derive(Debug)]
pub struct Buffer {
    pub id: Id,
    contents: Contents,
}

impl Buffer {
    pub fn empty(id: Id) -> Self {
        Self::new(id, Contents(vec![]))
    }

    pub fn new(id: Id, contents: Contents) -> Self {
        Self { id, contents }
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

    pub fn lines(&self, range: Range<usize>) -> &[String] {
        let range = range.start..std::cmp::min(range.end, self.contents.0.len());
        &self.contents.0[range]
    }

    pub fn line_at(&self, line: usize) -> Option<&str> {
        let line = line - 1;
        if line < self.contents.0.len() {
            Some(&self.contents.0[line])
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
