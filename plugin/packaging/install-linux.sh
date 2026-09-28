#!/usr/bin/env bash
# Install Soap for the current user (CLAP and VST3).
set -euo pipefail
cd "$(dirname "$0")"
NAME="Soap"
mkdir -p "$HOME/.clap" "$HOME/.vst3"
rm -rf "$HOME/.clap/$NAME" "$HOME/.vst3/$NAME.vst3"
cp -R "CLAP/$NAME" "$HOME/.clap/"
cp -R "VST3/$NAME.vst3" "$HOME/.vst3/"
echo "Installed in ~/.clap and ~/.vst3. Clear needs libcurl4 to download its model."
