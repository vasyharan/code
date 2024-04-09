use tree_sitter as ts;

use crate::Language;
use editor::BufferContents;

#[tracing::instrument(skip_all)]
pub fn highlight(
    buffer: &BufferContents,
    language: Language,
    tree: ts::Tree,
) -> editor::Highlights {
    let query = ts::Query::new(language.ts, &language.highlight_query).expect("invalid query");
    let mut cursor = ts::QueryCursor::new();
    let mut highlights = iset::IntervalMap::new();
    let captures =
        cursor.captures(&query, tree.root_node(), crate::BufferContentsTextProvider(buffer));
    for (query_match, _) in captures {
        for capture in query_match.captures {
            let capture_name = &query.capture_names()[capture.index as usize];
            // highlights.insert(capture.node.byte_range(), capture_name.clone());
            let range = capture.node.range();
            let start =
                editor::Point { line: range.start_point.row, column: range.start_point.column };
            let end = editor::Point { line: range.end_point.row, column: range.end_point.column };
            highlights.insert(start..end, capture_name.clone());
        }
    }
    highlights
}
