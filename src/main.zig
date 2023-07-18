const std = @import("std");

const ts = @import("./treesitter.zig");
const Editor = @import("./editor.zig").Editor;

var termios: ?std.os.termios = null;

pub fn main() !void {
    var args = std.process.args();
    _ = args.next(); // ignore self

    var gpa = std.heap.GeneralPurposeAllocator(.{}){};
    var allocator = gpa.allocator();
    defer _ = gpa.deinit();

    ts.init();
    var editor = try Editor.init(allocator);
    defer editor.deinit();

    termios = try Editor.enableRawMode();
    defer Editor.disableRawMode(termios.?) catch unreachable;

    if (args.next()) |path| {
        try editor.open(path);
    }

    try editor.run();
}

pub fn panic(msg: []const u8, error_return_trace: ?*std.builtin.StackTrace, ret_addr: ?usize) noreturn {
    if (termios) |t| {
        try Editor.disableRawMode(t);
    }
    std.builtin.default_panic(msg, error_return_trace, ret_addr);
}
