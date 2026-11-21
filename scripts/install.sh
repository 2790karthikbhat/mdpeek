#!/bin/sh
# Install the latest mdpeek release on macOS or Linux. Non-interactive, safe to re-run.
#
#   curl -fsSL https://raw.githubusercontent.com/2790karthikbhat/mdpeek/main/scripts/install.sh | sh
#
# Downloads the prebuilt binary for this OS and CPU, checks its SHA-256 and installs it to
# ~/.local/bin/mdpeek. Options (environment variables):
#   MDPEEK_VERSION=v0.1.0       install that release instead of the latest
#   MDPEEK_INSTALL_DIR=/path    install somewhere other than ~/.local/bin
#   MDPEEK_DOWNLOAD_URL=url     base URL holding the release files (mirrors, testing)
set -eu

REPO="2790karthikbhat/mdpeek"
VERSION="${MDPEEK_VERSION:-latest}"
DIR="${MDPEEK_INSTALL_DIR:-$HOME/.local/bin}"

die() {
  echo "error: $*" >&2
  exit 1
}

command -v curl >/dev/null 2>&1 || die "curl is required"
command -v tar >/dev/null 2>&1 || die "tar is required"

os="$(uname -s)"
arch="$(uname -m)"
case "$os/$arch" in
  Darwin/arm64 | Darwin/aarch64) target="aarch64-apple-darwin" ;;
  Darwin/x86_64) target="x86_64-apple-darwin" ;;
  Linux/x86_64 | Linux/amd64) target="x86_64-unknown-linux-musl" ;;
  Linux/aarch64 | Linux/arm64) target="aarch64-unknown-linux-musl" ;;
  *) die "no prebuilt binary for $os $arch. Install with Rust instead: cargo install mdpeek" ;;
esac

if [ -n "${MDPEEK_DOWNLOAD_URL:-}" ]; then
  base="$MDPEEK_DOWNLOAD_URL"
elif [ "$VERSION" = "latest" ]; then
  base="https://github.com/$REPO/releases/latest/download"
else
  base="https://github.com/$REPO/releases/download/$VERSION"
fi

asset="mdpeek-$target.tar.gz"
tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT INT TERM

echo "Downloading $base/$asset"
curl -fsSL -o "$tmp/$asset" "$base/$asset" ||
  die "download failed. Check that release $VERSION exists: https://github.com/$REPO/releases"
curl -fsSL -o "$tmp/$asset.sha256" "$base/$asset.sha256" || die "checksum download failed"

expected="$(cut -d ' ' -f 1 <"$tmp/$asset.sha256")"
if command -v sha256sum >/dev/null 2>&1; then
  actual="$(sha256sum "$tmp/$asset" | cut -d ' ' -f 1)"
elif command -v shasum >/dev/null 2>&1; then
  actual="$(shasum -a 256 "$tmp/$asset" | cut -d ' ' -f 1)"
else
  die "need sha256sum or shasum to verify the download"
fi
[ "$expected" = "$actual" ] || die "checksum mismatch for $asset (expected $expected, got $actual)"

tar -xzf "$tmp/$asset" -C "$tmp"
bin="$tmp/mdpeek-$target/mdpeek"
[ -f "$bin" ] || die "archive did not contain mdpeek"

mkdir -p "$DIR"
# Copy then rename, so a running mdpeek is never left half-written.
cp "$bin" "$DIR/.mdpeek.new"
chmod 755 "$DIR/.mdpeek.new"
mv -f "$DIR/.mdpeek.new" "$DIR/mdpeek"

echo "Installed $("$DIR/mdpeek" --version) to $DIR/mdpeek"
case ":$PATH:" in
  *":$DIR:"*) echo "Run: mdpeek README.md" ;;
  *)
    # The tilde is printed for the user to paste, not expanded here.
    # shellcheck disable=SC2088
    case "${SHELL:-}" in
      */zsh) rc="~/.zshrc" ;;
      */bash) rc="~/.bashrc" ;;
      *) rc="~/.profile" ;;
    esac
    echo "$DIR is not on your PATH. Add it, then open a new terminal:"
    echo "  echo 'export PATH=\"$DIR:\$PATH\"' >> $rc"
    ;;
esac
