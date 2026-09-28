#!/usr/bin/env bash
# Install the Soap app for the current user: the program in
# ~/.local/share/soap-app, a `soap` command in ~/.local/bin and an entry in
# the applications menu.
set -euo pipefail
cd "$(dirname "$0")"
data="${XDG_DATA_HOME:-$HOME/.local/share}"
dest="$data/soap-app"
rm -rf "$dest"
mkdir -p "$dest" "$HOME/.local/bin" "$data/applications"
cp -R Soap/. "$dest/"
ln -sf "$dest/soap" "$HOME/.local/bin/soap"
cat > "$data/applications/soap.desktop" <<DESKTOP
[Desktop Entry]
Type=Application
Name=Soap
GenericName=Voice cleaner
Comment=Denoise, dereverb and level voice recordings on your computer
Exec="$dest/soap" %f
Icon=$dest/soap.png
Terminal=false
Categories=AudioVideo;Audio;
MimeType=audio/wav;audio/x-wav;audio/flac;audio/mpeg;audio/mp4;audio/ogg;audio/aiff;
StartupWMClass=soap
DESKTOP
update-desktop-database "$data/applications" >/dev/null 2>&1 || true
echo "Soap installed: find it in your applications menu, or run 'soap'."
echo "Clear needs libcurl4 to download its model the first time."
