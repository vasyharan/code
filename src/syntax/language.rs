use tree_sitter as ts;

use crate::buffer::Buffer;

#[derive(Debug)]
pub(crate) enum Error {
    // UnsupportedLanguage,
}

#[derive(Debug)]
pub(crate) struct Language {
    pub(crate) ts: ts::Language,
    pub(crate) highlight_query: String,
}

impl TryFrom<&Buffer> for Language {
    type Error = self::Error;

    fn try_from(_: &Buffer) -> Result<Self, Self::Error> {
        Ok(Language {
            ts: tree_sitter_rust::language(),
            highlight_query: tree_sitter_rust::HIGHLIGHT_QUERY.into(),
        })
    }
}
