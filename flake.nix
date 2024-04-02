{
  inputs = {
    unstable.url= "github:NixOS/nixpkgs/nixpkgs-unstable";
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-22.11";
    flake-utils.url = "github:numtide/flake-utils";
  };

  nixConfig = {
    extra-trusted-public-keys = "devenv.cachix.org-1:w1cLUi8dv3hnoSPGAuibQv+f9TZLr6cv/Hm9XgU50cw=";
    extra-substituters = "https://devenv.cachix.org";
  };

  outputs = { self, nixpkgs, unstable, flake-utils, ... } @ inputs:
    flake-utils.lib.eachDefaultSystem (system: 
      let
        pkgs = import nixpkgs {
          inherit system;
        };
      in {
      devShells.default = pkgs.mkShell {
        buildInputs = [];
      };
    });
}