const std = @import("std");
const Allocator = std.mem.Allocator;

const Error = error{EOS};

const Colour = enum(u1) { red, black };
const NodeType = enum(u8) {
    branch,
    leaf,
};

const Node = struct {
    const Self = @This();

    parent_and_colour: usize, // parent | colour
    node_type: NodeType,
    ref_count: u16,
    len: usize,

    fn deref(self: *Self, allocator: Allocator) void {
        self.ref_count -= 1;
        if (self.ref_count == 0)
            switch (self.node_type) {
                NodeType.branch => BranchNode.fromNode(self).deinit(allocator),
                NodeType.leaf => LeafNode.fromNode(self).deinit(allocator),
            };
    }

    fn ref(self: *Self) void {
        self.ref_count += 1;
    }

    fn isRoot(self: *const Self) bool {
        return self.getParent() == null;
    }

    fn isLeaf(self: *const Self) bool {
        return self.node_type == .leaf;
    }

    fn getParent(self: *const Self) ?*Self {
        const mask: usize = 1;
        comptime {
            std.debug.assert(@alignOf(*Self) >= 2);
        }
        const maybe_ptr = self.parent_and_colour & ~mask;
        return if (maybe_ptr == 0) null else @as(*Self, @ptrFromInt(maybe_ptr));
    }

    fn setParent(node: *Node, parent: ?*Node) void {
        node.parent_and_colour = @intFromPtr(parent) | (node.parent_and_colour & 1);
    }

    fn getColor(self: *const Self) Colour {
        const colour_int = @as(u1, @intCast(self.parent_and_colour & 1));
        return @as(Colour, @enumFromInt(colour_int));
    }

    fn setColor(self: *Self, colour: Colour) void {
        const mask: usize = 1;
        self.parent_and_colour = (self.parent_and_colour & ~mask) | @intFromEnum(colour);
    }
};

const BranchNode = struct {
    const Self = @This();

    node: Node,
    left: ?*Node,
    right: ?*Node,

    fn init(allocator: Allocator, left: ?*Node, right: ?*Node) *Self {
        const self = allocator.create(Self) catch @panic("oom");
        var len: usize = 0;
        if (left) |n| {
            n.ref();
            // std.debug.assert(n.getParent() == null);
            n.setParent(&self.node);
            len += n.len;
        }
        if (right) |n| {
            n.ref();
            // std.debug.assert(n.getParent() == null);
            n.setParent(&self.node);
            len += n.len;
        }
        self.left = left;
        self.right = right;
        self.node = Node{
            .node_type = .branch,
            .parent_and_colour = @intFromEnum(Colour.red),
            .len = len,
            .ref_count = 0,
        };
        return self;
    }

    fn deinit(self: *const Self, allocator: Allocator) void {
        if (self.left) |left| left.deref(allocator);
        if (self.right) |right| right.deref(allocator);
        allocator.destroy(self);
    }

    fn replaceLeft(self: *const Self, allocator: Allocator, left: ?*Node) *BranchNode {
        const replaced = allocator.create(Self) catch @panic("oom");
        replaced.left = left;
        replaced.right = self.right;
        var len: usize = 0;
        if (left) |n| {
            n.ref();
            len = n.len;
        }
        if (self.right) |n| {
            n.ref();
            len += n.len;
        }
        replaced.node = Node{
            .node_type = .branch,
            .parent_and_colour = @intFromEnum(self.node.getColor()),
            .len = len,
            .ref_count = 0,
        };
        return replaced;
    }

    fn replaceRight(self: *const Self, allocator: Allocator, right: ?*Node) *BranchNode {
        const replaced = allocator.create(Self) catch @panic("oom");
        replaced.left = self.left;
        replaced.right = right;
        var len: usize = 0;
        if (right) |n| {
            len = n.len;
            n.ref();
        }
        if (self.left) |n| {
            n.ref();
            len += n.len;
        }
        replaced.node = Node{
            .node_type = .branch,
            .parent_and_colour = @intFromEnum(self.node.getColor()),
            .len = len,
            .ref_count = 0,
        };
        return replaced;
    }

    pub fn format(self: Self, comptime _: []const u8, _: std.fmt.FormatOptions, writer: anytype) std.os.WriteError!void {
        try writer.print(
            "rope.BranchNode{{ .colour = {}, .ref_count = {d}, .len = {d}",
            .{ self.node.getColor(), self.node.ref_count, self.node.len },
        );
        if (self.left) |left| {
            if (left.isLeaf()) {
                const leaf = LeafNode.fromNode(left);
                try writer.print(", .left = {}", .{leaf});
            } else {
                const branch = BranchNode.fromNode(left);
                try writer.print(", .left = {}", .{branch});
            }
        }
        if (self.right) |right| {
            if (right.isLeaf()) {
                const leaf = LeafNode.fromNode(right);
                try writer.print(", .right = {}", .{leaf});
            } else {
                const branch = BranchNode.fromNode(right);
                try writer.print(", .right = {}", .{branch});
            }
        }
        try writer.print(" }}", .{});
    }

    fn fromNode(node: *Node) *Self {
        return @fieldParentPtr(Self, "node", node);
    }
};

const LeafNode = struct {
    const Self = @This();

    node: Node,
    val: []const u8,

    fn initEmpty(allocator: Allocator) *Self {
        return LeafNode.init(allocator, "");
    }

    fn init(allocator: Allocator, val: []const u8) *Self {
        const self = allocator.create(Self) catch @panic("oom");
        self.node = Node{
            .node_type = .leaf,
            .parent_and_colour = @intFromEnum(Colour.black),
            .len = val.len,
            .ref_count = 0,
        };
        self.val = val;
        return self;
    }

    fn deinit(self: *const Self, allocator: Allocator) void {
        allocator.destroy(self);
    }

    fn slice(self: *const Self, allocator: Allocator, start: usize, end: usize) *LeafNode {
        return LeafNode.init(allocator, self.val[start..end]);
    }

    pub fn format(self: Self, comptime _: []const u8, _: std.fmt.FormatOptions, writer: anytype) std.os.WriteError!void {
        return writer.print(
            "rope.LeafNode{{ .ref_count = {d}, .len = {d}, .val = {s} }}",
            .{ self.node.ref_count, self.node.len, self.val },
        );
    }

    fn fromNode(node: *Node) *Self {
        return @fieldParentPtr(Self, "node", node);
    }
};

const Rope = struct {
    const Self = @This();

    allocator: Allocator,
    root: *Node,

    fn initEmpty(allocator: Allocator) Self {
        const root = LeafNode.initEmpty(allocator);
        return Rope.initNode(allocator, &root.node);
    }

    fn initNode(allocator: Allocator, node: *Node) Self {
        node.ref();
        return Self{ .allocator = allocator, .root = node };
    }

    fn deinit(self: *const Self) void {
        self.root.deref(self.allocator);
    }

    fn cursor(self: *const Self) Cursor {
        return Cursor.init(self.*);
    }

    fn isBalanced(self: *const Self) bool {
        if (self.root.isLeaf()) {
            return true;
        } else {
            return isNodeBalanced(self.root, 0).balanced;
        }
    }

    fn isNodeBalanced(maybe_node: ?*Node, black_depth: usize) struct { balanced: bool, black_height: usize = 0 } {
        const unbalanced = .{ .balanced = false };
        if (maybe_node == null or (maybe_node orelse unreachable).isLeaf()) {
            return .{ .balanced = true, .black_height = black_depth };
        }

        const node: *Node = maybe_node orelse unreachable;
        const branch = BranchNode.fromNode(node);
        if (branch.node.getColor() == .red) {
            if (branch.left) |left| if (left.getColor() == .red) return unbalanced;
            if (branch.right) |right| if (right.getColor() == .red) return unbalanced;
        }

        const next_black_depth = black_depth + @as(usize, if (node.getColor() == .black) 1 else 0);
        const left_result = isNodeBalanced(branch.left, next_black_depth);
        const right_result = isNodeBalanced(branch.right, next_black_depth);

        if (std.meta.eql(left_result, right_result))
            return left_result
        else
            return unbalanced;
    }

    pub fn format(self: Self, comptime _: []const u8, _: std.fmt.FormatOptions, writer: anytype) std.os.WriteError!void {
        if (self.root.isLeaf()) {
            const leaf = LeafNode.fromNode(self.root);
            return writer.print("rope.Rope{{ root = {} }}", .{leaf});
        } else {
            const branch = BranchNode.fromNode(self.root);
            return writer.print("rope.Rope{{ root = {} }}", .{branch});
        }
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

    fn insert(self: *Self, text: []const u8) Rope {
        if (text.len == 0) {
            self.rope = Rope.initNode(self.rope.allocator, self.rope.root);
            return self.rope;
        }

        if (!self.curr_node.isLeaf()) unreachable;
        const leaf = LeafNode.fromNode(self.curr_node);

        if (leaf.val.len == 0) {
            const new_leaf_node = LeafNode.init(self.rope.allocator, text);
            self.rope = Rope.initNode(self.rope.allocator, &new_leaf_node.node);
            self.curr_node = &new_leaf_node.node;
            self.curr_node_offset = text.len;
            return self.rope;
        }

        // create a new branch node, to insert the new text into.
        const new_branch_left = leaf.slice(self.rope.allocator, 0, self.curr_node_offset);
        const new_branch_right = LeafNode.init(self.rope.allocator, text);
        const new_branch = BranchNode.init(
            self.rope.allocator,
            &new_branch_left.node,
            &new_branch_right.node,
        );

        // balance the newly inserted node and update
        // the new node's path to the root (ancestors)
        const new_root = self.balance(&new_branch.node, &leaf.node);
        self.rope = Rope.initNode(self.rope.allocator, new_root);
        self.curr_node = &new_branch_right.node;
        self.curr_node_offset = text.len;
        return self.rope;
    }

    fn balance(self: Self, nnode: *Node, onode: *Node) *Node {
        var new_node: *Node = nnode;
        var old_node: *Node = onode;
        std.debug.assert(!new_node.isLeaf());
        std.debug.assert(new_node.getColor() == .red);

        while (old_node.getParent()) |parent_node| {
            std.debug.assert(!parent_node.isLeaf());
            if (parent_node.getColor() != .red) break;
            if (parent_node.getParent() == null) break;

            const grandparent_node = parent_node.getParent() orelse unreachable;
            std.debug.assert(!grandparent_node.isLeaf());
            if (grandparent_node.getColor() != .black) break;

            const parent = BranchNode.fromNode(parent_node);
            const grandparent = BranchNode.fromNode(grandparent_node);

            if (grandparent.left == parent_node and parent.left == old_node) {
                // case 1
                unreachable;
            } else if (grandparent.left == parent_node and parent.right == old_node) {
                // case2
                unreachable;
            } else if (grandparent.right == parent_node and parent.left == old_node) {
                // case3
                unreachable;
            } else if (grandparent.right == parent_node and parent.right == old_node) {
                // case 4
                const new_left_branch = BranchNode.init(
                    self.rope.allocator,
                    grandparent.left,
                    parent.left,
                );
                new_left_branch.node.setColor(.black);
                const new_branch = BranchNode.init(
                    self.rope.allocator,
                    &new_left_branch.node,
                    new_node,
                );
                new_node.setColor(.black);
                old_node = grandparent_node;
                new_node = &new_branch.node;
            }
        }

        while (old_node.getParent()) |parent_node| {
            std.debug.assert(!parent_node.isLeaf());
            const old_parent = BranchNode.fromNode(parent_node);
            const new_parent: *BranchNode = if (old_parent.left == old_node)
                old_parent.replaceLeft(self.rope.allocator, new_node)
            else if (old_parent.right == old_node)
                old_parent.replaceRight(self.rope.allocator, new_node)
            else
                unreachable;

            new_node.setParent(&new_parent.node);
            new_node = &new_parent.node;
            old_node = &old_parent.node;
        }

        new_node.setColor(.black);
        return new_node;
    }

    fn delete(self: *Self, len: u32) !Self {
        _ = len;
        _ = self;
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

    var cursor = rope0.cursor();
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
    // var ropes = [_]?Rope{null} ** parts.len;
    // defer {
    //     for (ropes) |maybe_rope| {
    //         if (maybe_rope) |rope| rope.deinit();
    //     }
    // }
    var prev_rope = rope0;
    for (parts, 0..) |part, i| {
        const rope = cursor.insert(part);
        // ropes[i] = rope;
        std.debug.print("rope{} = {}\n", .{ i + 1, rope });
        try std.testing.expect(rope.isBalanced());
        prev_rope.deinit();
        prev_rope = rope;
    }
    prev_rope.deinit();
}
