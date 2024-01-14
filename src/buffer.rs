use std::path::PathBuf;

use iset::IntervalMap;
use slotmap::{new_key_type, SlotMap};

use crate::editor::{self, Editor};
use crate::rope::Rope;

new_key_type! {
    pub(crate) struct Id;
}

#[derive(Debug)]
pub(crate) struct Buffer {
    pub(crate) id: Id,
    // pub(crate) path: Option<PathBuf>,
    pub(crate) contents: Rope,
    pub(crate) highlights: IntervalMap<usize, String>,
    pub(crate) editors: SlotMap<editor::Id, Editor>,
}

impl Buffer {
    pub(crate) fn empty(id: Id) -> Self {
        Self::new(id, None, Rope::empty())
    }

    pub(crate) fn new(id: Id, _path: Option<PathBuf>, contents: Rope) -> Self {
        let highlights = IntervalMap::new();
        let views = SlotMap::with_key();
        Self { id, contents, highlights, editors: views }
    }
}
