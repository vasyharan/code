const std = @import("std");
const GitRepoStep = @import("GitRepoStep.zig");

pub fn build(b: *std.build.Builder) void {
    const target = b.standardTargetOptions(.{});
    const optimize = b.standardOptimizeOption(.{});

    const treesitter_repo = GitRepoStep.create(b, .{
        .url = "https://github.com/tree-sitter/tree-sitter",
        .sha = "834ae233cbef757dbbed68eb149a7e3059cc1695",
        .sha_check = .err,
        .fetch_enabled = true,
    });
    const treesitter = b.addSharedLibrary(.{
        .name = "treesitter",
        .target = target,
        .optimize = optimize,
        .link_libc = true,
    });
    treesitter.step.dependOn(&treesitter_repo.step);
    treesitter.addCSourceFile(b.pathJoin(&[_][]const u8{ treesitter_repo.getPath(&treesitter.step), "lib/src/lib.c" }), &.{});
    treesitter.addIncludePath(b.pathJoin(&[_][]const u8{ treesitter_repo.getPath(&treesitter.step), "lib/include" }));
    treesitter.addIncludePath(b.pathJoin(&[_][]const u8{ treesitter_repo.getPath(&treesitter.step), "lib/src" }));

    const treesitter_zig_repo = GitRepoStep.create(b, .{
        .url = "https://github.com/maxxnino/tree-sitter-zig",
        .sha = "0d08703e4c3f426ec61695d7617415fff97029bd",
        .sha_check = .err,
        .fetch_enabled = true,
    });
    treesitter.step.dependOn(&treesitter_zig_repo.step);
    treesitter.addCSourceFile(b.pathJoin(&[_][]const u8{ treesitter_zig_repo.getPath(&treesitter.step), "src/parser.c" }), &.{});

    const treesitter_zig_queries = MyBuildStep.create(
        b,
        .{
            .source_path = std.build.FileSource.relative("dep/tree-sitter-zig/queries/highlights.scm"),
            .dest_path = std.build.FileSource.relative("src/tree-sitter/queries/zig/highlights.scm"),
        },
    );

    const exe = b.addExecutable(.{
        .name = "code",
        .root_source_file = std.build.FileSource.relative("src/main.zig"),
        .target = target,
        .optimize = optimize,
    });
    exe.step.dependOn(&treesitter_zig_queries.step);
    exe.addIncludePath(b.pathJoin(&[_][]const u8{ treesitter_repo.getPath(&treesitter.step), "lib/include" }));
    exe.linkLibrary(treesitter);
    b.installArtifact(exe);

    const run_cmd = b.addRunArtifact(exe);
    run_cmd.step.dependOn(b.getInstallStep());
    if (b.args) |args| {
        run_cmd.addArgs(args);
    }

    const run_step = b.step("run", "Run the app");
    run_step.dependOn(&run_cmd.step);

    const rope_tests = b.addTest(.{
        .name = "rope",
        .root_source_file = std.build.FileSource.relative("src/rope.zig"),
        .target = target,
        .optimize = optimize,
    });

    const test_step = b.step("test", "Run unit tests");
    test_step.dependOn(&rope_tests.step);
}

const MyBuildStep = struct {
    const Self = @This();
    const Options = struct {
        source_path: std.build.FileSource,
        dest_path: std.build.FileSource,
    };

    step: std.build.Step,
    source_path: []const u8,
    dest_path: []const u8,

    pub fn create(b: *std.build.Builder, options: Options) *Self {
        var self = b.allocator.create(Self) catch @panic("memory");
        self.* = MyBuildStep{
            .step = std.build.Step.init(.{
                .id = .custom,
                .name = b.fmt("copy {s} -> {s}", .{ options.source_path.path, options.dest_path.path }),
                .owner = b,
                .makeFn = Self.doStep,
            }),
            .source_path = options.source_path.getPath(b),
            .dest_path = options.dest_path.getPath(b),
        };
        return self;
    }

    pub fn doStep(step: *std.build.Step, prog_node: *std.Progress.Node) !void {
        _ = prog_node;
        const self = @fieldParentPtr(MyBuildStep, "step", step);
        // const dirname = std.fs.path.dirname(self.dest_path);
        // if (dirname) |dir| {
        //     std.fs.makeDirAbsolute(dir) catch @panic(self.step.owner.fmt("{s}", .{dir}));
        // }
        try std.fs.copyFileAbsolute(self.source_path, self.dest_path, .{});
    }
};
