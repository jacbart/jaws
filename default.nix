{ pkgs ? import <nixpkgs> {}
, rustVersion
, self
, version
, pname
, ... }:
let
  inherit (pkgs) lib;
  # outputHashes = { "package-version" = "sha256-xxx"; };
  rustPlatform = pkgs.makeRustPlatform {
    cargo = rustVersion;
    rustc = rustVersion;
  };
in
rustPlatform.buildRustPackage {
  inherit pname version;
  src = lib.cleanSource self;
  cargoLock = {
    lockFile = ./Cargo.lock;
    # inherit outputHashes;
  };
  meta = with lib; {
    description = "flake for ${pname} version ${version}";
    homepage = "https://github.com/jacbart/";
    license = with licenses; [ mpl20 ];
    maintainers = with maintainers; [ jacbart ];
  };
}
