#!/usr/bin/env bash
# Install Remove That Dirt for the current user (CLAP, VST3 and AU).
set -euo pipefail
cd "$(dirname "$0")"
NAME="Remove That Dirt"
base="$HOME/Library/Audio/Plug-Ins"
mkdir -p "$base/CLAP" "$base/VST3" "$base/Components"
rm -rf "$base/CLAP/$NAME.clap" "$base/VST3/$NAME.vst3" "$base/Components/$NAME.component"
cp -R "$NAME.clap" "$base/CLAP/"
cp -R "$NAME.vst3" "$base/VST3/"
cp -R "$NAME.component" "$base/Components/"
# Downloaded packages are quarantined; unsigned plugins would be refused.
xattr -dr com.apple.quarantine "$base/CLAP/$NAME.clap" "$base/VST3/$NAME.vst3" "$base/Components/$NAME.component" 2>/dev/null || true
killall -9 AudioComponentRegistrar 2>/dev/null || true
echo "Installed in $base. Rescan plugins in your DAW."
