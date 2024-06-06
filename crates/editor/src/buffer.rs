use anyhow::Result;
use rope::{Rope, RopeBuilder};
use slotmap::new_key_type;
use std::ops::{Deref, DerefMut};
use std::path::PathBuf;
use tore::Point;

pub type Highlights = iset::IntervalMap<usize, String>;

new_key_type! {
    pub struct Id;
}

#[derive(Debug, Clone)]
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
        Self::new(id, Contents(Rope::new()))
    }

    pub fn new(id: Id, contents: Contents) -> Self {
        Self { id, contents, highlights: Default::default() }
    }

    pub async fn read(filename: &PathBuf) -> Result<Contents> {
        use tokio::fs::File;
        use tokio::io::AsyncReadExt;

        let mut file = File::open(filename).await?;

        const BUFFER_SIZE: usize = rope::MAX_BYTES * 2;
        let mut buffer = [0u8; BUFFER_SIZE];
        let mut builder = RopeBuilder::new();
        let mut fill_idx = 0; // How much `buffer` is currently filled with valid data
        loop {
            let read_count = file.read(&mut buffer[fill_idx..]).await?;
            fill_idx += read_count;

            // Determine how much of the buffer is valid utf8.
            let valid_count = match std::str::from_utf8(&buffer[..fill_idx]) {
                Ok(_) => fill_idx,
                Err(e) => e.valid_up_to(),
            };

            // Append the valid part of the buffer to the rope.
            if valid_count > 0 {
                // The unsafe block here is reinterpreting the bytes as
                // utf8.  This is safe because the bytes being
                // reinterpreted have already been validated as utf8
                // just above.
                builder.append(unsafe { std::str::from_utf8_unchecked(&buffer[..valid_count]) });
            }

            // Shift the un-read part of the buffer to the beginning.
            if valid_count < fill_idx {
                buffer.copy_within(valid_count..fill_idx, 0);
            }
            fill_idx -= valid_count;

            if fill_idx == BUFFER_SIZE {
                // Buffer is full and none of it could be consumed.  Utf8
                // codepoints don't get that large, so it's clearly not
                // valid text.
                anyhow::bail!("stream contained invalid UTF-8");
                // return Err(std::io::Error::new(
                //     std::io::ErrorKind::InvalidData,
                //     "stream did not contain valid UTF-8",
                // ));
            }

            // If we're done reading
            if read_count == 0 {
                if fill_idx > 0 {
                    // We couldn't consume all data.
                    anyhow::bail!("stream contained invalid UTF-8");
                    // return Err(io::Error::new(
                    //     io::ErrorKind::InvalidData,
                    //     "stream contained invalid UTF-8",
                    // ));
                } else {
                    return Ok(Contents(builder.finish()));
                }
            }
        }
    }

    pub fn command(&mut self, command: Command) {
        match command {
            Command::Highlight(hls) => self.highlights = hls,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Contents(Rope);

impl Contents {
    pub(crate) fn point_to_char_offset(&self, cursor: Point) -> usize {
        let line_offset = self.0.line_to_char(cursor.line);
        line_offset + cursor.column
    }

    pub(crate) fn char_offset_to_point(&self, offset: usize) -> Point {
        let line = self.0.char_to_line(offset);
        let column = offset - self.0.line_to_char(line);
        Point { line, column }
    }
}

impl Deref for Contents {
    type Target = Rope;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for Contents {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
