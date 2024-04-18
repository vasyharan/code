use anyhow::Result;
use rope::{Rope, SlabAllocator};
use slotmap::new_key_type;
use std::ops::Deref;
use std::path::PathBuf;

pub type Highlights = iset::IntervalMap<usize, String>;

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
        Self::new(id, Contents(Rope::empty()))
    }

    pub fn new(id: Id, contents: Contents) -> Self {
        Self { id, contents, highlights: Default::default() }
    }

    pub async fn read(alloc: &mut SlabAllocator, filename: &PathBuf) -> Result<Contents> {
        use tokio::fs::File;

        let mut file = File::open(filename).await?;
        let mut rope = Rope::empty();
        loop {
            let (block, read) = alloc.read(&mut file).await?;
            if read == 0 {
                break Ok(Contents(rope));
            }
            rope = rope.append(block)?;
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

impl Deref for Contents {
    type Target = Rope;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
