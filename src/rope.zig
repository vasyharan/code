const std = @import("std");
const Allocator = std.mem.Allocator;

const Error = error{EOS};

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

    fn getParent(self: Self) ?*BranchNode {
        return switch (self) {
            .leaf => |leaf| leaf.getParent(),
            .branch => |branch| branch.getParent(),
        };
    }

    fn setParent(self: *Self, parent: ?*BranchNode) void {
        switch (self.*) {
            .leaf => |*leaf| leaf.setParent(parent),
            .branch => |*branch| branch.setParent(parent),
        }
    }
};

const BranchNode = struct {
    const Self = @This();

    parent_and_colour: usize, // parent | colour
    ref_count: u16,
    len: usize,

    left: ?*Node,
    right: ?*Node,

    fn init(allocator: Allocator, left: ?*Node, right: ?*Node) *Node {
        std.debug.assert((left == null and right == null) or (left != null));
        const self = allocator.create(Node) catch @panic("oom");
        self.* = Node{
            .branch = BranchNode{
                .left = left,
                .right = right,
                .parent_and_colour = @intFromEnum(Colour.red),
                .len = 0,
                .ref_count = 0,
            },
        };
        if (left) |n| {
            n.ref();
            n.setParent(&self.branch);
            self.branch.len += n.len();
        }
        if (right) |n| {
            n.ref();
            n.setParent(&self.branch);
            self.branch.len += n.len();
        }
        return self;
    }

    fn deinit(self: *Self, allocator: Allocator) void {
        if (self.left) |left| left.deref(allocator);
        if (self.right) |right| right.deref(allocator);
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

    fn getParent(self: Self) ?*BranchNode {
        const mask: usize = 1;
        comptime {
            std.debug.assert(@alignOf(*Self) >= 2);
        }
        const maybe_ptr = self.parent_and_colour & ~mask;
        return if (maybe_ptr == 0) null else @as(*BranchNode, @ptrFromInt(maybe_ptr));
    }

    fn setParent(self: *Self, parent: ?*BranchNode) void {
        self.parent_and_colour = @intFromPtr(parent) | (self.parent_and_colour & 1);
    }

    fn getColour(self: Self) Colour {
        const colour_int = @as(u1, @intCast(self.parent_and_colour & 1));
        return @as(Colour, @enumFromInt(colour_int));
    }

    fn setColour(self: *Self, colour: Colour) void {
        const mask: usize = 1;
        self.parent_and_colour = (self.parent_and_colour & ~mask) | @intFromEnum(colour);
    }

    fn replaceLeft(self: Self, allocator: Allocator, left: ?*Node) *Node {
        std.debug.assert((left == null and self.right == null) or (left != null));
        const replaced = allocator.create(Node) catch @panic("oom");
        replaced.* = Node{
            .branch = .{
                .left = left,
                .right = self.right,
                .parent_and_colour = @intFromEnum(self.getColour()),
                .len = 0,
                .ref_count = 0,
            },
        };
        if (left) |n| {
            n.ref();
            replaced.branch.len = n.len();
        }
        if (self.right) |n| {
            n.ref();
            replaced.branch.len += n.len();
        }
        return replaced;
    }

    fn replaceRight(self: Self, allocator: Allocator, right: ?*Node) *Node {
        std.debug.assert((self.left == null and right == null) or (self.left != null));
        const replaced = allocator.create(Node) catch @panic("oom");
        replaced.* = Node{
            .branch = .{
                .left = self.left,
                .right = right,
                .parent_and_colour = @intFromEnum(self.getColour()),
                .len = 0,
                .ref_count = 0,
            },
        };
        if (right) |n| {
            n.ref();
            replaced.branch.len = n.len();
        }
        if (self.left) |n| {
            n.ref();
            replaced.branch.len += n.len();
        }
        return replaced;
    }

    pub fn format(self: Self, comptime _: []const u8, _: std.fmt.FormatOptions, writer: anytype) std.os.WriteError!void {
        try writer.print(
            "rope.BranchNode{{ .colour = {}, .ref_count = {d}, .len = {d}",
            .{ self.getColour(), self.ref_count, self.len },
        );
        if (self.left) |left| {
            switch (left.*) {
                .leaf => |leaf| try writer.print(", .left = {}", .{leaf}),
                .branch => |branch| try writer.print(", .left = {}", .{branch}),
            }
        }
        if (self.right) |right| {
            switch (right.*) {
                .leaf => |leaf| try writer.print(", .right = {}", .{leaf}),
                .branch => |branch| try writer.print(", .right = {}", .{branch}),
            }
        }
        try writer.print(" }}", .{});
    }

    fn getNode(self: *Self) *Node {
        return @fieldParentPtr(Node, "branch", self);
    }
};

const LeafNode = struct {
    const Self = @This();

    parent_and_colour: usize, // parent | colour
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
                .parent_and_colour = @intFromEnum(Colour.black),
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

    fn getParent(self: Self) ?*BranchNode {
        const mask: usize = 1;
        comptime {
            std.debug.assert(@alignOf(*Self) >= 2);
        }
        const maybe_ptr = self.parent_and_colour & ~mask;
        return if (maybe_ptr == 0) null else @as(*BranchNode, @ptrFromInt(maybe_ptr));
    }

    fn setParent(self: *Self, parent: ?*BranchNode) void {
        self.parent_and_colour = @intFromPtr(parent) | (self.parent_and_colour & 1);
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

    fn cursor(self: Self) Cursor {
        return Cursor.init(self);
    }

    fn len(self: Self) usize {
        return self.root.len();
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
            .branch => |branch| {
                const unbalanced = .{ .balanced = false };
                if (branch.getColour() == .red) {
                    if (branch.left) |left|
                        if (@as(NodeType, left.*) == NodeType.branch and
                            left.branch.getColour() == .red) return unbalanced;
                    if (branch.right) |right|
                        if (@as(NodeType, right.*) == NodeType.branch and
                            right.branch.getColour() == .red) return unbalanced;
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

    fn insertAt(self: Self, pos: Position, text: []const u8) !Rope {
        var leaf_offset: usize = 0;
        const leaf = try leafNodeAt(self.root, pos, &leaf_offset);
        if (text.len == 0) {
            return Rope.initNode(self.allocator, self.root);
        }

        if (leaf.val.len == 0) {
            const new_leaf_node = LeafNode.init(self.allocator, text);
            return Rope.initNode(self.allocator, new_leaf_node);
        }

        // create a new branch node, to insert the new text into.
        const new_branch_left = leaf.slice(self.allocator, 0, leaf_offset);
        const new_branch_right = LeafNode.init(self.allocator, text);
        const new_branch = BranchNode.init(
            self.allocator,
            new_branch_left,
            new_branch_right,
        );

        // balance the newly inserted node and update
        // the new node's path to the root (ancestors)
        const new_root = self.balance(&new_branch.branch, leaf);
        return Rope.initNode(self.allocator, new_root);
    }

    fn balance(self: Self, new_branch_node: *BranchNode, old_leaf_node: *LeafNode) *Node {
        var new_node: *BranchNode = new_branch_node;
        var old_node: *Node = old_leaf_node.getNode();
        std.debug.assert(new_node.getColour() == .red);

        while (old_node.getParent()) |parent| {
            if (parent.getColour() != .red) break;
            if (parent.getParent() == null) break;

            const grandparent = parent.getParent() orelse unreachable;
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
                old_node = grandparent.getNode();
                new_node = &new_branch.branch;
            }
        }

        while (old_node.getParent()) |old_parent| {
            // std.debug.assert(!parent_node.isLeaf());
            // const old_parent = BranchNokVjkde.fromNode(parent_node);
            const new_parent: *Node = if (old_parent.left == old_node)
                old_parent.replaceLeft(self.allocator, new_node.getNode())
            else if (old_parent.right == old_node)
                old_parent.replaceRight(self.allocator, new_node.getNode())
            else
                unreachable;

            new_node.setParent(&new_parent.branch);
            new_node = &new_parent.branch;
            old_node = old_parent.getNode();
        }

        new_node.setColour(.black);
        return new_node.getNode();
    }

    pub fn format(self: Self, comptime _: []const u8, _: std.fmt.FormatOptions, writer: anytype) std.os.WriteError!void {
        return switch (self.root.*) {
            .leaf => |leaf| writer.print("rope.Rope{{ root = {} }}", .{leaf}),
            .branch => |branch| writer.print("rope.Rope{{ root = {} }}", .{branch}),
        };
    }
};

const Cursor = struct {
    const Self = @This();

    rope: Rope,
    curr_node: *Node,
    curr_node_offset: usize,

    fn init(rope: Rope) Self {
        return Self{
            .rope = rope,
            .curr_node = rope.root,
            .curr_node_offset = 0,
        };
    }

    fn next(self: *Self, maxlen: u32) []u8 {
        _ = maxlen;
        _ = self;
    }

    fn advanceBytes(self: *Self, bytes: u32) !void {
        _ = bytes;
        switch (self.curr_node.node_type) {
            .leaf => {},
            .branch => {},
        }
    }
};

const Buffer = struct {
    const Self = @This();
    const RopeList = std.ArrayList(Rope);

    allocator: Allocator,
    versions: RopeList,

    fn initEmpty(allocator: Allocator) !Self {
        return Self{
            .allocator = allocator,
            .versions = RopeList.init(allocator),
        };
    }
};

test "Cursor basic test" {
    std.debug.print("\n", .{});

    const allocator = std.testing.allocator;
    const rope0 = Rope.initEmpty(allocator);
    std.debug.print("rope0 = {}\n", .{rope0});

    const parts = [_][]const u8{
        "Lorem ", // 1
        "ipsum ", // 2
        "dolor ", // 3
        "sit ", // 4
        "amet, ", // 5
        "consectetur ", // 6
        "adipiscing ", // 7
        "elit, ", // 8
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

    var prev_rope = rope0;
    for (parts, 0..) |part, i| {
        const rope = try prev_rope.insertAt(.{ .byte_offset = prev_rope.len() }, part);
        prev_rope.deinit();
        std.debug.print("rope{} = {}\n", .{ i + 1, rope });
        try std.testing.expect(rope.isBalanced());
        prev_rope = rope;
    }
    prev_rope.deinit();
}

fn leafNodeAt(root: *Node, pos: Position, node_offset: *usize) !*LeafNode {
    return switch (pos) {
        .byte_offset => |byte_offset| leafNodeAtByteOffset(root, byte_offset, node_offset),
        .line_and_column => unreachable, //|p| leafNodeAtLineAndColumn(root, p.line, p.column),
    };
}

fn leafNodeAtByteOffset(root: *Node, byte_offset: usize, node_offset: *usize) !*LeafNode {
    var node: *Node = root;
    var offset = byte_offset;
    while (true) {
        if (offset > node.len()) return Error.EOS;
        switch (node.*) {
            .leaf => |*leaf| {
                node_offset.* = offset;
                return leaf;
            },
            .branch => |branch| {
                if (branch.left) |left| {
                    const left_len = left.len();
                    if (left_len > offset) {
                        node = left;
                        continue;
                    } else if (branch.right) |right| {
                        node = right;
                        offset -= left.len();
                        continue;
                    }
                }
            },
        }
        unreachable;
    }
    unreachable;
}
