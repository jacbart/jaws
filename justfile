alias b := build
alias c := clean

date := `date "+%Y-%m-%d"`
version := `git tag --points-at HEAD --sort -version:refname`

build:
    CGO_ENABLED=1 go build -ldflags "-s -w -X 'main.Version={{version}}-rc' -X 'main.Date={{date}}'" ./cmd/jaws

release:
    goreleaser --rm-dist

clean:
    rm -f ./jaws
    rm -rf ./dist