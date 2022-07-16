alias b := build
alias c := clean

date := `date "+%Y-%m-%d"`
version := "v0.1.1-rc"

build:
    CGO_ENABLED=0 go build -ldflags "-s -w -X 'main.Version={{version}}' -X 'main.Date={{date}}'" ./cmd/jaws

release:
    goreleaser --rm-dist

clean:
    rm ./jaws
    rm -rf ./dist