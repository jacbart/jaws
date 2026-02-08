{
  pkgs ? import <nixpkgs> { },
  pname,
  rustVersion,
  version,
  ...
}:

let
  onepasswordSdk = pkgs.callPackage ./nix/onepassword-sdk.nix { };
in
pkgs.mkShell {
  name = "${pname}-${version}";
  buildInputs = with pkgs; [
    (rustVersion.override { extensions = [ "rust-src" ]; })
    bacon
    rust-analyzer
    lldb
    openssl
    pkg-config
    git-cliff
    cargo-zigbuild
    # zig
  ];
  nativeBuildInputs = with pkgs; [
    pkg-config
  ];

  ONEPASSWORD_SKIP_DOWNLOAD = "1";
  ONEPASSWORD_LIB_PATH = "${onepasswordSdk}/lib";

  shellHook = ''
    export PKG_CONFIG_PATH="${pkgs.openssl.dev}/lib/pkgconfig:$PKG_CONFIG_PATH"
    export OPENSSL_DIR="${pkgs.openssl.dev}"
    export OPENSSL_LIB_DIR="${pkgs.openssl.out}/lib"
    export LD_LIBRARY_PATH="${onepasswordSdk}/lib:$LD_LIBRARY_PATH"
  '';
  RUST_LOG = "debug";
  RUST_BACKTRACE = 1;
}
