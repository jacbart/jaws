#!/bin/sh
set -e

RELEASES_URL="https://github.com/jacbart/jaws/releases"
FILE_BASENAME="jaws"

test -z "$VERSION" && VERSION="$(curl -sfL -o /dev/null -w %{url_effective} "$RELEASES_URL/latest" |
		rev |
		cut -f1 -d'/'|
		rev)"

test -z "$VERSION" && {
	echo "Unable to get jaws version." >&2
	exit 1
}

test -z "$TMPDIR" && TMPDIR="$(mktemp -d)"
export TAR_FILE="$TMPDIR/${FILE_BASENAME}_$(echo "$VERSION" | cut -c 2-)_$(uname -s)_$(uname -m).tar.gz"

(
	cd "$TMPDIR"
	echo "Downloading jaws $VERSION..."
	curl -sfLo "$TAR_FILE" \
		"$RELEASES_URL/download/$VERSION/${FILE_BASENAME}_$(echo "$VERSION" | cut -c 2-)_$(uname -s)_$(uname -m).tar.gz"
)

tar -xf "$TAR_FILE" -C "$TMPDIR"
"${TMPDIR}/jaws" "version"

mv "${TMPDIR}/jaws" "$HOME/.local/bin/jaws"

