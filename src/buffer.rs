use std::path::PathBuf;

use iset::IntervalMap;

use crate::rope::Rope;

#[derive(Debug)]
pub(crate) struct Buffer {
    pub(crate) path: Option<PathBuf>,
    pub(crate) contents: Rope,
    pub(crate) highlights: IntervalMap<usize, String>,
}

impl Buffer {
    pub(crate) fn empty() -> Self {
        let contents = Rope::empty();
        let highlights = IntervalMap::new();
        Self { path: None, contents, highlights }
    }

    pub(crate) fn new(path: PathBuf, contents: Rope) -> Self {
        let highlights = IntervalMap::new();
        Self { path: Some(path), contents, highlights }
    }
}
