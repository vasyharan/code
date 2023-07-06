{ pkgs, ... }:

{
  devcontainer.enable = true;

  languages.nix.enable = true;
  languages.zig.enable = true;
}