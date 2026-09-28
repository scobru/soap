Soap
====

Clean voice recordings on your computer: denoise, dereverb and loudness
normalization with the Clear model by Desert Ant Labs.

Install
-------
  macOS   : drag Soap.app to Applications. Apple Silicon, macOS 14 or later.
            The app is not notarized: the first time, open it, then go to
            System Settings > Privacy & Security and click "Open Anyway"
            (or run: xattr -dr com.apple.quarantine /Applications/Soap.app).
  Windows : double-click install.cmd (installs for your user, adds Soap to the
            Start menu), or just run Soap\Soap.exe from this folder. Windows
            may warn that the app is unrecognized: More info > Run anyway.
  Linux   : run ./install.sh (adds Soap to the applications menu and a
            `soap` command), or run Soap/soap directly. Needs libcurl4.

Use
---
  1. Drop a recording on the window, or press Open audio. WAV, AIFF, FLAC,
     MP3, M4A/AAC, ALAC and Ogg Vorbis work.
  2. Soap washes it right away (the model is downloaded once, the first time).
  3. Play and compare: A is the original, B the clean version. Space plays and
     pauses; click a waveform to jump.
  4. Change the settings and press Clean again, then Save WAV.

From a terminal, `soap --clean input.mp3 output.wav` (Soap.exe on Windows,
Soap.app/Contents/MacOS/Soap on macOS) does the same without a window; add
--help for the options.

License
-------
Voice enhancement by Clear from Desert Ant Labs, under the Desert Ant Labs
Source-Available License (https://license.desertant.com/1.0): free below
100,000 monthly active devices. The SDK reports an anonymous active-device
count to Desert Ant Labs; your audio never leaves your computer.
