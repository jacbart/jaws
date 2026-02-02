{
  description = "jaws flake";

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";
    rust-overlay.url = "github:oxalica/rust-overlay";
  };

  outputs =
    {
      self,
      nixpkgs,
      rust-overlay,
      ...
    }:
    let
      pname = "jaws";
      version = (builtins.fromTOML (builtins.readFile ./Cargo.toml)).package.version;
      projectRustVersion = "1.92.0";
      inherit (nixpkgs) lib;
      allSystems = [
        "x86_64-linux"
        "aarch64-linux"
        "x86_64-darwin"
        "aarch64-darwin"
      ];
      overlays = [ (import rust-overlay) ];
      forAllSystems =
        f:
        lib.genAttrs allSystems (
          system:
          f {
            pkgs = import nixpkgs { inherit system overlays; };
          }
        );
    in
    {
      packages = forAllSystems (
        { pkgs }:
        let
          rustVersion = pkgs.rust-bin.stable.${projectRustVersion}.default;
        in
        {
          default = pkgs.callPackage ./default.nix {
            inherit
              pname
              pkgs
              rustVersion
              self
              version
              ;
          };
        }
      );
      devShells = forAllSystems (
        { pkgs }:
        let
          rustVersion = pkgs.rust-bin.stable.${projectRustVersion}.default;
        in
        {
          default = pkgs.callPackage ./shell.nix {
            inherit
              pname
              pkgs
              rustVersion
              self
              version
              ;
          };
        }
      );

      homeManagerModules.default = import ./nix/hm-module.nix;
    };
}
