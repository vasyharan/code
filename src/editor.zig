const std = @import("std");

const ScreenSize = struct { rows: u16, cols: u16 };
const Position = struct { x: u16 = 0, y: u16 = 0 };

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

const Row = struct {
    src: []u8,
};
const Buffer = std.ArrayList(Row);

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
};

pub const Editor = struct {
    const Self = @This();

    allocator: std.mem.Allocator,

    c: [3]u8 = [_]u8{0} ** 3,
    stdin: std.fs.File = undefined,
    // stdout: std.fs.File = undefined,
    stdout: std.io.BufferedWriter(4096, std.fs.File.Writer) = undefined,

    cursor: Position = Position{},
    offset: Position = Position{},
    size: ScreenSize = undefined,

    buffer: Buffer,

    pub fn init(allocator: std.mem.Allocator) !Self {
        return Self{
            .allocator = allocator,
            .stdin = std.io.getStdIn(),
            .stdout = std.io.bufferedWriter(std.io.getStdOut().writer()),
            // .stdout = std.io.getStdOut(),
            .size = try Self.getWindowSize(),
            .buffer = Buffer.init(allocator),
        };
    }

    pub fn deinit(self: *Self) void {
        // self.disableRawMode() catch unreachable;
        self.clearScreen() catch unreachable;

        for (self.buffer.items) |row| {
            self.allocator.free(row.src);
        }
        self.buffer.deinit();
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
            var row = Row{
                .src = try self.allocator.dupe(u8, line),
            };
            try self.buffer.append(row);
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
        for (0..(self.size.rows)) |numrow| {
            const bufrow = numrow + self.offset.y;
            var line: []const u8 = undefined;
            if (self.buffer.items.len > bufrow) {
                line = self.buffer.items[bufrow].src;
                const start = @min(line.len, self.offset.x);
                const end = @min(line.len, start + self.size.cols);
                line = line[start..end];
            } else {
                line = "~";
            }

            try self.draw(line);
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
                .cursor_down => if (self.cursor.y < self.buffer.items.len - 1) {
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
};
