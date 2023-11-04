use std::ops::Bound;

pub(crate) type Result<T> = std::result::Result<T, Error>;

#[derive(Debug)]
pub enum Error {
    IndexOutOfBounds(usize, usize),
    RangeOutOfBounds(Bound<usize>, Bound<usize>, usize),
}

impl Error {
    pub(super) fn deref_bound<T: Copy>(b: Bound<&T>) -> Bound<T> {
        use Bound::*;
        match b {
            Included(x) => Included(*x),
            Excluded(x) => Excluded(*x),
            Unbounded => Unbounded,
        }
    }
}
