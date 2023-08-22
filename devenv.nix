{ inputs, ... } @ args:
let
  pkgs = import inputs.nixpkgs { 
    system = args.pkgs.stdenv.system; 
    overlays = [ 
      (final: prev: rec {
        zigpkgs = inputs.zig.packages.${prev.system};
        zig = zigpkgs.master-2023-07-05;
      })
     ];
  };
  unstable = import inputs.unstable {
    system = args.pkgs.stdenv.system; 
  };
in {
  devcontainer.enable = true;

  languages.nix.enable = true;
  languages.zig = {
    enable = true;
    package = pkgs.zig;
  };

  packages = [
    pkgs.gdb
    unstable.zls
    unstable.mdbook
    unstable.graphviz
    unstable.mdbook-graphviz
  ];
}