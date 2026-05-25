#!/usr/bin/env sh
set -eu

manifest_value() {
    sed -n "s/^$1 = \"\\(.*\\)\"/\\1/p" Cargo.toml | head -n 1
}

NAME=$(manifest_value name)
VERSION=$(manifest_value version)
HOST=$(rustc -vV | sed -n 's/^host: //p')
DIST=${DIST_DIR:-target/dist}
ARCHIVE_ROOT="$NAME-v$VERSION-$HOST"
ARCHIVE="$DIST/$ARCHIVE_ROOT.tar.gz"

if [ -z "$NAME" ] || [ -z "$VERSION" ] || [ -z "$HOST" ]; then
    echo "could not read package name, version, or host target" >&2
    exit 1
fi

cargo build --release --locked

mkdir -p "$DIST"
STAGING_PARENT=$(mktemp -d "$DIST/.staging.XXXXXX")
STAGING="$STAGING_PARENT/$ARCHIVE_ROOT"

mkdir -p \
    "$STAGING/bin" \
    "$STAGING/docs" \
    "$STAGING/share/man/man1" \
    "$STAGING/systemd"

cp "target/release/$NAME" "$STAGING/bin/$NAME"
cp Cargo.lock Cargo.toml CHANGELOG LICENSE README.md "$STAGING/"
cp ARCHITECTURE.md RELEASE_CHECKLIST.md ROADMAP.md "$STAGING/docs/"
cp nyancat.1 "$STAGING/share/man/man1/nyancat.1"
cp systemd/nyancat.socket systemd/nyancat@.service "$STAGING/systemd/"

tar -C "$STAGING_PARENT" -czf "$ARCHIVE" "$ARCHIVE_ROOT"

printf 'release archive: %s\n' "$ARCHIVE"
printf 'staging directory: %s\n' "$STAGING_PARENT"
