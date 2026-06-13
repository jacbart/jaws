{
  lib,
  stdenv,
  fetchurl,
  unzip,
  autoPatchelfHook,
  # Optional: override the target system for cross-compilation
  # If not provided, defaults to the host platform
  targetSystem ? null,
}:

let
  version = "0.4.0";

  sources = {
    x86_64-linux = {
      url = "https://files.pythonhosted.org/packages/20/d9/aabbec9ac27ddf42d062f5327f3da4b5ca1f5ce34b3319d5e0ec41fea67b/onepassword_sdk-0.4.0-cp39-cp39-manylinux_2_32_x86_64.whl";
      sha256 = "13vjh40yv06bzcp4b0j9g3nkg8qmf7a3dlcdibax0fvxshwaih37";
    };
    aarch64-linux = {
      url = "https://files.pythonhosted.org/packages/22/fb/54e1615166330f10bfaed806b2872bb3093a49ccb1e4a4e50ada7275e150/onepassword_sdk-0.4.0-cp39-cp39-manylinux_2_32_aarch64.whl";
      sha256 = "070nvdr8pyzhms53a0nzr42jgz5707496lpi34jrzlpb9sqdj9i5";
    };
    x86_64-darwin = {
      url = "https://files.pythonhosted.org/packages/2f/35/195045a31f950809477c8aec277ec39e5dc423ad8ffa35d9cd629d951256/onepassword_sdk-0.4.0-cp39-cp39-macosx_10_9_x86_64.whl";
      sha256 = "0k1d10fzjz3a29nnzr2kc7ga90gxnzyjfw1zspncxplyrllwhcy5";
    };
    aarch64-darwin = {
      url = "https://files.pythonhosted.org/packages/52/90/e1f5867161e20605a0b4f7c520327698b34e05c59844df37a7f6e70308e3/onepassword_sdk-0.4.0-cp39-cp39-macosx_11_0_arm64.whl";
      sha256 = "sha256-4XnWtpyW5b1AN19+sIGUpUaTtIB2NewWjP1v/0/ehOA=";
    };
  };

  # Use targetSystem if provided (for cross-compilation), otherwise use host platform
  system = if targetSystem != null then targetSystem else stdenv.hostPlatform.system;
  src = sources.${system} or (throw "Unsupported system: ${system}");

  # Determine if target is Linux or Darwin for library handling
  isTargetLinux = builtins.match ".*-linux" system != null;
  isTargetDarwin = builtins.match ".*-darwin" system != null;

in
stdenv.mkDerivation {
  pname = "onepassword-sdk";
  inherit version;

  src = fetchurl {
    inherit (src) url sha256;
  };

  # For cross-compilation, we only need unzip on the build machine
  # autoPatchelfHook is only needed when building for Linux AND running on Linux
  nativeBuildInputs = [
    unzip
  ]
  ++ lib.optionals (stdenv.isLinux && isTargetLinux) [ autoPatchelfHook ];

  buildInputs = lib.optionals (stdenv.isLinux && isTargetLinux) [ stdenv.cc.cc.lib ];

  unpackPhase = ''
    unzip $src
  '';

  installPhase = ''
    mkdir -p $out/lib
    find . -name "*.so" -o -name "*.dylib" -o -name "*.dll" | xargs -I {} cp {} $out/lib/
  '';

  # Metadata for cross-compilation awareness
  passthru = {
    inherit system isTargetLinux isTargetDarwin;
  };
}
