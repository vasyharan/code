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

    fn lenBytes(self: Self) usize {
        return switch (self) {
            .leaf => |leaf| leaf.len_bytes,
            .branch => |branch| branch.len_bytes,
        };
    }
};

const LeafNodePath = struct {
    const Self = @This();

    parents: BranchNodeStack,
    leaf: *LeafNode,
    offset: usize,

    fn deinit(self: *Self) void {
        self.parents.deinit();
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

pub fn Deque(comptime T: type) type {
    const StackNode = struct {
        const Self = @This();

        item: T,
        next: ?*Self,
        prev: ?*Self,

        fn init(allocator: Allocator, item: T) *Self {
            const self = allocator.create(Self) catch @panic("oom");
            self.* = .{ .item = item, .next = null, .prev = null };
            return self;
        }
    };

    return struct {
        const Self = @This();

        allocator: Allocator,
        head: ?*StackNode,
        tail: ?*StackNode,

        fn initEmpty(allocator: Allocator) Self {
            return .{
                .allocator = allocator,
                .head = null,
                .tail = null,
            };
        }

        fn deinit(self: *Self) void {
            while (!self.isEmpty()) _ = self.pop() catch unreachable;
        }

        fn isEmpty(self: Self) bool {
            return self.head == null;
        }

        fn push(self: *Self, item: T) void {
            const node = StackNode.init(self.allocator, item);
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

        fn pop(self: *Self) StackError!T {
            if (self.tail == null) {
                std.debug.assert(self.head == null);
                return StackError.Empty;
            }
            const tail = self.tail.?;
            const item = tail.item;
            const prev = tail.prev;
            self.allocator.destroy(tail);
            self.tail = prev;
            if (self.tail) |new_tail| {
                new_tail.next = null;
            } else {
                self.head = null;
            }
            return item;
        }

        fn popN(self: *Self, n: usize) StackError!void {
            for (0..n) |_| {
                _ = try self.pop();
            }
        }

        fn peek(self: Self) ?T {
            return self.peekNth(0);
        }

        fn peekNth(self: Self, n: usize) ?T {
            var stack = self.tail;
            for (0..n) |_| {
                if (stack) |s|
                    stack = s.prev
                else
                    break;
            }
            return if (stack) |s| s.item else null;
        }

        fn append(self: *Self, other: *Self) void {
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
}

const BranchNodeStack = Deque(*BranchNode);

const BranchNode = struct {
    const Self = @This();

    colour: Colour,
    ref_count: u16,
    len_bytes: usize,

    left: *Node,
    right: *Node,

    fn init(allocator: Allocator, left: *Node, right: *Node) *Node {
        const self = allocator.create(Node) catch @panic("oom");
        self.* = Node{
            .branch = BranchNode{
                .left = left,
                .right = right,
                .colour = .red,
                .len_bytes = 0,
                .ref_count = 0,
            },
        };
        left.ref();
        self.branch.len_bytes += left.lenBytes();
        right.ref();
        self.branch.len_bytes += right.lenBytes();
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
                .len_bytes = 0,
                .ref_count = 0,
            },
        };
        left.ref();
        replaced.branch.len_bytes = left.lenBytes();
        self.right.ref();
        replaced.branch.len_bytes += self.right.lenBytes();
        return replaced;
    }

    fn replaceRight(self: Self, allocator: Allocator, right: *Node) *Node {
        const replaced = allocator.create(Node) catch @panic("oom");
        replaced.* = Node{
            .branch = .{
                .left = self.left,
                .right = right,
                .colour = self.getColour(),
                .len_bytes = 0,
                .ref_count = 0,
            },
        };
        right.ref();
        replaced.branch.len_bytes = right.lenBytes();
        self.left.ref();
        replaced.branch.len_bytes += self.left.lenBytes();
        return replaced;
    }

    pub fn format(self: Self, comptime _: []const u8, _: std.fmt.FormatOptions, writer: anytype) std.os.WriteError!void {
        try writer.print(
            "rope.BranchNode{{ .colour = {}, .ref_count = {d}, .len_bytes = {d}",
            .{ self.getColour(), self.ref_count, self.len_bytes },
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
    len_bytes: usize,
    val: []const u8,

    fn initEmpty(allocator: Allocator) *Node {
        return LeafNode.init(allocator, "");
    }

    fn init(allocator: Allocator, val: []const u8) *Node {
        const self = allocator.create(Node) catch @panic("oom");
        self.* = Node{
            .leaf = LeafNode{
                .val = val,
                .len_bytes = val.len,
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
            "rope.LeafNode{{ .ref_count = {d}, .len_bytes = {d}, .val = {s} }}",
            .{ self.ref_count, self.len_bytes, self.val },
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

    pub fn clone(self: Self) Self {
        return Rope.initNode(self.allocator, self.root);
    }

    pub fn cursor(self: Self) Cursor {
        return Cursor.init(self);
    }

    pub fn lenBytes(self: Self) usize {
        return self.root.lenBytes();
    }

    pub fn insertAt(self: Self, pos: Position, text: []const u8) !Rope {
        if (text.len == 0) {
            return Rope.initNode(self.allocator, self.root);
        }

        const leaf_path = try getLeafNodeAtPosition(self.allocator, self.root, pos);
        if (leaf_path.leaf.len_bytes == 0) {
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
        const new_root = self.balance(&new_branch.branch, leaf_path.leaf, leaf_path.parents);
        return Rope.initNode(self.allocator, new_root);
    }

    pub fn deleteAt(self: Self, pos: Position, len: usize) !Self {
        if (len == 0) {
            return Rope.initNode(self.allocator, self.root);
        }

        var remaining_len = len;
        var leaf_path = try getLeafNodeAtPosition(self.allocator, self.root, pos);
        // loop until the requested number of bytes have been deleted.
        while (remaining_len > 0) {
            const leaf_path_len_bytes = leaf_path.leaf.len_bytes - leaf_path.offset;
            if (leaf_path_len_bytes > remaining_len or (leaf_path_len_bytes == remaining_len and leaf_path.offset > 0)) {
                // handle deleting a leaf node val without needing to remove the leaf node.
                return self.deleteLeaf(leaf_path.leaf, leaf_path.parents, leaf_path.offset, leaf_path.offset + remaining_len);
            }

            unreachable;
        }
        unreachable;
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

    fn deleteLeaf(self: Self, leaf: *LeafNode, path: BranchNodeStack, from: usize, to: usize) Self {
        std.debug.assert(from >= 0 and from <= leaf.len_bytes and from < to);
        std.debug.assert(to >= 0 and to <= leaf.len_bytes and to > from);
        std.debug.assert(to - from < leaf.len_bytes);
        var parents = path;
        if (from > 0 and to < leaf.len_bytes) {
            // case 1: delete the middle of the node's val; eg. "a(bcdef)g".
            //      replace the leaf node with a branch node, with the left
            //      branch including the contents before ("a") and the right
            //      branch including the contents after ("g")
            const new_branch_left = leaf.slice(self.allocator, 0, from);
            const new_branch_right = leaf.slice(self.allocator, to, leaf.len_bytes);
            const new_branch = BranchNode.init(self.allocator, new_branch_left, new_branch_right);

            // balance the newly inserted node and update
            // the new node's path to the root (ancestors)
            const new_root = self.balance(&new_branch.branch, leaf, parents);
            return Rope.initNode(self.allocator, new_root);
        }

        var old_node = leaf.getNode();
        var new_node = if (from == 0 and to < leaf.len_bytes)
            // case 2: delete the end of the node's val; eg. "a(bcdefg)".
            //      replace the leaf node with a new leaf node with just the
            //      contents before ("a")
            LeafNode.init(self.allocator, leaf.val[to..])
        else if (from > 0 and to == leaf.len_bytes)
            // case 3: delete the start of the node's val; eg. "(abcdef)g".
            //      replace the leaf node with a new leaf node with just the
            //      contents after ("g")
            LeafNode.init(self.allocator, leaf.val[from..to])
        else
            unreachable;

        while (parents.peekNth(0)) |old_parent| {
            new_node = if (old_parent.left == old_node)
                old_parent.replaceLeft(self.allocator, new_node)
            else if (old_parent.right == old_node)
                old_parent.replaceRight(self.allocator, new_node)
            else
                unreachable;

            old_node = old_parent.getNode();
            _ = parents.pop() catch unreachable;
        }

        return Rope.initNode(self.allocator, new_node);
    }

    fn balance(self: Self, new_branch_node: *BranchNode, leaf: *LeafNode, path: BranchNodeStack) *Node {
        var new_node: *BranchNode = new_branch_node;
        var old_node: *Node = leaf.getNode();
        var parents = path;
        std.debug.assert(new_node.getColour() == .red);

        while (parents.peekNth(0)) |parent| {
            if (parent.getColour() != .red) break;

            const maybe_grandparent = parents.peekNth(1);
            if (maybe_grandparent == null) break;
            const grandparent = maybe_grandparent.?;
            if (grandparent.getColour() != .black) break;

            const parent_node = parent.getNode();
            if (grandparent.left == parent_node and parent.left == old_node) {
                // case 1
                unreachable; // only ever append to the right.
            } else if (grandparent.left == parent_node and parent.right == old_node) {
                // case2
                const new_left_branch = BranchNode.init(
                    self.allocator,
                    parent.left,
                    new_node.left,
                );
                new_left_branch.branch.setColour(.black);
                const new_right_branch = BranchNode.init(
                    self.allocator,
                    new_node.right,
                    grandparent.right,
                );
                new_right_branch.branch.setColour(.black);
                const new_branch = BranchNode.init(
                    self.allocator,
                    new_left_branch,
                    new_right_branch,
                );

                new_node = &new_branch.branch;
                old_node = grandparent.getNode();
                parents.popN(2) catch unreachable;
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
                parents.popN(2) catch unreachable;
            }
        }

        while (parents.peekNth(0)) |old_parent| {
            const new_parent: *Node = if (old_parent.left == old_node)
                old_parent.replaceLeft(self.allocator, new_node.getNode())
            else if (old_parent.right == old_node)
                old_parent.replaceRight(self.allocator, new_node.getNode())
            else
                unreachable;

            new_node = &new_parent.branch;
            old_node = old_parent.getNode();
            _ = parents.pop() catch unreachable;
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

    pub fn writeDot(self: Self, writer: anytype) !void {
        try writer.print("digraph {{\n", .{});
        switch (self.root.*) {
            .leaf => |*leaf| try writeLeafDot(leaf, writer),
            .branch => |*branch| try writeBranchDot(branch, writer),
        }
        try writer.print("}}", .{});
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
        if (self.leaf_path) |*leaf_path| leaf_path.deinit();
    }

    fn next(self: *Self, maxlen: u32) ?[]const u8 {
        if (self.leaf_path) |*leaf_path| {
            const leaf = leaf_path.leaf;
            const from = leaf_path.offset;
            const to = @min(from + maxlen, leaf.len_bytes);
            if (leaf_path.leaf.len_bytes > to) {
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
    var parents = BranchNodeStack.initEmpty(allocator);
    while (true) {
        if (offset > node.lenBytes()) return Error.EOS;
        switch (node.*) {
            .leaf => |*leaf| {
                return .{ .parents = parents, .leaf = leaf, .offset = offset };
            },
            .branch => |*branch| {
                parents.push(branch);
                const left_len_bytes = branch.left.lenBytes();
                if (left_len_bytes > offset) {
                    node = branch.left;
                } else {
                    offset -= left_len_bytes;
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
    var parents = BranchNodeStack.initEmpty(allocator);
    while (maybe_node) |node| {
        switch (node.*) {
            .branch => |*branch| {
                parents.push(branch);
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
            parents.append(&nlp.parents);
            return .{ .leaf = nlp.leaf, .offset = nlp.offset, .parents = parents };
        } else if (parent.right == search_node) {
            _ = parents.pop() catch unreachable;
            search_node = parent.getNode();
        } else {
            return null;
        }
    }
    return null;
}

fn writeLeafDot(leaf: *const LeafNode, writer: anytype) !void {
    try writer.print("\tn{x}[shape=square,label=\"'{s}'\"];\n", .{ @intFromPtr(leaf), leaf.val });
}

fn writeBranchDot(branch: *const BranchNode, writer: anytype) !void {
    // try writer.print("\tn{x}[shape=circle,color={s},label=\"{x} ({})\"];\n", .{
    //     @intFromPtr(branch),
    //     @tagName(branch.getColour()),
    //     @intFromPtr(branch) & 0xffffff,
    //     branch.ref_count,
    // });
    try writer.print("\tn{x}[shape=circle,color={s},label=\"\"];\n", .{ @intFromPtr(branch), @tagName(branch.getColour()) });
    switch (branch.left.*) {
        .leaf => |*left| {
            try writeLeafDot(left, writer);
            try writer.print("\tn{x} -> n{x} [label=\"{}\"];\n", .{ @intFromPtr(branch), @intFromPtr(left), left.len_bytes });
        },
        .branch => |*left| {
            try writeBranchDot(left, writer);
            try writer.print("\tn{x} -> n{x} [label=\"{}\"];\n", .{ @intFromPtr(branch), @intFromPtr(left), left.len_bytes });
        },
    }
    switch (branch.right.*) {
        .leaf => |*right| {
            try writeLeafDot(right, writer);
            try writer.print("\tn{x} -> n{x} [label=\"{}\"];\n", .{ @intFromPtr(branch), @intFromPtr(right), right.len_bytes });
        },
        .branch => |*right| {
            try writeBranchDot(right, writer);
            try writer.print("\tn{x} -> n{x} [label=\"{}\"];\n", .{ @intFromPtr(branch), @intFromPtr(right), right.len_bytes });
        },
    }
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
    const RopeStack = Deque(Rope);
    const allocator = std.testing.allocator;

    var rope_stack = RopeStack.initEmpty(allocator);
    defer {
        while (!rope_stack.isEmpty()) {
            const rope = rope_stack.pop() catch unreachable;
            rope.deinit();
        }
    }

    var rope = Rope.initEmpty(allocator);
    rope_stack.push(rope);
    for (parts, 0..) |part, i| {
        rope = try rope.insertAt(.{ .byte_offset = rope.lenBytes() }, part);
        rope_stack.push(rope);

        // std.debug.print("rope{} = {}\n", .{ i + 1, rope });
        var buf: [14]u8 = [_]u8{0} ** 14;
        var filename = try std.fmt.bufPrint(&buf, "tmp/rope{d:0>2}.dot", .{i});
        const f = try std.fs.cwd().createFile(filename, .{});
        defer f.close();
        try rope.writeDot(f.writer());

        try std.testing.expect(rope.isBalanced());
    }

    const file = try std.fs.cwd().createFile("tmp/rope.dot", .{});
    defer file.close();

    rope = try rope.deleteAt(.{ .byte_offset = 2 }, 2);
    rope_stack.push(rope);
    // std.debug.print("rope{} = {}\n\n", .{ i + 1, rope });
    // try rope.writeDot(file.writer());
    try std.testing.expect(rope.isBalanced());

    rope = try rope.deleteAt(.{ .byte_offset = 0 }, 1);
    rope_stack.push(rope);
    // std.debug.print("rope{} = {}\n\n", .{ i + 1, rope });
    // try rope.writeDot(file.writer());
    try std.testing.expect(rope.isBalanced());

    rope = try rope.deleteAt(.{ .byte_offset = 2 }, 1);
    rope_stack.push(rope);
    // std.debug.print("rope{} = {}\n\n", .{ i + 1, rope });
    try rope.writeDot(file.writer());
    try std.testing.expect(rope.isBalanced());

    rope = try rope.deleteAt(.{ .byte_offset = 10 }, 22);
    rope_stack.push(rope);
    // std.debug.print("rope{} = {}\n\n", .{ i + 1, rope });
    try rope.writeDot(file.writer());
    try std.testing.expect(rope.isBalanced());
}

test "Cursor basic tests" {
    std.debug.print("\n", .{});

    const allocator = std.testing.allocator;
    var rope = Rope.initEmpty(allocator);
    for (parts) |part| {
        const new_rope = try rope.insertAt(.{ .byte_offset = rope.lenBytes() }, part);
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
