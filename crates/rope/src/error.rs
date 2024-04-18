use std::ops::Bound;

pub(crate) type Result<T> = std::result::Result<T, Error>;

#[derive(Debug)]
pub enum Error {
    IndexOutOfBounds(usize, usize),
    RangeOutOfBounds(Bound<usize>, Bound<usize>, usize),
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl std::error::Error for Error {}

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
