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
  languages.zig = {
    enable = true;
    package = pkgs.zig;
  };

  packages = [
    unstable.mdbook
    unstable.graphviz
    unstable.mdbook-graphviz
  ] ++ lib.optionals (!config.container.isBuilding) [
    pkgs.gdb
    # unstable.zls
    (pkgs.stdenvNoCC.mkDerivation {
      name = "zls";
      src = pkgs.fetchFromGitHub {
        owner = "zigtools";
        repo = "zls";
        rev = "14f03d9c679b988d078d9b25e8fbf0596fc05bff";
        hash = "sha256-lWH0K8eVgoZfN9ZBDWUVF9XtZ6VJSPRGhW/NQy4XSok=";
        # fetchSubmodules = true;
      };
      nativeBuildInputs = [ pkgs.zig ];
      dontConfigure = true;
      # dontInstall = true;
      preBuild = ''
        mkdir -p $out
        mkdir -p .cache/{p,z,tmp}
      '';
      buildPhase = ''
        zig build --cache-dir $(pwd)/zig-cache --global-cache-dir $(pwd)/.cache -Dcpu=baseline -Doptimize=ReleaseSafe --prefix $out
      '';
      installPhase = ''
        zig build --cache-dir $(pwd)/zig-cache --global-cache-dir $(pwd)/.cache -Dcpu=baseline -Doptimize=ReleaseSafe --prefix $out install
      '';
    })
  ];
}
