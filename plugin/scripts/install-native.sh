#!/usr/bin/env sh
# Install the prebuilt Clear native core (from the @desert-ant-labs/clear npm
# package) where the plugin looks for it. Run once per machine.
set -eu

VERSION="${CLEAR_SDK_VERSION:-3.5.0}"

case "$(uname -s)-$(uname -m)" in
  Linux-x86_64) key=linux-x64 ;;
  Linux-aarch64 | Linux-arm64) key=linux-arm64 ;;
  Darwin-arm64) key=darwin-arm64 ;;
  *)
    echo "Clear has no native build for $(uname -s) $(uname -m)." >&2
    echo "Supported: Linux x86_64/arm64, macOS Apple Silicon." >&2
    exit 1
    ;;
esac

case "$key" in
  darwin-*) data="$HOME/Library/Application Support" ;;
  *) data="${XDG_DATA_HOME:-$HOME/.local/share}" ;;
esac
target="${SOAP_NATIVE_DIR:-$data/soap-voice/native/$key}"

tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT

echo "Downloading @desert-ant-labs/clear ${VERSION}..."
curl -fsSL "https://registry.npmjs.org/@desert-ant-labs/clear/-/clear-$VERSION.tgz" | tar -xz -C "$tmp"

mkdir -p "$target"
cp "$tmp/package/native/$key/"* "$target/"
cp "$tmp/package/LICENSE.md" "$target/LICENSE.md"
echo "Clear native core installed in: $target"
echo "The model weights (Hugging Face) are downloaded the first time you press Clean."
