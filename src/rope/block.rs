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
    pub fn is_empty(&self) -> bool {
    self.1.is_empty()
    }

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

impl Default for BlockBuffer {
    fn default() -> Self {
        Self::new()
    }
}

impl BlockBuffer {
    pub fn new() -> Self {
        Self { block: Arc::new(Bytes([0; BLOCK_CAPACITY])), head: 0 }
    }

    pub fn append(&mut self, val: &[u8]) -> std::io::Result<(BlockRange, usize)> {
        use std::io::Write;
        let (block, head, rem) = self.block_remaining();
        let len = min(val.len(), rem);
        let mut bytes: &mut [u8] = unsafe {
            let bytes = (&block.as_ref().0 as *const u8) as *mut u8;
            std::slice::from_raw_parts_mut(bytes.add(head), len)
        };
        let written = bytes.write(&val[..len])?;
        self.head += written;
        let range = head..(head + written);
        Ok((BlockRange(block.clone(), range), written))
    }

    pub async fn read(&mut self, file: &mut File) -> std::io::Result<(BlockRange, usize)> {
        let (block, head, rem) = self.block_remaining();
        let bytes: &mut [u8] = unsafe {
            let bytes = (&block.as_ref().0 as *const u8) as *mut u8;
            std::slice::from_raw_parts_mut(bytes.add(head), rem)
        };
        let written = file.read(bytes).await?;
        self.head += written;
        let range = head..(head + written);
        Ok((BlockRange(block.clone(), range), written))
    }

    fn block_remaining(&mut self) -> (Arc<Bytes>, usize, usize) {
        if self.head >= BLOCK_CAPACITY {
            // new block please
            self.block = Arc::new(Bytes([0; BLOCK_CAPACITY]));
            self.head = 0;
        }
        (self.block.clone(), self.head, BLOCK_CAPACITY - self.head)
    }
}
