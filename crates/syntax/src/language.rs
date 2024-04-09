use anyhow::Result;
use tree_sitter as ts;

use editor::Buffer;

#[derive(Debug)]
pub struct Language {
    pub ts: ts::Language,
    pub highlight_query: String,
}

impl TryFrom<&Buffer> for Language {
    type Error = anyhow::Error;

    fn try_from(_: &Buffer) -> Result<Self> {
        Ok(Language {
            ts: tree_sitter_rust::language(),
            highlight_query: tree_sitter_rust::HIGHLIGHT_QUERY.into(),
        })
    }
}
