{
  description = "jaws flake";

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-25.11";
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

      # Cross-compilation target definitions
      # Maps a short name to { triple, system } for cargo-zigbuild
      crossTargets = {
        "x86_64-linux" = {
          triple = "x86_64-unknown-linux-gnu";
          system = "x86_64-linux";
        };
        "aarch64-linux" = {
          triple = "aarch64-unknown-linux-gnu";
          system = "aarch64-linux";
        };
        "x86_64-darwin" = {
          triple = "x86_64-apple-darwin";
          system = "x86_64-darwin";
        };
        "aarch64-darwin" = {
          triple = "aarch64-apple-darwin";
          system = "aarch64-darwin";
        };
      };
    in
    {
      packages = forAllSystems (
        { pkgs }:
        let
          rustVersion = pkgs.rust-bin.stable.${projectRustVersion}.default;

          # Helper to create a cross-compiled package
          mkCrossPackage =
            targetName: targetInfo:
            pkgs.callPackage ./nix/cross-build.nix {
              inherit
                pname
                pkgs
                self
                version
                ;
              rustVersion = pkgs.rust-bin.stable.${projectRustVersion}.default.override {
                targets = [ targetInfo.triple ];
              };
              targetTriple = targetInfo.triple;
              targetSystem = targetInfo.system;
            };

          # Generate cross-compiled packages for all targets
          crossPackages = lib.mapAttrs' (name: info: {
            name = "jaws-${name}";
            value = mkCrossPackage name info;
          }) crossTargets;
        in
        {
          # Native build (uses buildRustPackage, optimal for current platform)
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
        // crossPackages
      );
      devShells = forAllSystems (
        { pkgs }:
        let
          # Include cross-compilation targets for GoReleaser builds
          rustVersion = pkgs.rust-bin.stable.${projectRustVersion}.default.override {
            targets = [
              "x86_64-unknown-linux-gnu"
              "aarch64-unknown-linux-gnu"
              "x86_64-apple-darwin"
              "aarch64-apple-darwin"
            ];
          };
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
