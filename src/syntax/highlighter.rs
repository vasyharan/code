use tree_sitter as ts;

use super::language::Language;
use crate::rope::Rope;

pub(crate) type Highlights = iset::IntervalMap<usize, String>;

pub(super) fn highlight(contents: &Rope, language: Language, tree: ts::Tree) -> Highlights {
    let _span = tracing::info_span!("highlight").entered();
    let query = ts::Query::new(language.ts, &language.highlight_query).expect("invalid query");
    let mut cursor = ts::QueryCursor::new();
    let mut highlights = iset::IntervalMap::new();
    let captures = cursor.captures(&query, tree.root_node(), super::RopeTextProvider(&contents));
    for (query_match, _) in captures {
        for capture in query_match.captures {
            let capture_name = &query.capture_names()[capture.index as usize];
            highlights.insert(capture.node.byte_range(), capture_name.clone());
        }
    }
    highlights
}
