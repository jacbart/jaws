set shell := ["zsh", "-c"]

alias b := build
alias c := clean

build:
  goreleaser build --single-target --clean --snapshot

build-docker:
  jaws pull -i docker.jaws
  docker build -f Dockerfile . -t jaws:dev
  @ docker run -it jaws:dev jaws version

run-docker +ARGS:
  @ echo "jaws {{ARGS}}"
  @ docker run -it --mount type=bind,source="$(pwd)"/jaws.conf,target=/app/jaws.conf jaws:dev jaws {{ARGS}}

install:
  @ just build
  mv ./dist/*/jaws ~/.local/bin/
  @ just clean

build-all:
  goreleaser build --clean --snapshot

test-unit:
  go test -tags=unit github.com/jacbart/jaws/... -v -cover

test-integration:
  go test -tags=integration github.com/jacbart/jaws/cmd/jaws/... -v -cover

release:
  GITHUB_TOKEN=$(bw get notes gh-releaser) goreleaser release --clean

clean:
  rm -f ./jaws
  rm -rf ./dist
