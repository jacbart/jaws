{
  pkgs ? import <nixpkgs> { },
  rustVersion,
  self,
  version,
  pname,
  ...
}:
let
  inherit (pkgs) lib;
  outputHashes = {
    "ff-1.0.5" = "sha256-eaLHCLKVQ+A54c8PXv5DuVeCKUsiGZTsDh79df5Ja/g=";
  };
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
    inherit outputHashes;
  };
  meta = with lib; {
    description = "flake for ${pname} version ${version}";
    homepage = "https://github.com/jacbart/jaws";
    license = with licenses; [ mpl20 ];
    maintainers = with maintainers; [ jacbart ];
  };
}
