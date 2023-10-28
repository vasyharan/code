use std::{
    cmp::min,
    ops::{Range, RangeBounds},
    sync::Arc,
};

use tokio::{fs::File, io::AsyncReadExt};

const BLOCK_CAPACITY: usize = 4096;

#[derive(Debug)]
struct Bytes([u8; BLOCK_CAPACITY]); // TODO: tune size of byte array

#[derive(Debug)]
pub struct BlockRange(Arc<Bytes>, Range<usize>);

impl BlockRange {
    pub fn len(&self) -> usize {
        self.1.len()
    }

    pub fn as_bytes(&self) -> &[u8] {
        &self.0.as_ref().0[self.1.clone()]
    }

    pub fn substr(&self, range: impl RangeBounds<usize>) -> Self {
        use core::ops::Bound;
        let start = self.1.start
            + match range.start_bound() {
                Bound::Included(&n) => n,
                Bound::Excluded(&n) => n + 1,
                Bound::Unbounded => 0,
            };

        let end = match range.end_bound() {
            Bound::Included(&n) => start + n + 1,
            Bound::Excluded(&n) => start + n,
            Bound::Unbounded => self.1.end,
        };
        assert!(start >= self.1.start && start <= self.1.end);
        assert!(end >= self.1.start && end <= self.1.end);

        Self(self.0.clone(), start..end)
    }
}

pub struct BlockBuffer {
    block: Arc<Bytes>,
    head: usize,
}

impl BlockBuffer {
    pub fn new() -> Self {
        Self { block: Arc::new(Bytes([0; BLOCK_CAPACITY])), head: 0 }
    }

    pub fn append(&mut self, val: &[u8]) -> std::io::Result<(BlockRange, usize)> {
        use std::io::Write;
        let head = self.head;
        let len = min(val.len(), BLOCK_CAPACITY - head);
        let mut bytes: &mut [u8] = unsafe {
            let bytes = (&self.block.as_ref().0 as *const u8) as *mut u8;
            std::slice::from_raw_parts_mut(bytes.offset(head as isize), len)
        };
        let written = bytes.write(&val[..len])?;
        self.head += written;
        let range = head..(head + written);
        Ok((BlockRange(self.block.clone(), range), written))
    }

    pub async fn read(&mut self, file: &mut File) -> std::io::Result<(BlockRange, usize)> {
        let head = self.head;
        let len = BLOCK_CAPACITY - head;
        let mut bytes: &mut [u8] = unsafe {
            let bytes = (&self.block.as_ref().0 as *const u8) as *mut u8;
            std::slice::from_raw_parts_mut(bytes.offset(head as isize), len)
        };
        let written = file.read(&mut bytes).await?;
        self.head += written;
        let range = head..(head + written);
        Ok((BlockRange(self.block.clone(), range), written))
    }
}
