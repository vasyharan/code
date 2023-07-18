const std = @import("std");
const ts = @import("./treesitter.zig");

const ScreenSize = struct { rows: u16, cols: u16 };
const Position = struct { x: u32 = 0, y: u32 = 0 };

const KeyType = enum { normal, escape_seq };
const KeyEscapeSeq = enum {
    arrow_up,
    arrow_down,
    arrow_right,
    arrow_left,
};
const Key = union(KeyType) {
    normal: u8,
    escape_seq: KeyEscapeSeq,
};

const HighlightColour = [3]u8;
const HighlightStyle = struct {
    name: []const u8,
    fg: HighlightColour,
    bg: HighlightColour,
};
const Highlight = struct {
    start: Position,
    end: Position,
    style: HighlightStyle,
};
const Row = []u8;
const Buffer = struct {
    rows: std.ArrayList(Row),
    highlights: std.ArrayList(Highlight),
};

const EditorAction = enum {
    unknown,

    cursor_left,
    cursor_right,
    cursor_up,
    cursor_down,

    quit,
};

const EditorError = error{
    SetParam,
    GetWindowSize,
    TSParserError,
};

pub const Editor = struct {
    const Self = @This();

    allocator: std.mem.Allocator,

    c: [3]u8 = [_]u8{0} ** 3,
    stdin: std.fs.File = undefined,
    // stdout: std.fs.File = undefined,
    stdout: std.io.BufferedWriter(4096, std.fs.File.Writer) = undefined,
    log: std.fs.File = undefined,

    cursor: Position = Position{},
    offset: Position = Position{},
    size: ScreenSize = undefined,

    buffer: Buffer,

    ts_parser: ts.Parser = undefined,

    pub fn init(allocator: std.mem.Allocator) !Self {
        return Self{
            .allocator = allocator,
            .stdin = std.io.getStdIn(),
            .stdout = std.io.bufferedWriter(std.io.getStdOut().writer()),
            // .stdout = std.io.getStdOut(),
            .log = std.io.getStdErr(),
            .size = try Self.getWindowSize(),
            .buffer = Buffer{
                .rows = std.ArrayList(Row).init(allocator),
                .highlights = std.ArrayList(Highlight).init(allocator),
            },
            .ts_parser = try ts.Parser.init(allocator),
            .cursor = Position{ .y = 100 },
        };
    }

    pub fn deinit(self: *Self) void {
        self.clearScreen() catch unreachable;
        self.ts_parser.deinit();

        for (self.buffer.rows.items) |row| {
            self.allocator.free(row);
        }
        self.buffer.rows.deinit();
        for (self.buffer.highlights.items) |hl| {
            self.allocator.free(hl.style.name);
        }
        self.buffer.highlights.deinit();
    }

    pub fn enableRawMode() !std.os.termios {
        const os = std.os;
        const termios = try os.tcgetattr(os.STDIN_FILENO);
        var raw = termios;
        raw.iflag &= ~(os.linux.BRKINT | os.linux.ICRNL | os.linux.INPCK | os.linux.ISTRIP | os.linux.IXON);
        raw.oflag &= ~(os.linux.OPOST);
        raw.cflag |= (os.linux.CS8);
        raw.lflag &= ~(os.linux.ECHO | os.linux.ICANON | os.linux.IEXTEN | os.linux.ISIG);
        if (os.linux.tcsetattr(os.linux.STDIN_FILENO, .FLUSH, &raw) == -1) {
            return EditorError.SetParam;
        }
        return termios;
    }

    pub fn disableRawMode(termios: std.os.termios) !void {
        if (std.os.linux.tcsetattr(std.os.linux.STDIN_FILENO, .FLUSH, &termios) == -1) {
            return EditorError.SetParam;
        }
    }

    pub fn open(self: *Self, path: []const u8) !void {
        const file = try std.fs.cwd().createFile(path, .{ .read = true, .truncate = false });
        defer file.close();

        var contents = try file.reader().readAllAlloc(self.allocator, std.math.maxInt(u32));
        defer self.allocator.free(contents);

        var it = std.mem.splitSequence(u8, contents, "\n");
        while (it.next()) |line| {
            var dupe = try self.allocator.dupe(u8, line);
            try self.buffer.rows.append(dupe);
        }

        try self.ts_parser.setLanguage(ts.langZig());
        var ts_tree = try self.ts_parser.parseString(contents);
        defer ts_tree.deinit();

        const ts_query_source = @embedFile("tree-sitter/queries/zig/highlights.scm");
        var ts_query = try ts.Query.init(ts.langZig(), ts_query_source);
        defer ts_query.deinit();
        var ts_query_cursor = ts.QueryCursor.init();
        defer ts_query_cursor.deinit();

        ts_query_cursor.exec(ts_query, ts_tree.rootNode());
        while (ts_query_cursor.nextMatch()) |match| {
            for (0..match.captureCount()) |i| {
                if (match.capture(@intCast(i))) |capture| {
                    var tmp = capture.node();
                    var start = tmp.startPoint();
                    var end = tmp.endPoint();
                    const hl = Highlight{
                        .start = Position{ .y = start.row, .x = start.column },
                        .end = Position{ .y = end.row, .x = end.column },
                        .style = self.mapHighlight(capture.name(ts_query)),
                    };
                    //if (std.mem.eql(u8, "keyword", capture.name(ts_query))) {
                    // std.log.debug("capture={s}, name={s}, fg={any}, start={}/{}, end={}/{}", .{
                    //     capture.name(ts_query),
                    //     hl.style.name,
                    //     hl.style.fg,
                    //     hl.start.y,
                    //     hl.start.x,
                    //     hl.end.y,
                    //     hl.end.x,
                    // });
                    try self.buffer.highlights.append(hl);
                    //}
                }
            }
        }
    }

    fn readKey(self: *Self) !Key {
        _ = try self.stdin.read(self.c[0..1]);
        // _ = try std.os.read(std.os.linux.STDIN_FILENO, self.c[0..1]);

        return switch (self.c[0]) {
            '\x1b' => {
                // TODO: handle other escape sequence keys, pageup/down, home, end, delete
                _ = try self.stdin.read(self.c[1..2]);
                // _ = try std.os.read(std.os.linux.STDIN_FILENO, self.c[1..2]);
                return switch (self.c[1]) {
                    '[' => {
                        _ = try self.stdin.read(self.c[2..3]);
                        // _ = try std.os.read(std.os.linux.STDIN_FILENO, self.c[2..3]);
                        return switch (self.c[2]) {
                            'A' => Key{ .escape_seq = KeyEscapeSeq.arrow_up },
                            'B' => Key{ .escape_seq = KeyEscapeSeq.arrow_down },
                            'C' => Key{ .escape_seq = KeyEscapeSeq.arrow_right },
                            'D' => Key{ .escape_seq = KeyEscapeSeq.arrow_left },
                            else => Key{ .normal = self.c[0] },
                        };
                    },
                    else => Key{ .normal = self.c[0] },
                };
            },
            else => Key{ .normal = self.c[0] },
        };
    }

    fn mapKey(self: *Self, k: Key) EditorAction {
        _ = self;
        return switch (k) {
            KeyType.normal => |c| {
                return switch (c) {
                    Self.cntrlkey('q') => EditorAction.quit,
                    'k' => EditorAction.cursor_up,
                    'j' => EditorAction.cursor_down,
                    'l' => EditorAction.cursor_right,
                    'h' => EditorAction.cursor_left,
                    else => EditorAction.unknown,
                };
            },
            KeyType.escape_seq => |s| {
                return switch (s) {
                    .arrow_up => EditorAction.cursor_up,
                    .arrow_down => EditorAction.cursor_down,
                    .arrow_right => EditorAction.cursor_right,
                    .arrow_left => EditorAction.cursor_left,
                };
            },
        };
    }

    fn clearScreen(self: *Self) !void {
        try self.draw("\x1b[2J");
        try self.draw("\x1b[H");
        try self.stdout.flush();
    }

    fn refreshScreen(self: *Self) !void {
        try self.draw("\x1b[?25l");
        try self.draw("\x1b[H");

        if (self.cursor.y < self.offset.y) {
            self.offset.y = self.cursor.y;
        } else if (self.cursor.y >= self.offset.y + self.size.rows) {
            self.offset.y = self.cursor.y - self.size.rows + 1;
        }
        if (self.cursor.x < self.offset.x) {
            self.offset.x = self.cursor.x;
        } else if (self.cursor.x >= self.offset.x + self.size.cols) {
            self.offset.x = self.cursor.x - self.size.cols + 1;
        }

        try self.drawRows();

        try self.draw("\x1b[0K");
        try self.draw("\x1b[7m");
        var buf: [32]u8 = [_]u8{0} ** 32;
        var fmtted = try std.fmt.bufPrint(&buf, "{d}:{d} -- {d}:{d}", .{
            (self.cursor.x) + 1,
            (self.cursor.y) + 1,
            self.offset.x,
            self.offset.y,
        });
        try self.draw(fmtted);
        try self.draw("\x1b[0m");

        // draw cursor
        fmtted = try std.fmt.bufPrint(&buf, "\x1b[{d};{d}H", .{
            (self.cursor.y - self.offset.y) + 1,
            (self.cursor.x - self.offset.x) + 1,
        });

        try self.draw(fmtted);
        try self.draw("\x1b[?25h");
        try self.stdout.flush();
    }

    fn drawRows(self: *Self) !void {
        const fg_reset = HighlightColour{ 253, 244, 193 };
        var fg = fg_reset;
        var hl_index: u32 = 0;
        while (hl_index < self.buffer.highlights.items.len and self.buffer.highlights.items[hl_index].start.y < self.offset.y) {
            const hl = self.buffer.highlights.items[hl_index];
            fg = hl.style.fg;
            hl_index += 1;
        }
        for (0..(self.size.rows)) |numrow| {
            const bufrow = numrow + self.offset.y;
            if (self.buffer.rows.items.len > bufrow) {
                var line = self.buffer.rows.items[bufrow];
                const start = @min(line.len, self.offset.x);
                const end = @min(line.len, start + self.size.cols);

                var curr_x = start;
                var maybe_hl: ?Highlight = if (hl_index < self.buffer.highlights.items.len) self.buffer.highlights.items[hl_index] else null;
                while (maybe_hl) |hl| {
                    if (hl.start.y > bufrow) {
                        break;
                    }
                    if (curr_x >= end) {
                        break;
                    }
                    if (hl.start.x > curr_x) {
                        const next_x = hl.start.x;
                        try self.draw(line[curr_x..next_x]);
                        curr_x = next_x;
                    }
                    if (hl.start.x <= curr_x) {
                        // start highlight
                        // try self.draw("\x1b[31m");
                        fg = hl.style.fg;
                        var buf: [32]u8 = [_]u8{0} ** 32;
                        var fmtted = try std.fmt.bufPrint(&buf, "\x1b[38;2;{d};{d};{d}m", .{ fg[0], fg[1], fg[2] });
                        try self.draw(fmtted);
                    }
                    var next_x = end;
                    hl_index += 1;
                    maybe_hl = if (hl_index < self.buffer.highlights.items.len) self.buffer.highlights.items[hl_index] else null;
                    if (hl.end.y == bufrow) {
                        if (maybe_hl) |next_hl| {
                            if (next_hl.start.y == bufrow) {
                                next_x = @min(end, next_hl.start.x);
                            } else {
                                next_x = @min(end, hl.end.x);
                            }
                        } else {
                            next_x = @min(end, hl.end.x);
                        }
                    } else {
                        next_x = end;
                    }
                    // if (bufrow == 126) {
                    //     std.log.debug("{s}, {} {} {} {} {}/{} {}/{}", .{
                    //         hl.style.name,
                    //         start,
                    //         end,
                    //         curr_x,
                    //         next_x,
                    //         hl.start.y,
                    //         hl.start.x,
                    //         hl.end.y,
                    //         hl.end.x,
                    //     });
                    // }
                    try self.draw(line[curr_x..next_x]);

                    // end highlight
                    try self.draw("\x1b[39m");
                    fg = fg_reset;
                    curr_x = next_x;
                }
                try self.draw(line[curr_x..end]);
            } else {
                try self.draw("~");
            }

            try self.draw("\x1b[K");
            try self.draw("\r\n");
        }
    }

    fn draw(self: *Self, bytes: []const u8) !void {
        // _ = try std.os.write(std.os.linux.STDOUT_FILENO, bytes);
        _ = try self.stdout.write(bytes);
    }

    pub fn run(self: *Self) !void {
        while (true) {
            try self.refreshScreen();
            var key = try self.readKey();
            var action = self.mapKey(key);
            switch (action) {
                .quit => break,
                .cursor_up => if (self.cursor.y > 0) {
                    self.cursor.y -= 1;
                },
                .cursor_down => if (self.buffer.rows.items.len > 0 and self.cursor.y < self.buffer.rows.items.len - 1) {
                    self.cursor.y += 1;
                },
                .cursor_right => self.cursor.x += 1,
                .cursor_left => if (self.cursor.x > 0) {
                    self.cursor.x -= 1;
                },
                else => continue,
            }
        }
    }

    fn getWindowSize() !ScreenSize {
        var wsz: std.os.linux.winsize = undefined;
        const fd = @as(usize, @bitCast(@as(isize, std.os.linux.STDOUT_FILENO)));
        if (std.os.linux.syscall3(.ioctl, fd, std.os.linux.T.IOCGWINSZ, @intFromPtr(&wsz)) == -1 or wsz.ws_col == 0) {
            // _ = try os.write(os.linux.STDOUT_FILENO, "\x1b[999C\x1b[999B");
            // return ScreenSize{ .rows = 0, .cols = 0 };
            return EditorError.GetWindowSize;
        } else {
            // self.screenrows = wsz.ws_row;
            // self.screencols = wsz.ws_col;
            return ScreenSize{ .rows = wsz.ws_row - 1, .cols = wsz.ws_col };
        }
    }

    fn iscntrl(c: u8) bool {
        return c < 0x20 or c == 0x7f;
    }

    fn cntrlkey(c: u8) u8 {
        return c & 0x1f;
    }

    fn mapHighlight(self: Self, capture: []const u8) HighlightStyle {
        const red = HighlightColour{ 251, 73, 52 };
        const green = HighlightColour{ 184, 187, 38 };
        const yellow = HighlightColour{ 250, 189, 47 };
        const blue = HighlightColour{ 131, 165, 152 };
        const purple = HighlightColour{ 211, 134, 155 };
        const aqua = HighlightColour{ 142, 192, 124 };
        const orange = HighlightColour{ 254, 128, 25 };

        if (std.mem.eql(u8, "keyword", capture))
            return HighlightStyle{
                .name = self.allocator.dupe(u8, capture) catch unreachable,
                .fg = red,
                .bg = HighlightColour{ 40, 40, 40 },
            }
        else if (std.mem.eql(u8, "variable", capture))
            return HighlightStyle{
                .name = self.allocator.dupe(u8, capture) catch unreachable,
                .fg = green,
                .bg = HighlightColour{ 40, 40, 40 },
            }
        else if (std.mem.eql(u8, "type", capture))
            return HighlightStyle{
                .name = self.allocator.dupe(u8, capture) catch unreachable,
                .fg = yellow,
                .bg = HighlightColour{ 40, 40, 40 },
            }
        else if (std.mem.eql(u8, "function", capture))
            return HighlightStyle{
                .name = self.allocator.dupe(u8, capture) catch unreachable,
                .fg = blue,
                .bg = HighlightColour{ 40, 40, 40 },
            }
        else if (std.mem.eql(u8, "constant", capture))
            return HighlightStyle{
                .name = self.allocator.dupe(u8, capture) catch unreachable,
                .fg = purple,
                .bg = HighlightColour{ 40, 40, 40 },
            }
        else if (std.mem.eql(u8, "field", capture))
            return HighlightStyle{
                .name = self.allocator.dupe(u8, capture) catch unreachable,
                .fg = aqua,
                .bg = HighlightColour{ 40, 40, 40 },
            }
        else if (std.mem.eql(u8, "punctuation", capture))
            return HighlightStyle{
                .name = self.allocator.dupe(u8, capture) catch unreachable,
                .fg = orange,
                .bg = HighlightColour{ 40, 40, 40 },
            }
        else if (std.mem.eql(u8, "string", capture))
            return HighlightStyle{
                .name = self.allocator.dupe(u8, capture) catch unreachable,
                .fg = yellow,
                .bg = HighlightColour{ 40, 40, 40 },
            }
        else if (std.mem.eql(u8, "string.escape", capture))
            return HighlightStyle{
                .name = self.allocator.dupe(u8, capture) catch unreachable,
                .fg = red,
                .bg = HighlightColour{ 40, 40, 40 },
            }
        else
            return HighlightStyle{
                .name = self.allocator.dupe(u8, capture) catch unreachable,
                .fg = HighlightColour{ 253, 244, 193 },
                .bg = HighlightColour{ 40, 40, 40 },
            };
    }

    fn log(self: *Self, bytes: []const u8) !void {
        _ = try self.log.write(bytes);
    }
};
