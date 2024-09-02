[Back to Readme](../README.md)

# Installation

## Dependencies

- git (optional for `jaws diff` and `jaws status` command)

## Install Nix

```sh
nix profile install 'github:jacbart/jaws'
```

## Install Brew

```sh
brew tap jacbart/homebrew-taps
brew install jaws
```

## Install latest released binary

This script will download the latest released jaws binary for your system and move it into `~/.local/bin`.

```sh
curl -sfL https://raw.githubusercontent.com/jacbart/jaws/main/install.sh | sh
```

## Install manually

go to the releases page of [jaws](https://github.com/jacbart/jaws/releases) and download the tar file for your computer. Then install the binary using the commands below. Or just open the archive and move the jaws binary to your PATH.

```sh
TMPDIR="$(mktemp -d)"
tar -xf ~/Downloads/jaws_*.tar.gz -C $TMPDIR
mv $TMPDIR/jaws ~/.local/bin
jaws version # check it is in your PATH and working
# If you are on an Apple computer you might need to go into Security & Privacy to allow the executable
rm -rf $TMPDIR
```

## Install with golang

**Dependencies**

- golang >=1.22

```sh
go install github.com/jacbart/jaws/cmd/jaws@latest
```

## Build from source

```sh
git clone git@github.com:jacbart/jaws.git && cd jaws
CGO_ENABLED=0 go build -ldflags "-s -w -X 'main.Version=0.1.9-rc' -X 'main.Date=YYYY-MM-DD'" ./cmd/jaws
```

or with the `just` cli tool

```sh
just build
mv ./dist/*/jaws /SOMEWHERE/IN/YOUR/$PATH
```

# Updating

To use the self update you either need to set the env variable `GH_TOKEN` or add `gh_token` to your `jaws.conf` under the general settings and set it to your github personal access token. Setting one of those is required due to jaws being on a private repo at the time of writing this.

```sh
jaws update
# checks if there is a newer version released on github
# if so it downloads the release for your computer, un-tars the archive, tests the binary
# then backs up the old cli to jaws.old and moves the new one into your PATH
```
