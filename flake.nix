{
  inputs = {
    unstable.url= "github:NixOS/nixpkgs/nixpkgs-unstable";
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-22.11";
    flake-utils.url = "github:numtide/flake-utils";
    fenix = {
      url = "github:nix-community/fenix";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  nixConfig = {
    extra-trusted-public-keys = "devenv.cachix.org-1:w1cLUi8dv3hnoSPGAuibQv+f9TZLr6cv/Hm9XgU50cw=";
    extra-substituters = "https://devenv.cachix.org";
  };

  outputs = { self, nixpkgs, unstable, flake-utils, fenix, ... } @ inputs:
    flake-utils.lib.eachDefaultSystem (system:
      let
        overlays = [ fenix.overlays.default ];
        pkgs = import nixpkgs {
          inherit system overlays;
        };
        rustToolchain = pkgs.fenix.fromToolchainFile {
          file = ./rust-toolchain.toml;
          sha256 = "sha256-sH6tcgUqft0TlybCuF4LirxmePW09mZDgu/SFP1bxGE=";
        };
        rustPlatform = pkgs.makeRustPlatform {
          cargo = rustToolchain;
          rustc = rustToolchain;
        };
      in {
      devShells.default = pkgs.mkShell {
        packages = with pkgs; [
          rustToolchain
          cargo-watch
        ] ++ lib.optional pkgs.stdenv.isDarwin pkgs.libiconv;
        shellHook = ''
          export RUST_LOG="info"
          export RUST_BACKTRACE=1
        '';
      };
      packages.default = rustPlatform.buildRustPackage {
          pname = "toku";
          version = "0.1.0";
          src = ./.;
          cargoLock.lockFile = ./Cargo.lock;
        };
    });
}
