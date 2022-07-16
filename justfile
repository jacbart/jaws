alias b := build
alias c := clean

build:
    goreleaser build --single-target --rm-dist --snapshot

build-all:
    goreleaser build --rm-dist --snapshot

release:
    goreleaser release --rm-dist

clean:
    rm -f ./jaws
    rm -rf ./dist
