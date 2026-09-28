#!/usr/bin/env bash
# Build the Soap desktop app package: Soap.app on macOS, a folder with an
# installer on Linux, with the Clear native core inside. Run from plugin/.
set -euo pipefail

VERSION="${VERSION:-dev}"

case "$(uname -s)-$(uname -m)" in
  Darwin-arm64) os=macos key=darwin-arm64 ;;
  Linux-x86_64) os=linux key=linux-x64 ;;
  Linux-aarch64) os=linux-arm64 key=linux-arm64 ;;
  *) echo "Unsupported build host: $(uname -s) $(uname -m)" >&2; exit 1 ;;
esac

native="$PWD/target/native/$key"
if [ ! -e "$native/LICENSE.md" ]; then
  SOAP_NATIVE_DIR="$native" ./scripts/install-native.sh
fi

[ "$os" = macos ] && export MACOSX_DEPLOYMENT_TARGET=14.0
cargo build --release -p soap-app
bin="$PWD/target/release/soap-app"
app_version=$(sed -n 's/^version = "\(.*\)"/\1/p' app/Cargo.toml | head -1)

out="$PWD/target/dist/soap-app-$VERSION-$os"
rm -rf "$out" "$out.zip" && mkdir -p "$out"

if [ "$os" = macos ]; then
  app="$out/Soap.app"
  mkdir -p "$app/Contents/MacOS" "$app/Contents/Resources/native"
  cp "$bin" "$app/Contents/MacOS/Soap"
  cp app/assets/Soap.icns "$app/Contents/Resources/"
  cp "$native/libClearNode.dylib" "$native/LICENSE.md" "$app/Contents/Resources/native/"
  sed "s/@VERSION@/$app_version/g" packaging/app/Info.plist > "$app/Contents/Info.plist"
  # Ad-hoc signatures: enough to run on Apple Silicon. Opening without a
  # warning on other Macs needs a Developer ID signature and notarization.
  codesign --force --sign - "$app/Contents/Resources/native/libClearNode.dylib"
  codesign --force --sign - "$app"
  cp packaging/app/README.txt "$out/"
  (cd "$(dirname "$out")" && ditto -c -k --keepParent "$(basename "$out")" "$(basename "$out").zip")
else
  mkdir -p "$out/Soap/native"
  cp "$bin" "$out/Soap/soap"
  cp "$native"/*.so "$native/LICENSE.md" "$out/Soap/native/"
  cp app/assets/soap-256.png "$out/Soap/soap.png"
  cp packaging/app/install-linux.sh "$out/install.sh"
  cp packaging/app/README.txt "$out/"
  (cd "$(dirname "$out")" && zip -qry "$(basename "$out").zip" "$(basename "$out")")
fi
echo "Package: $out.zip"
