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

    allocator: Allocator,
    parent_and_colour: usize, // parent | colour
    node_type: NodeType,
    ref_count: u16,
    len: usize,

    fn deref(self: *Self) void {
        self.ref_count -= 1;
        if (self.ref_count == 0)
            switch (self.node_type) {
                NodeType.branch => BranchNode.fromNode(self).deinit(),
                NodeType.leaf => LeafNode.fromNode(self).deinit(),
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
        self.parent_and_color = (self.parent_and_color & ~mask) | @intFromEnum(colour);
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
            std.debug.assert(n.getParent() == null);
            n.setParent(&self.node);
            len += n.len;
        }
        if (right) |n| {
            n.ref();
            std.debug.assert(n.getParent() == null);
            n.setParent(&self.node);
            len += n.len;
        }
        self.left = left;
        self.right = right;
        self.node = Node{
            .allocator = allocator,
            .node_type = NodeType.branch,
            .parent_and_colour = @intFromEnum(Colour.red),
            .len = len,
            .ref_count = 0,
        };
        return self;
    }

    fn deinit(self: *const Self) void {
        if (self.left) |left| left.deref();
        if (self.right) |right| right.deref();
        self.node.allocator.destroy(self);
    }

    fn replaceLeft(self: *const Self, left: ?*Node) *BranchNode {
        const replaced = self.node.allocator.create(Self) catch @panic("oom");
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
            .allocator = self.node.allocator,
            .node_type = .branch,
            .parent_and_colour = @intFromEnum(self.node.getColor()),
            .len = len,
            .ref_count = 0,
        };
        return replaced;
    }

    fn replaceRight(self: *const Self, right: ?*Node) *BranchNode {
        const replaced = self.node.allocator.create(Self) catch @panic("oom");
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
            .allocator = self.node.allocator,
            .node_type = .branch,
            .parent_and_colour = @intFromEnum(self.node.getColor()),
            .len = len,
            .ref_count = 0,
        };
        return replaced;
    }

    pub fn format(self: Self, comptime _: []const u8, _: std.fmt.FormatOptions, writer: anytype) std.os.WriteError!void {
        try writer.print(
            "rope.BranchNode{{ .node = {*}, .ref_count = {d}, .len = {d}",
            .{ &self.node, self.node.ref_count, self.node.len },
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
            .allocator = allocator,
            .node_type = .leaf,
            .parent_and_colour = @intFromEnum(Colour.black),
            .len = val.len,
            .ref_count = 0,
        };
        self.val = val;
        return self;
    }

    fn deinit(self: *Self) void {
        self.node.allocator.destroy(self);
    }

    fn slice(self: *const Self, start: usize, end: usize) *LeafNode {
        return LeafNode.init(
            self.node.allocator,
            self.val[start..end],
        );
    }

    pub fn format(self: Self, comptime _: []const u8, _: std.fmt.FormatOptions, writer: anytype) std.os.WriteError!void {
        return writer.print(
            "rope.LeafNode{{ .node = {*}, .ref_count = {d}, .len = {d}, .val = {s} }}",
            .{ &self.node, self.node.ref_count, self.node.len, self.val },
        );
    }

    fn fromNode(node: *Node) *Self {
        return @fieldParentPtr(Self, "node", node);
    }
};

const Rope = struct {
    const Self = @This();

    root: *Node,

    fn initEmpty(allocator: Allocator) Self {
        const root = LeafNode.initEmpty(allocator);
        return Rope.initNode(&root.node);
    }

    fn initNode(node: *Node) Self {
        node.ref();
        return Self{ .root = node };
    }

    fn deinit(self: *const Self) void {
        self.root.deref();
    }

    fn cursor(self: *const Self) Cursor {
        return Cursor.init(self.*);
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
        if (!self.curr_node.isLeaf()) unreachable;
        const leaf = LeafNode.fromNode(self.curr_node);

        // create a new branch node, to insert the new text into.
        const new_branch_left = leaf.slice(0, self.curr_node_offset);
        const new_branch_right = LeafNode.init(self.rope.root.allocator, text);
        const new_branch = BranchNode.init(
            self.rope.root.allocator,
            &new_branch_left.node,
            &new_branch_right.node,
        );

        // update the new node's path to the root (ancentors)
        var old_node: *Node = &leaf.node;
        var new_node: *Node = &new_branch.node;
        while (old_node.getParent()) |pnode| {
            std.debug.assert(!pnode.isLeaf());
            const old_parent = BranchNode.fromNode(pnode);
            const new_parent: *BranchNode = if (old_parent.left == old_node)
                old_parent.replaceLeft(new_node)
            else if (old_parent.right == old_node)
                old_parent.replaceRight(new_node)
            else
                unreachable;

            new_node.setParent(&new_parent.node);
            new_node = &new_parent.node;
            old_node = &old_parent.node;
        }

        self.rope = Rope.initNode(new_node);
        self.curr_node = &new_branch_right.node;
        self.curr_node_offset = text.len;
        return self.rope;
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
    const allocator = std.testing.allocator;
    const rope0 = Rope.initEmpty(allocator);
    std.debug.print("\n", .{});
    defer rope0.deinit();

    var cursor = rope0.cursor();
    const rope1 = cursor.insert("Hello");
    defer rope1.deinit();

    const rope2 = cursor.insert(" World!");
    defer rope2.deinit();

    std.debug.print("rope0 = {}\n", .{rope0});
    std.debug.print("rope1 = {}\n", .{rope1});
    std.debug.print("rope2 = {}\n", .{rope2});

    const rope3 = cursor.insert(" I");
    defer rope3.deinit();
    std.debug.print("rope3 = {}\n", .{rope3});
}
