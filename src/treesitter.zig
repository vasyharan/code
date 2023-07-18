const std = @import("std");
const ts = @cImport(@cInclude("tree_sitter/api.h"));

// tree-sitter language extern defs
extern fn tree_sitter_zig() ?*ts.TSLanguage;

var lang_zig: ?*ts.TSLanguage = null;

pub fn init() void {
    lang_zig = tree_sitter_zig();
}

pub fn langZig() *ts.TSLanguage {
    return lang_zig orelse unreachable;
}

const ParserError = error{ Init, SetLanguage, Parse };

pub const Parser = struct {
    const Self = @This();

    ts_parser: *ts.TSParser = undefined,

    pub fn init(allocator: std.mem.Allocator) ParserError!Self {
        _ = allocator;
        if (ts.ts_parser_new()) |ts_parser| {
            return Self{
                .ts_parser = ts_parser,
            };
        } else {
            return ParserError.Init;
        }
    }

    pub fn deinit(self: *Self) void {
        if (self.ts_parser != undefined) {
            ts.ts_parser_delete(self.ts_parser);
        }
    }

    pub fn setLanguage(self: *Self, lang: *ts.TSLanguage) !void {
        if (!ts.ts_parser_set_language(self.ts_parser, lang)) {
            return ParserError.SetLanguage;
        }
    }

    pub fn parseString(self: *Self, str: []const u8) !Tree {
        if (ts.ts_parser_parse_string(self.ts_parser, null, str.ptr, @intCast(str.len))) |ts_tree| {
            return Tree.init(ts_tree);
        } else {
            return ParserError.Parse;
        }
    }
};

pub const Tree = struct {
    const Self = @This();

    ts_tree: *ts.TSTree,

    pub fn init(ts_tree: *ts.TSTree) !Self {
        return Self{ .ts_tree = ts_tree };
    }

    pub fn deinit(self: *Self) void {
        ts.ts_tree_delete(self.ts_tree);
    }

    pub fn rootNode(self: *Self) Node {
        var ts_node = ts.ts_tree_root_node(self.ts_tree);
        return Node.init(ts_node);
    }

    pub fn copy(self: *Self) Self {
        return Self.init(ts.ts_tree_copy(self.ts_tree));
    }
};

pub const Node = struct {
    const Self = @This();

    ts_node: ts.TSNode,

    pub fn init(ts_node: ts.TSNode) Self {
        return Self{ .ts_node = ts_node };
    }

    pub fn startPoint(self: *Self) ts.TSPoint {
        return ts.ts_node_start_point(self.ts_node);
    }
    pub fn startByte(self: *Self) ts.TSPoint {
        return ts.ts_node_start_byte(self.ts_node);
    }

    pub fn endPoint(self: *Self) ts.TSPoint {
        return ts.ts_node_end_point(self.ts_node);
    }
    pub fn endByte(self: *Self) ts.TSPoint {
        return ts.ts_node_end_byte(self.ts_node);
    }
};

const QueryError = error{Unknown};
pub const Query = struct {
    const Self = @This();

    ts_query: *ts.TSQuery,

    pub fn init(lang: *ts.TSLanguage, q: []const u8) !Self {
        var query_err_offset: u32 = 0;
        var query_err_type: ts.TSQueryError = ts.TSQueryErrorNone;
        var ts_query = ts.ts_query_new(
            lang,
            q.ptr,
            @intCast(q.len),
            &query_err_offset,
            &query_err_type,
        );
        if (query_err_type != ts.TSQueryErrorNone) {
            // TODO: map ts query error types.
            return QueryError.Unknown;
        }
        return Self{ .ts_query = ts_query orelse unreachable };
    }

    pub fn deinit(self: *Self) void {
        ts.ts_query_delete(self.ts_query);
    }

    pub fn captureName(self: *const Self, capture_id: u32) []const u8 {
        var len: u32 = 0;
        var name = ts.ts_query_capture_name_for_id(self.ts_query, capture_id, &len);
        return name[0..len];
    }
};

pub const QueryCursor = struct {
    const Self = @This();

    ts_cursor: *ts.TSQueryCursor,

    pub fn init() Self {
        var ts_cursor = ts.ts_query_cursor_new() orelse unreachable;
        return Self{ .ts_cursor = ts_cursor };
    }

    pub fn deinit(self: *Self) void {
        ts.ts_query_cursor_delete(self.ts_cursor);
    }

    pub fn exec(self: *Self, q: Query, node: Node) void {
        ts.ts_query_cursor_exec(self.ts_cursor, q.ts_query, node.ts_node);
    }

    pub fn nextMatch(self: *Self) ?QueryMatch {
        var ts_match = ts.TSQueryMatch{
            .id = 0,
            .pattern_index = 0,
            .capture_count = 0,
            .captures = null,
        };
        if (ts.ts_query_cursor_next_match(self.ts_cursor, &ts_match)) {
            return QueryMatch.init(&ts_match);
        }
        return null;
    }
};

const QueryMatch = struct {
    const Self = @This();

    ts_match: *ts.TSQueryMatch,

    fn init(ts_match: *ts.TSQueryMatch) Self {
        return Self{ .ts_match = ts_match };
    }

    pub fn id(self: *const Self) u32 {
        return self.ts_match.id;
    }

    pub fn patternIndex(self: *const Self) u32 {
        return self.ts_match.pattern_index;
    }

    pub fn captureCount(self: *const Self) u32 {
        return self.ts_match.capture_count;
    }

    pub fn capture(self: *const Self, i: u32) ?QueryCapture {
        if (i < self.captureCount()) {
            return QueryCapture.init(self.ts_match.captures[i]);
        }
        return null;
    }
};

const QueryCapture = struct {
    const Self = @This();

    ts_capture: ts.TSQueryCapture,

    fn init(ts_capture: ts.TSQueryCapture) Self {
        return Self{ .ts_capture = ts_capture };
    }

    pub fn index(self: *const Self) u32 {
        return self.ts_capture.index;
    }

    pub fn node(self: *const Self) Node {
        return Node{ .ts_node = self.ts_capture.node };
    }

    pub fn name(self: *const Self, q: Query) []const u8 {
        return q.captureName(self.index());
    }
};
