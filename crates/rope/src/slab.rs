use std::cmp::min;
use std::ops::{Range, RangeBounds};
use std::sync::Arc;

use bstr::ByteSlice;
use tokio::fs::File;
use tokio::io::AsyncReadExt;

const BLOCK_CAPACITY: usize = 4096;

#[derive(Debug)]
struct SlabBlock([u8; BLOCK_CAPACITY]); // TODO: tune size of byte array

#[derive(Clone)]
pub struct Slab(Arc<SlabBlock>, Range<usize>);

impl std::fmt::Debug for Slab {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Slab({}/{})", &self.as_bytes().as_bstr(), self.1.len())
    }
}

impl Slab {
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
        use std::ops::Bound;
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

pub struct SlabAllocator {
    block: Arc<SlabBlock>,
    head: usize,
}

impl Default for SlabAllocator {
    fn default() -> Self {
        Self::new()
    }
}

impl SlabAllocator {
    pub fn new() -> Self {
        Self { block: Arc::new(SlabBlock([0; BLOCK_CAPACITY])), head: 0 }
    }

    pub fn append(&mut self, val: &[u8]) -> std::io::Result<(Slab, usize)> {
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
        Ok((Slab(block.clone(), range), written))
    }

    pub async fn read(&mut self, file: &mut File) -> std::io::Result<(Slab, usize)> {
        let (block, head, rem) = self.block_remaining();
        let bytes: &mut [u8] = unsafe {
            let bytes = (&block.as_ref().0 as *const u8) as *mut u8;
            std::slice::from_raw_parts_mut(bytes.add(head), rem)
        };
        let written = file.read(bytes).await?;
        self.head += written;
        let range = head..(head + written);
        Ok((Slab(block.clone(), range), written))
    }

    fn block_remaining(&mut self) -> (Arc<SlabBlock>, usize, usize) {
        if self.head >= BLOCK_CAPACITY {
            // new block please
            self.block = Arc::new(SlabBlock([0; BLOCK_CAPACITY]));
            self.head = 0;
        }
        (self.block.clone(), self.head, BLOCK_CAPACITY - self.head)
    }
}

