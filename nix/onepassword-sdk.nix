{
  lib,
  stdenv,
  fetchurl,
  unzip,
  autoPatchelfHook,
}:

let
  version = "0.3.2";

  sources = {
    x86_64-linux = {
      url = "https://files.pythonhosted.org/packages/cb/eb/86f450df04eccbc1317b8316061e768c93554e9fc0facb0185dcf60ef387/onepassword_sdk-0.3.2-cp39-cp39-manylinux_2_32_x86_64.whl";
      sha256 = "963af1fa1466e794783a657ac25a93d88b0c1e6330354fd8509aeaf0acc9164c";
    };
    aarch64-linux = {
      url = "https://files.pythonhosted.org/packages/b3/6d/81a56216dcc1db7c0e259da3b7c9c290158d0f5ab1c93ed2ab78a8fb9829/onepassword_sdk-0.3.2-cp39-cp39-manylinux_2_32_aarch64.whl";
      sha256 = "be4382eb5d007c153b0c6c8c096c46280b9d35b513f44bc0fe09a2078dda563f";
    };
    x86_64-darwin = {
      url = "https://files.pythonhosted.org/packages/84/c9/c57b228b379375e71d06084b7f7b26dfb874b994d0db96e30553cc2b34aa/onepassword_sdk-0.3.2-cp39-cp39-macosx_10_9_x86_64.whl";
      sha256 = "9116696cd26f53419423128a9e15c4d634921b9c82d1e6ebf038a9dc3eb795af";
    };
    aarch64-darwin = {
      url = "https://files.pythonhosted.org/packages/04/72/e75e54e985a4affdf0e5d9b5a9c3945dc0583ce32c1f62d6ee0faef78c41/onepassword_sdk-0.3.2-cp39-cp39-macosx_11_0_arm64.whl";
      sha256 = "76bb47fd06ef4f8231e0d04c85606dee66c37f50ef0541fc24e88d56a5156931";
    };
  };

  system = stdenv.hostPlatform.system;
  src = sources.${system} or (throw "Unsupported system: ${system}");

in
stdenv.mkDerivation {
  pname = "onepassword-sdk";
  inherit version;

  src = fetchurl {
    inherit (src) url sha256;
  };

  nativeBuildInputs = [ unzip ] ++ lib.optionals stdenv.isLinux [ autoPatchelfHook ];

  buildInputs = lib.optionals stdenv.isLinux [ stdenv.cc.cc.lib ];

  unpackPhase = ''
    unzip $src
  '';

  installPhase = ''
    mkdir -p $out/lib
    find . -name "*.so" -o -name "*.dylib" -o -name "*.dll" | xargs -I {} cp {} $out/lib/
  '';
}
