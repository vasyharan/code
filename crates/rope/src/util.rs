use std::ops::{Bound, Range, RangeBounds};

pub(super) fn bound_range(range: &impl RangeBounds<usize>, bounds: Range<usize>) -> Range<usize> {
    let start = match range.start_bound() {
        Bound::Included(&n) => bounds.start + n,
        Bound::Excluded(&n) => bounds.start + n + 1,
        Bound::Unbounded => bounds.start,
    };

    let end = match range.end_bound() {
        Bound::Included(&n) => n + 1,
        Bound::Excluded(&n) => n,
        Bound::Unbounded => bounds.end,
    };

    std::cmp::min(start, bounds.end)..std::cmp::min(end, bounds.end)
}

#[cfg(test)]
mod tests {
    #[test]
    fn to_range() {
        assert_eq!(super::bound_range(&(0..0), 0..10), 0..0);
        assert_eq!(super::bound_range(&(0..5), 0..10), 0..5);
        assert_eq!(super::bound_range(&(0..10), 0..10), 0..10);
        assert_eq!(super::bound_range(&(0..20), 0..10), 0..10);
        assert_eq!(super::bound_range(&(5..10), 0..10), 5..10);
        assert_eq!(super::bound_range(&(10..10), 0..10), 10..10);
        assert_eq!(super::bound_range(&(20..10), 0..10), 10..10);

        assert_eq!(super::bound_range(&(0..), 0..10), 0..10);
        assert_eq!(super::bound_range(&(5..), 0..10), 5..10);
        assert_eq!(super::bound_range(&(10..), 0..10), 10..10);

        assert_eq!(super::bound_range(&(..0), 0..10), 0..0);
        assert_eq!(super::bound_range(&(..5), 0..10), 0..5);
        assert_eq!(super::bound_range(&(..10), 0..10), 0..10);

        assert_eq!(super::bound_range(&(..), 0..10), 0..10);

        assert_eq!(super::bound_range(&(0..=5), 0..10), 0..6);
        assert_eq!(super::bound_range(&(0..=9), 0..10), 0..10);
        assert_eq!(super::bound_range(&(0..=20), 0..10), 0..10);

        assert_eq!(super::bound_range(&(5..), 0..10), 5..10);
        assert_eq!(super::bound_range(&(..8), 0..10), 0..8);
        assert_eq!(super::bound_range(&(..), 0..10), 0..10);
    }
}