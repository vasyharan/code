{ inputs, lib, config, ... } @ args:
let
  pkgs = import inputs.nixpkgs { 
    system = args.pkgs.stdenv.system; 
    overlays = [ 
      (final: prev: rec {
        zigpkgs = inputs.zig.packages.${prev.system};
        zig = zigpkgs.master-2023-09-18;
      })
     ];
  };
  unstable = import inputs.unstable {
    system = args.pkgs.stdenv.system; 
  };
in {
  devcontainer.enable = true;

  languages.nix.enable = true;
  languages.rust = {
    enable = true;
    channel = "nightly";
    components = [ 
      "rustc" 
      "cargo" 
      "cargo-watch" 
      "cargo-fuzz" 
      "clippy" 
      "rustfmt" 
      "rust-analyzer" 
    ];
    toolchain.cargo-watch = unstable.cargo-watch;
    toolchain.cargo-fuzz = unstable.cargo-fuzz;
  };

  packages = [
    # unstable.gdb
    unstable.lldb
    # unstable.mdbook
    # unstable.graphviz
    # unstable.mdbook-graphviz
  ];
}
