Remove That Dirt (RTD)
======================

Offline voice cleanup for your DAW: denoise, dereverb and loudness
normalization with the Clear model by Desert Ant Labs.

Install
-------
  Windows : double-click install.cmd (asks for admin rights; installs to
            C:\Program Files\Common Files\CLAP and \VST3). Run
            "install.ps1 -User" to install for your user only.
  macOS   : run ./install.sh (installs CLAP, VST3 and AU to
            ~/Library/Audio/Plug-Ins). Apple Silicon, macOS 14 or later.
  Linux   : run ./install.sh (installs to ~/.clap and ~/.vst3).

On Windows the VST3 loads the CLAP from the CLAP folder: install both.

Use
---
  1. Insert Remove That Dirt on the voice track.
  2. Press Capture, then play the part to clean in your DAW.
  3. When the transport stops, the take is cleaned (the model is downloaded
     once, the first time).
  4. Play the project: the clean take replaces the input over that region.
     A / B switches between original and clean.

Takes are saved as WAV files (Open folder in the plugin shows where), so you
can also drag the clean file straight into your project.

License
-------
Voice enhancement by Clear from Desert Ant Labs, under the Desert Ant Labs
Source-Available License (https://license.desertant.com/1.0): free below
100,000 monthly active devices. The SDK reports an anonymous active-device
count to Desert Ant Labs; your audio never leaves your computer.
