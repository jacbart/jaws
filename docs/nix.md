# Nix

## Table of Contents

- [Flake](#flake)
- [Running Without Installing](#running-without-installing)
- [Installing](#installing)
- [Overlay](#overlay)
- [Home Manager Module](#home-manager-module)
- [Cross-Compiled Binaries](#cross-compiled-binaries)
- [Development Shell](#development-shell)

---

## Flake

JAWS is distributed as a Nix flake. Add it to your inputs:

```nix
{
  inputs.jaws.url = "github:jacbart/jaws";

  outputs = { self, nixpkgs, jaws, ... }: {
    # Use jaws.packages.${system}.default
  };
}
```

## Running Without Installing

```bash
nix run github:jacbart/jaws -- --version
```

## Installing

### To your profile

```bash
nix profile install github:jacbart/jaws
```

### As a system package (NixOS)

```nix
{ inputs, pkgs, ... }:
{
  environment.systemPackages = [ inputs.jaws.packages.${pkgs.system}.default ];
}
```

### Via overlay

```nix
{ inputs, ... }:
{
  nixpkgs.overlays = [ inputs.jaws.overlays.default ];
  # Now pkgs.jaws is available everywhere
}
```

---

## Overlay

The flake exports an overlay that adds `jaws` to `pkgs`:

```nix
{
  inputs.jaws.url = "github:jacbart/jaws";

  outputs = { self, nixpkgs, jaws, ... }:
    let
      pkgs = import nixpkgs {
        system = "x86_64-linux";
        overlays = [ jaws.overlays.default ];
      };
    in {
      # pkgs.jaws is now available
    };
}
```

---

## Home Manager Module

The flake includes a Home Manager module for declarative configuration:

```nix
{ inputs, ... }:
{
  imports = [ inputs.jaws.homeManagerModules.default ];

  programs.jaws = {
    enable = true;
    settings = {
      editor = "nvim";
      secretsPath = "~/.jaws/secrets";
      cacheTtl = 900;
      defaultProvider = "jaws";
      maxVersions = 10;
    };
    providers = {
      aws-prod = {
        kind = "aws";
        profile = "production";
        region = "us-east-1";
      };
      op-team = {
        kind = "onepassword";
        vault = "abc123";
      };
      gcp-prod = {
        kind = "gcp";
        project = "my-project-id";
      };
    };
  };
}
```

This generates `~/.config/jaws/jaws.hcl` from the declared options.

### Provider Options

| Option | Types | Description |
|--------|-------|-------------|
| `kind` | all | Provider type: `aws`, `onepassword`, `bw`, `gcp` |
| `profile` | aws | AWS profile name, or `"all"` for auto-discovery |
| `region` | aws | AWS region |
| `vault` | onepassword, bw | Vault UUID or `"all"` |
| `organization` | bw | Bitwarden organization UUID |
| `tokenEnv` | bw | Environment variable for access token |
| `project` | gcp | GCP project ID |

---

## Cross-Compiled Binaries

Build for other platforms without a native toolchain:

```bash
# Intel/AMD Linux
nix build .#jaws-x86_64-linux

# ARM64 Linux (AWS Graviton, Raspberry Pi)
nix build .#jaws-aarch64-linux

# Intel Mac
nix build .#jaws-x86_64-darwin

# Apple Silicon Mac
nix build .#jaws-aarch64-darwin
```

The derivation uses `cargo-zigbuild` for Linux targets to avoid needing full cross-compilation toolchains.

---

## Development Shell

Enter a shell with all build dependencies:

```bash
nix develop
```

Or add to your flake:

```nix
devShells.default = jaws.devShells.${system}.default;
```

---

See also:
- [Configuration](configuration.md) — `jaws.hcl` format and provider setup
- [Development](development.md) — building, testing, releases
- [Getting Started](getting-started.md) — first-time user walkthrough
