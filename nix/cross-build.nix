# Cross-compilation builder using cargo-zigbuild
#
# This derivation uses cargo-zigbuild to cross-compile the project
# for different target architectures from any host system.
#
# Usage in flake.nix:
#   pkgs.callPackage ./nix/cross-build.nix {
#     targetTriple = "aarch64-unknown-linux-gnu";
#     targetSystem = "aarch64-linux";
#     ...
#   }
#
# Note: Cross-compiling to Darwin targets requires macOS SDK headers
# which are only available when building on macOS. For best results:
# - Linux targets: Can be cross-compiled from any system
# - Darwin targets: Best built on macOS using native cargo
{
  pkgs,
  lib,
  rustVersion,
  self,
  version,
  pname,
  # Rust target triple (e.g., "aarch64-unknown-linux-gnu")
  targetTriple,
  # Nix system string for the target (e.g., "aarch64-linux")
  targetSystem,
}:

let
  # Get the 1Password SDK for the target platform
  onepasswordSdk = pkgs.callPackage ./onepassword-sdk.nix {
    inherit targetSystem;
  };

  # Determine target platform characteristics
  isTargetLinux = builtins.match ".*-linux-.*" targetTriple != null;
  isTargetDarwin = builtins.match ".*-apple-darwin" targetTriple != null;

  # Determine host platform characteristics
  isHostDarwin = pkgs.stdenv.isDarwin;

  # Library extension for the target platform
  libExt = if isTargetDarwin then "dylib" else "so";

  # For Linux targets, use vendored OpenSSL (compiles from source)
  # For Darwin targets built on Darwin, we can use system OpenSSL
  cargoFeatures = if isTargetLinux then "--features vendored-openssl" else "";

  # For Darwin targets, use native cargo instead of cargo-zigbuild
  # since zigbuild has issues with macOS SDK headers
  useCargoBuild = isTargetDarwin && isHostDarwin;

  # Rust toolchain with the target added
  rustToolchain = rustVersion.override {
    targets = [ targetTriple ];
  };

  outputHashes = {
    "ff-1.0.10" = "sha256-pAuqe5ivORrBM22dF+55wVMz9/f9XTNI1jjhM9GPQpc=";
  };

in
pkgs.stdenv.mkDerivation {
  pname = "${pname}-${targetTriple}";
  inherit version;

  src = lib.cleanSource self;

  nativeBuildInputs =
    with pkgs;
    [
      rustToolchain
      pkg-config
      makeWrapper
      git
      cacert # For HTTPS git operations
      perl # Required for building vendored OpenSSL
    ]
    ++ lib.optionals (!useCargoBuild) [
      cargo-zigbuild
      zig
    ];

  # Build-time dependencies (for the build host)
  buildInputs = with pkgs; [
    openssl
    openssl.dev
  ];

  # Environment variables for the build
  RUSTUP_TOOLCHAIN = "none";

  # Configure OpenSSL to use vendored version for cross-compilation
  OPENSSL_STATIC = "1";
  OPENSSL_NO_VENDOR = "0";

  # 1Password SDK configuration
  ONEPASSWORD_SKIP_DOWNLOAD = "1";
  ONEPASSWORD_LIB_PATH = "${onepasswordSdk}/lib";

  # Set HOME to a writable directory for cargo-zigbuild cache
  HOME = "/tmp";

  # Cargo configuration for git dependencies
  configurePhase = ''
    runHook preConfigure

    # Set up cargo home and cache directories
    export CARGO_HOME="$PWD/.cargo-home"
    mkdir -p "$CARGO_HOME"

    # Ensure cargo-zigbuild has a writable cache directory
    export XDG_CACHE_HOME="$PWD/.cache"
    mkdir -p "$XDG_CACHE_HOME"

    # Create .cargo/config.toml for the build
    mkdir -p .cargo
    cat > .cargo/config.toml << 'EOF'
    [net]
    git-fetch-with-cli = true

    [registries.crates-io]
    protocol = "sparse"
    EOF

    runHook postConfigure
  '';

  buildPhase = ''
    runHook preBuild

    # Ensure environment variables are set (in case configurePhase exports didn't persist)
    export CARGO_HOME="$PWD/.cargo-home"
    export XDG_CACHE_HOME="$PWD/.cache"

    echo "Building for target: ${targetTriple}"
    echo "Using ${if useCargoBuild then "native cargo" else "cargo-zigbuild"} for compilation"

    # Note: We don't use --locked because the source directory from Nix is read-only
    # and cargo may need to update the lock file for git dependencies.
    # The Cargo.lock from the source is already authoritative.
    ${
      if useCargoBuild then
        ''
          # Use native cargo for Darwin-to-Darwin builds
          cargo build \
            --release \
            --target ${targetTriple}
        ''
      else
        ''
          # Use cargo-zigbuild for cross-compilation (Linux targets)
          # For Linux targets, use vendored OpenSSL (compiles from source)
          cargo zigbuild \
            --release \
            --target ${targetTriple} \
            ${cargoFeatures}
        ''
    }

    runHook postBuild
  '';

  installPhase = ''
    runHook preInstall

    # Install the binary
    mkdir -p $out/bin
    cp target/${targetTriple}/release/${pname} $out/bin/

    # Install the 1Password SDK library
    mkdir -p $out/lib
    cp ${onepasswordSdk}/lib/libop_uniffi_core.${libExt} $out/lib/ || true

    # Wrap the binary to set library paths
    wrapProgram $out/bin/${pname} \
      --set ONEPASSWORD_LIB_PATH "$out/lib/libop_uniffi_core.${libExt}" \
      ${if isTargetLinux then ''--prefix LD_LIBRARY_PATH : "$out/lib"'' else ""}

    runHook postInstall
  '';

  # Skip fixup phase for cross-compiled binaries
  # (we can't run patchelf on binaries for different architectures)
  dontFixup = true;

  meta = with lib; {
    description = "${pname} cross-compiled for ${targetTriple}";
    homepage = "https://github.com/jacbart/jaws";
    license = with licenses; [ mpl20 ];
    maintainers = with maintainers; [ jacbart ];
    platforms = platforms.all;
  };
}
