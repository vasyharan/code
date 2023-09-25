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
    channel = "stable";
    components = [ 
      "rustc" 
      "cargo" 
      "cargo-watch" 
      "clippy" 
      "rustfmt" 
      "rust-analyzer" 
    ];
    toolchain.cargo-watch = unstable.cargo-watch;
  };

  packages = [
    pkgs.gdb
    # unstable.mdbook
    # unstable.graphviz
    # unstable.mdbook-graphviz
  ];
}
