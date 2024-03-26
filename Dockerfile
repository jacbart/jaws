FROM golang:1.20-alpine3.16 AS builder

# Install goreleaser
RUN go install github.com/goreleaser/goreleaser@latest

# Install just cli
RUN apk add --update --no-cache git

WORKDIR /go/src/github.com/jacbart/jaws

# add mod and sum first to test if dependencies will download
ADD go.mod .
ADD go.sum .

# download deps
RUN go mod download

# add the src files
ADD ./cmd ./cmd
ADD ./integration ./integration
ADD ./pkg ./pkg
ADD ./utils ./utils

# add goreleaser
ADD .git .git
ADD ./.goreleaser.yaml ./.goreleaser.yaml

# build binary
RUN goreleaser build --single-target --rm-dist --snapshot

RUN mv ./dist/*/jaws .

# Run JAWS on alpine image
FROM alpine:3.16

# get ca certs to allow connection with aws
RUN apk --update --no-cache add ca-certificates

WORKDIR /app

# copy binary over
COPY --from=builder /go/src/github.com/jacbart/jaws/jaws .

# add to path
ENV PATH /app:$PATH

CMD [ "/app/jaws" ]