const std = @import("std");
const Allocator = std.mem.Allocator;

const Error = error{EOS};

const StackError = error{Empty};

const PositionType = enum { byte_offset, line_and_column };
const Position = union(PositionType) {
    byte_offset: usize,
    line_and_column: LineAndColumn,
};
const LineAndColumn = struct {
    line: usize,
    column: usize,
};

const Colour = enum(u1) { red, black };
const NodeType = enum(u8) {
    branch,
    leaf,
};

const Node = union(NodeType) {
    const Self = @This();

    branch: BranchNode,
    leaf: LeafNode,

    fn ref(self: *Self) void {
        switch (self.*) {
            .leaf => |*leaf| leaf.ref(),
            .branch => |*branch| branch.ref(),
        }
    }

    fn deref(self: *Self, allocator: Allocator) void {
        switch (self.*) {
            .leaf => |*leaf| leaf.deref(allocator),
            .branch => |*branch| branch.deref(allocator),
        }
    }

    fn len(self: Self) usize {
        return switch (self) {
            .leaf => |leaf| leaf.len,
            .branch => |branch| branch.len,
        };
    }
};

const LeafNodePath = struct {
    const Self = @This();

    parents: BranchNodeStack,
    leaf: *LeafNode,
    offset: usize,

    fn deinit(self: *Self, allocator: Allocator) void {
        self.parents.deinit(allocator);
    }

    fn getParent(self: Self) ?*BranchNode {
        if (self.parents) |parents| return parents.item;
        return null;
    }

    fn getGrandparent(self: Self) ?*BranchNode {
        if (self.parents) |parents| {
            if (parents.next) |grandparents| {
                return grandparents.item;
            }
        }
        return null;
    }
};

const BranchNodeStackNode = struct {
    const Self = @This();

    item: *BranchNode,
    next: ?*BranchNodeStackNode,
    prev: ?*BranchNodeStackNode,

    fn init(allocator: Allocator, item: *BranchNode) *Self {
        const self = allocator.create(Self) catch @panic("oom");
        self.* = .{ .item = item, .next = null, .prev = null };
        return self;
    }
};

const BranchNodeStack = struct {
    const Self = @This();

    head: ?*BranchNodeStackNode,
    tail: ?*BranchNodeStackNode,

    fn initEmpty() Self {
        return .{
            .head = null,
            .tail = null,
        };
    }

    fn deinit(self: *Self, allocator: Allocator) void {
        while (!self.isEmpty()) _ = self.pop(allocator) catch unreachable;
    }

    fn isEmpty(self: Self) bool {
        return self.head == null;
    }

    fn push(self: *Self, allocator: Allocator, item: *BranchNode) void {
        const node = BranchNodeStackNode.init(allocator, item);
        if (self.tail) |tail| {
            tail.next = node;
            node.prev = tail;
            self.tail = node;
        } else {
            std.debug.assert(self.head == null);
            self.head = node;
            self.tail = node;
        }
    }

    fn pop(self: *Self, allocator: Allocator) StackError!*BranchNode {
        if (self.tail == null) {
            std.debug.assert(self.head == null);
            return StackError.Empty;
        }
        const tail = self.tail.?;
        const item = tail.item;
        const prev = tail.prev;
        allocator.destroy(tail);
        self.tail = prev;
        if (self.tail) |new_tail| {
            new_tail.next = null;
        } else {
            self.head = null;
        }
        return item;
    }

    fn popN(self: *Self, allocator: Allocator, n: usize) StackError!void {
        for (0..n) |_| {
            _ = try self.pop(allocator);
        }
    }

    fn peek(self: Self) ?*BranchNode {
        return self.peekNth(0);
    }

    fn peekNth(self: Self, n: usize) ?*BranchNode {
        var stack = self.tail;
        for (0..n) |_| {
            if (stack) |s|
                stack = s.prev
            else
                break;
        }
        return if (stack) |s| s.item else null;
    }

    fn concat(self: *Self, _: Allocator, other: *Self) void {
        var maybe_head = other.head;
        while (maybe_head) |node| {
            if (self.tail) |tail| {
                tail.next = node;
                node.prev = tail;
                self.tail = node;
            } else {
                std.debug.assert(self.head == null);
                self.head = node;
                self.tail = node;
            }
            maybe_head = node.next;
        }
    }
};

const BranchNode = struct {
    const Self = @This();

    colour: Colour,
    ref_count: u16,
    len: usize,

    left: *Node,
    right: *Node,

    fn init(allocator: Allocator, left: *Node, right: *Node) *Node {
        const self = allocator.create(Node) catch @panic("oom");
        self.* = Node{
            .branch = BranchNode{
                .left = left,
                .right = right,
                .colour = .red,
                .len = 0,
                .ref_count = 0,
            },
        };
        left.ref();
        self.branch.len += left.len();
        right.ref();
        self.branch.len += right.len();
        return self;
    }

    fn deinit(self: *Self, allocator: Allocator) void {
        self.left.deref(allocator);
        self.right.deref(allocator);
        allocator.destroy(self.getNode());
    }

    fn ref(self: *Self) void {
        self.ref_count += 1;
    }

    fn deref(self: *Self, allocator: Allocator) void {
        self.ref_count -= 1;
        if (self.ref_count == 0)
            self.deinit(allocator);
    }

    fn isRoot(self: Self) bool {
        return self.getParent() == null;
    }

    fn getColour(self: Self) Colour {
        return self.colour;
    }

    fn setColour(self: *Self, colour: Colour) void {
        self.colour = colour;
    }

    fn replaceLeft(self: Self, allocator: Allocator, left: *Node) *Node {
        const replaced = allocator.create(Node) catch @panic("oom");
        replaced.* = Node{
            .branch = .{
                .left = left,
                .right = self.right,
                .colour = self.getColour(),
                .len = 0,
                .ref_count = 0,
            },
        };
        left.ref();
        replaced.branch.len = left.len();
        self.right.ref();
        replaced.branch.len += self.right.len();
        return replaced;
    }

    fn replaceRight(self: Self, allocator: Allocator, right: *Node) *Node {
        const replaced = allocator.create(Node) catch @panic("oom");
        replaced.* = Node{
            .branch = .{
                .left = self.left,
                .right = right,
                .colour = self.getColour(),
                .len = 0,
                .ref_count = 0,
            },
        };
        right.ref();
        replaced.branch.len = right.len();
        self.left.ref();
        replaced.branch.len += self.left.len();
        return replaced;
    }

    pub fn format(self: Self, comptime _: []const u8, _: std.fmt.FormatOptions, writer: anytype) std.os.WriteError!void {
        try writer.print(
            "rope.BranchNode{{ .colour = {}, .ref_count = {d}, .len = {d}",
            .{ self.getColour(), self.ref_count, self.len },
        );
        switch (self.left.*) {
            .leaf => |leaf| try writer.print(", .left = {}", .{leaf}),
            .branch => |branch| try writer.print(", .left = {}", .{branch}),
        }
        switch (self.right.*) {
            .leaf => |leaf| try writer.print(", .right = {},", .{leaf}),
            .branch => |branch| try writer.print(", .right = {},", .{branch}),
        }
        try writer.print(" }}", .{});
    }

    fn getNode(self: *Self) *Node {
        return @fieldParentPtr(Node, "branch", self);
    }
};

const LeafNode = struct {
    const Self = @This();

    ref_count: u16,
    len: usize,
    val: []const u8,

    fn initEmpty(allocator: Allocator) *Node {
        return LeafNode.init(allocator, "");
    }

    fn init(allocator: Allocator, val: []const u8) *Node {
        const self = allocator.create(Node) catch @panic("oom");
        self.* = Node{
            .leaf = LeafNode{
                .val = val,
                .len = val.len,
                .ref_count = 0,
            },
        };
        return self;
    }

    fn deinit(self: *Self, allocator: Allocator) void {
        allocator.destroy(self.getNode());
    }

    fn ref(self: *Self) void {
        self.ref_count += 1;
    }

    fn deref(self: *Self, allocator: Allocator) void {
        self.ref_count -= 1;
        if (self.ref_count == 0)
            self.deinit(allocator);
    }

    fn isRoot(self: Self) bool {
        return self.getParent() == null;
    }

    fn slice(self: Self, allocator: Allocator, start: usize, end: usize) *Node {
        return LeafNode.init(allocator, self.val[start..end]);
    }

    pub fn format(self: Self, comptime _: []const u8, _: std.fmt.FormatOptions, writer: anytype) std.os.WriteError!void {
        return writer.print(
            "rope.LeafNode{{ .ref_count = {d}, .len = {d}, .val = {s} }}",
            .{ self.ref_count, self.len, self.val },
        );
    }

    fn getNode(self: *Self) *Node {
        return @fieldParentPtr(Node, "leaf", self);
    }
};

const Rope = struct {
    const Self = @This();

    allocator: Allocator,
    root: *Node,

    fn initEmpty(allocator: Allocator) Self {
        const root = LeafNode.initEmpty(allocator);
        return Rope.initNode(allocator, root);
    }

    fn initNode(allocator: Allocator, node: *Node) Self {
        node.ref();
        return Self{ .allocator = allocator, .root = node };
    }

    fn deinit(self: Self) void {
        self.root.deref(self.allocator);
    }

    pub fn cursor(self: Self) Cursor {
        return Cursor.init(self);
    }

    pub fn len(self: Self) usize {
        return self.root.len();
    }

    pub fn insertAt(self: Self, pos: Position, text: []const u8) !Rope {
        if (text.len == 0) {
            return Rope.initNode(self.allocator, self.root);
        }

        const leaf_path = try getLeafNodeAtPosition(self.allocator, self.root, pos);
        if (leaf_path.leaf.val.len == 0) {
            const new_leaf_node = LeafNode.init(self.allocator, text);
            return Rope.initNode(self.allocator, new_leaf_node);
        }

        // create a new branch node, to insert the new text into.
        const new_branch_left = leaf_path.leaf.slice(self.allocator, 0, leaf_path.offset);
        const new_branch_right = LeafNode.init(self.allocator, text);
        const new_branch = BranchNode.init(
            self.allocator,
            new_branch_left,
            new_branch_right,
        );

        // balance the newly inserted node and update
        // the new node's path to the root (ancestors)
        const new_root = self.balance(&new_branch.branch, leaf_path);
        return Rope.initNode(self.allocator, new_root);
    }

    fn isBalanced(self: Self) bool {
        return switch (self.root.*) {
            .leaf => true,
            .branch => isNodeBalanced(self.root, 0).balanced,
        };
    }

    fn isNodeBalanced(maybe_node: ?*Node, black_depth: usize) struct { balanced: bool, black_height: usize = 0 } {
        if (maybe_node == null)
            return .{ .balanced = true, .black_height = black_depth };

        const node = maybe_node orelse unreachable;
        switch (node.*) {
            .leaf => return .{ .balanced = true, .black_height = black_depth },
            .branch => |*branch| {
                const unbalanced = .{ .balanced = false };
                if (branch.getColour() == .red) {
                    if (@as(NodeType, branch.left.*) == NodeType.branch and
                        branch.left.branch.getColour() == .red) return unbalanced;
                    if (@as(NodeType, branch.right.*) == NodeType.branch and
                        branch.right.branch.getColour() == .red) return unbalanced;
                }

                const next_black_depth = black_depth + @as(usize, if (branch.getColour() == .black) 1 else 0);
                const left_result = isNodeBalanced(branch.left, next_black_depth);
                const right_result = isNodeBalanced(branch.right, next_black_depth);

                if (std.meta.eql(left_result, right_result))
                    return left_result
                else
                    return unbalanced;
            },
        }
    }

    fn balance(self: Self, new_branch_node: *BranchNode, leaf_path: LeafNodePath) *Node {
        var new_node: *BranchNode = new_branch_node;
        var old_node: *Node = leaf_path.leaf.getNode();
        var parent_stack = leaf_path.parents;
        std.debug.assert(new_node.getColour() == .red);

        while (parent_stack.peekNth(0)) |parent| {
            if (parent.getColour() != .red) break;

            const maybe_grandparent = parent_stack.peekNth(1);
            if (maybe_grandparent == null) break;
            const grandparent = maybe_grandparent.?;
            if (grandparent.getColour() != .black) break;

            const parent_node = parent.getNode();
            if (grandparent.left == parent_node and parent.left == old_node) {
                // case 1
                unreachable; // only ever append to the right.
            } else if (grandparent.left == parent_node and parent.right == old_node) {
                // case2
                unreachable; // only ever append to the right.
            } else if (grandparent.right == parent_node and parent.left == old_node) {
                // case3
                unreachable; // unimplemented
            } else if (grandparent.right == parent_node and parent.right == old_node) {
                // case 4
                const new_left_branch = BranchNode.init(
                    self.allocator,
                    grandparent.left,
                    parent.left,
                );
                new_left_branch.branch.setColour(.black);
                const new_branch = BranchNode.init(
                    self.allocator,
                    new_left_branch,
                    new_node.getNode(),
                );
                new_node.setColour(.black);

                new_node = &new_branch.branch;
                old_node = grandparent.getNode();
                parent_stack.popN(self.allocator, 2) catch unreachable;
            }
        }

        while (parent_stack.peekNth(0)) |old_parent| {
            const new_parent: *Node = if (old_parent.left == old_node)
                old_parent.replaceLeft(self.allocator, new_node.getNode())
            else if (old_parent.right == old_node)
                old_parent.replaceRight(self.allocator, new_node.getNode())
            else
                unreachable;

            new_node = &new_parent.branch;
            old_node = old_parent.getNode();
            _ = parent_stack.pop(self.allocator) catch unreachable;
        }

        new_node.setColour(.black);
        return new_node.getNode();
    }

    pub fn format(self: Self, comptime _: []const u8, _: std.fmt.FormatOptions, writer: anytype) std.os.WriteError!void {
        return switch (self.root.*) {
            .leaf => |leaf| writer.print("rope.Rope{{ .root = {} }}", .{leaf}),
            .branch => |branch| writer.print("rope.Rope{{ .root = {} }}", .{branch}),
        };
    }
};

const Cursor = struct {
    const Self = @This();

    rope: Rope,
    leaf_path: ?LeafNodePath,

    fn init(rope: Rope) Self {
        return Self{
            .rope = rope,
            .leaf_path = getLeftMostLeafNode(rope.allocator, rope.root),
        };
    }

    fn deinit(self: *Self) void {
        if (self.leaf_path) |*leaf_path| leaf_path.deinit(self.rope.allocator);
    }

    fn next(self: *Self, maxlen: u32) ?[]const u8 {
        if (self.leaf_path) |*leaf_path| {
            const leaf = leaf_path.leaf;
            const from = leaf_path.offset;
            const to = @min(from + maxlen, leaf.len);
            if (leaf_path.leaf.len > to) {
                // stay on the same node.
                leaf_path.offset = to;
            } else {
                self.leaf_path = getNextLeafNode(self.rope.allocator, leaf_path.*);
            }
            return leaf.val[from..to];
        }
        return null;
    }
};

fn getLeafNodeAtPosition(allocator: Allocator, root: *Node, pos: Position) !LeafNodePath {
    return switch (pos) {
        .byte_offset => |byte_offset| getLeafNodeAtByteOffset(allocator, root, byte_offset),
        .line_and_column => unreachable, //|p| leafNodeAtLineAndColumn(root, p.line, p.column),
    };
}

fn getLeafNodeAtByteOffset(allocator: Allocator, root: *Node, byte_offset: usize) !LeafNodePath {
    var node: *Node = root;
    var offset = byte_offset;
    var parents = BranchNodeStack.initEmpty();
    while (true) {
        if (offset > node.len()) return Error.EOS;
        switch (node.*) {
            .leaf => |*leaf| {
                return .{ .parents = parents, .leaf = leaf, .offset = offset };
            },
            .branch => |*branch| {
                parents.push(allocator, branch);
                const left_len = branch.left.len();
                if (left_len > offset) {
                    node = branch.left;
                } else {
                    offset -= left_len;
                    node = branch.right;
                }
                continue;
            },
        }
        unreachable;
    }
    unreachable;
}

fn getLeftMostLeafNode(allocator: Allocator, from_node: *Node) LeafNodePath {
    var maybe_node: ?*Node = from_node;
    var parents = BranchNodeStack.initEmpty();
    while (maybe_node) |node| {
        switch (node.*) {
            .branch => |*branch| {
                parents.push(allocator, branch);
                maybe_node = branch.left;
            },
            .leaf => |*leaf| {
                return .{ .parents = parents, .leaf = leaf, .offset = 0 };
            },
        }
    }
    unreachable;
}

fn getNextLeafNode(allocator: Allocator, from_leaf_path: LeafNodePath) ?LeafNodePath {
    var from_leaf = from_leaf_path.leaf;
    var parents = from_leaf_path.parents;
    var search_node: ?*Node = from_leaf.getNode();

    while (!parents.isEmpty()) {
        const parent = parents.peekNth(0).?;
        if (parent.left == search_node) {
            var nlp = getLeftMostLeafNode(allocator, parent.right);
            parents.concat(allocator, &nlp.parents);
            return .{ .leaf = nlp.leaf, .offset = nlp.offset, .parents = parents };
        } else if (parent.right == search_node) {
            _ = parents.pop(allocator) catch unreachable;
            search_node = parent.getNode();
        } else {
            return null;
        }
    }
    return null;
}

const parts = [_][]const u8{
    "Lorem ", // 1
    "ipsum ", // 2
    "dolor ", // 3
    "sit ", // 4
    "amet ", // 5
    "consectetur ", // 6
    "adipiscing ", // 7
    "elit ", // 8
    "sed ", // 9
    "do ", // 10
    "eiusmod ", // 11
    "tempor ", // 12
    "incididunt ", // 13
    "ut ", // 14
    "labore ", // 15
    "et ", // 16
    "dolore ", // 17
    "magna ", // 18
    "aliqua", // 19
};

test "Rope basic tests" {
    std.debug.print("\n", .{});

    const allocator = std.testing.allocator;
    const rope0 = Rope.initEmpty(allocator);
    std.debug.print("rope0 = {}\n", .{rope0});

    var prev_rope = rope0;
    // for (parts, 0..) |part, i| {
    for (parts) |part| {
        const rope = try prev_rope.insertAt(.{ .byte_offset = prev_rope.len() }, part);
        prev_rope.deinit();
        // std.debug.print("rope{} = {}\n", .{ i + 1, rope });
        try std.testing.expect(rope.isBalanced());
        prev_rope = rope;
    }
    prev_rope.deinit();
}

test "Cursor basic tests" {
    std.debug.print("\n", .{});

    const allocator = std.testing.allocator;
    var rope = Rope.initEmpty(allocator);
    for (parts) |part| {
        const new_rope = try rope.insertAt(.{ .byte_offset = rope.len() }, part);
        try std.testing.expect(new_rope.isBalanced());
        rope.deinit();
        rope = new_rope;
    }

    defer rope.deinit();
    var cursor = rope.cursor();
    defer cursor.deinit();
    for (parts) |part| {
        // std.debug.print("{s}", .{part});
        try std.testing.expectEqualStrings(part, cursor.next(32).?);
    }
}
