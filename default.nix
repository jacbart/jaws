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
    "ff-1.0.12" = "sha256-8ORRIrpuDPhfliR+8/vAFaKy++LaC1m+8pG0NONJ8GE=";
  };

  onepasswordSdk = pkgs.callPackage ./nix/onepassword-sdk.nix { };

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
  nativeBuildInputs = with pkgs; [
    pkg-config
    makeWrapper
  ];
  buildInputs = with pkgs; [
    openssl
    onepasswordSdk
  ];

  # Configure corteq-onepassword to use the local SDK
  ONEPASSWORD_SKIP_DOWNLOAD = "1";
  ONEPASSWORD_LIB_PATH = "${onepasswordSdk}/lib";

  postInstall = ''
    lib_name="libop_uniffi_core.so"
    if [ "${toString pkgs.stdenv.hostPlatform.isDarwin}" = "1" ]; then
      lib_name="libop_uniffi_core.dylib"
    fi

    wrapProgram $out/bin/jaws \
      --set ONEPASSWORD_LIB_PATH "${onepasswordSdk}/lib/$lib_name" \
      --prefix LD_LIBRARY_PATH : "${onepasswordSdk}/lib"
  '';

  meta = with lib; {
    description = "flake for ${pname} version ${version}";
    homepage = "https://github.com/jacbart/jaws";
    license = with licenses; [ mpl20 ];
    maintainers = with maintainers; [ jacbart ];
  };
}
