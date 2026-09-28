#!/usr/bin/env bash
# Build the release package for macOS (CLAP + VST3 + AU) or Linux (CLAP +
# VST3), with the Clear native core inside. Run from plugin/.
set -euo pipefail

VERSION="${VERSION:-dev}"
NAME="Soap"

case "$(uname -s)-$(uname -m)" in
  Darwin-arm64) os=macos key=darwin-arm64 ;;
  Linux-x86_64) os=linux key=linux-x64 ;;
  Linux-aarch64) os=linux-arm64 key=linux-arm64 ;;
  *) echo "Unsupported build host: $(uname -s) $(uname -m)" >&2; exit 1 ;;
esac

cargo xtask bundle soap --release
clap="$PWD/target/bundled/$NAME.clap"

native="$PWD/target/native/$key"
if [ ! -e "$native/LICENSE.md" ]; then
  SOAP_NATIVE_DIR="$native" ./scripts/install-native.sh
fi

out="$PWD/target/dist/soap-$VERSION-$os"
rm -rf "$out" && mkdir -p "$out"

find_bundle() { find target/wrapper -maxdepth 3 -name "$1" -print -quit; }

if [ "$os" = macos ]; then
  # The core goes inside the CLAP, which the VST3 and AU embed in turn.
  mkdir -p "$clap/Contents/Resources/native"
  cp "$native/libClearNode.dylib" "$native/LICENSE.md" "$clap/Contents/Resources/native/"
  codesign --force --sign - "$clap/Contents/Resources/native/libClearNode.dylib"
  codesign --force --sign - "$clap"

  cmake -S wrapper -B target/wrapper -DCMAKE_BUILD_TYPE=Release -DSOAP_CLAP_BUNDLE="$clap"
  cmake --build target/wrapper --config Release -j 4

  cp -R "$clap" "$out/"
  cp -R "$(find_bundle "$NAME.vst3")" "$(find_bundle "$NAME.component")" "$out/"
  # Ad-hoc signatures: enough to load on Apple Silicon. Distribution to other
  # Macs without warnings needs a Developer ID signature and notarization.
  codesign --force --deep --sign - "$out/$NAME.vst3" "$out/$NAME.component"
  cp packaging/install-macos.sh "$out/install.sh"
else
  mkdir -p "$out/CLAP/$NAME" "$out/VST3"
  cp "$clap" "$out/CLAP/$NAME/"
  cp "$native"/*.so "$native/LICENSE.md" "$out/CLAP/$NAME/"

  cmake -S wrapper -B target/wrapper -DCMAKE_BUILD_TYPE=Release
  cmake --build target/wrapper --config Release -j 4
  cp -R "$(find_bundle "$NAME.vst3")" "$out/VST3/"
  cp packaging/install-linux.sh "$out/install.sh"
fi

cp packaging/README.txt "$out/"
(cd "$(dirname "$out")" && rm -f "$(basename "$out").zip" && zip -qry "$(basename "$out").zip" "$(basename "$out")")
echo "Package: $out.zip"
