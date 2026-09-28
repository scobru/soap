# Clear Voice

Voice cleanup (denoise, dereverb, loudness normalization) built on
[Clear](https://desertant.com/models/clear/) from Desert Ant Labs, a fine-tuned
DeepFilterNet 3 that runs entirely on-device. It comes in two forms:

| | `web/` | `plugin/` |
|---|---|---|
| What it is | Static browser app | CLAP and VST3 plugin (Rust, nih-plug) |
| Runtime | WebAssembly + LiteRT.js (CPU) or WebGPU | Native Clear core (LiteRT on Linux, Core ML on macOS) |
| Platforms | Any modern browser | Linux x64/arm64, macOS Apple Silicon |
| Workflow | Load or record a file, clean it, compare, export WAV | Capture a track region, clean it, play back on the timeline |

Clear processes whole takes, not a real-time stream: loudness normalization
measures the integrated LUFS of the whole take. That's why the plugin works
"offline", Melodyne style.

## Web interface (`web/`)

```bash
cd web
npm install
npm run dev      # http://localhost:5173
npm run build    # static site in web/dist/
```

- Load any file the browser can decode (WAV, MP3, M4A, OGG, FLAC, video audio tracks), or record from the mic with the browser's DSP turned off
- Settings: strength, LUFS target (Podcast −19, Streaming −14, EBU R128 −23, custom, off), true-peak ceiling, max gain, mono or stereo, 48/44.1 kHz output, CPU or WebGPU
- Before/after comparison with waveforms, click-to-seek, and A/B switching (`A`/`B` keys) that keeps the playback position
- Export WAV as 16-bit PCM or 32-bit float

Deployment notes:

- The weights (`desert-ant-labs/clear` on Hugging Face, pinned by the SDK) are downloaded on first use and cached by the browser. To serve them yourself, set `VITE_CLEAR_MODEL_BASE_URL` at build time.
- The LiteRT.js runtime is copied into `public/litert/` and served from your own origin instead of jsDelivr.
- The multi-threaded runtime needs `Cross-Origin-Opener-Policy: same-origin` and `Cross-Origin-Embedder-Policy: credentialless` headers. The Vite dev and preview servers already send them. Without them it still works, just slower.
- The SDK core weighs about 46 MB (about 19 MB gzipped).

## Audio plugin (`plugin/`)

### Build

Prerequisites are Rust 1.88+ and, on Linux, the X11/ALSA/JACK headers (for example
`libx11-xcb-dev libxcursor-dev libgl-dev libasound2-dev libjack-jackd2-dev`).

```bash
cd plugin
cargo xtask bundle clear_voice --release
# → target/bundled/Clear Voice.clap and Clear Voice.vst3
```

Copy the bundles into your plugin folder (`~/.clap`, `~/.vst3` on Linux;
`~/Library/Audio/Plug-Ins/CLAP` and `VST3` on macOS).

### Install the Clear native core (once per machine)

The plugin loads the prebuilt Clear library at runtime rather than bundling it.
The script downloads it from the official npm package:

```bash
./plugin/scripts/install-native.sh
```

It installs to `~/.local/share/clear-voice/native/<platform>` on Linux or
`~/Library/Application Support/clear-voice/native/<platform>` on macOS. The
`CLEAR_VOICE_NATIVE_DIR` environment variable overrides the location. On Linux,
the library needs `libcurl4`. The model weights are downloaded from Hugging Face
the first time you press Clean.

To check the install outside a DAW:

```bash
cd plugin && cargo run --release --example enhance_wav -- noisy.wav clean.wav
```

### Usage

1. Put Clear Voice on the voice track.
2. Press **Capture**, then play the section you want to clean in your DAW.
3. When the transport stops, the take is cleaned in the background (turn off *Clean when capture stops* to do it by hand).
4. Play the project: over the captured region the plugin replaces the input with the clean version. **A · Original / B · Clean** switches between them instantly.
5. If you change strength, loudness, or other settings, press **Clean again**: it re-renders from the saved original, so you don't need to capture again.

Details:

- Capture follows the timeline position, so it handles loops, jumps, and several passes over the same region.
- The project saves only a reference to the take. The audio (original and clean, 32-bit float WAV) lives in `~/.local/share/clear-voice/takes/`, or `~/Library/Application Support/clear-voice/takes/` on macOS. **Open folder** takes you there, so you can also drag the clean file straight into the DAW.
- If the project's sample rate changes, the take is re-rendered automatically from the original.
- The audio thread never allocates or blocks. Capture goes through lock-free ring buffers, and download, inference, and disk I/O run on a worker thread.
- Windows isn't supported: Desert Ant Labs doesn't ship a native Clear core for Windows. There, use the web interface.

### Tests

```bash
cd plugin && cargo test
```

The tests cover the FFI payload encoding and decoding (the same format as the
SDK's `codec.js`), placing the capture on the timeline, playback, the WAV
round-trip, and resampling. The CLAP bundle passes `clap-validator` except for
the `state-reproducibility-*` tests, which nih-plug's own examples fail too.

## Licenses

- **Clear** (model, SDK, native core) is under the [Desert Ant Labs Source-Available License](https://license.desertant.com/1.0). It's not an OSI open-source license:
  - Free up to 100,000 monthly active devices per platform, per model. Above that you need a commercial license.
  - Desert Ant Labs must be credited in the app (both the web UI and the plugin do this).
  - You may not use the model or its outputs to train competing models.
  - The SDK sends Desert Ant Labs an active-device count (usage telemetry). It never sends the audio.
- **nih-plug** is ISC. The VST3 export, though, goes through `vst3-sys`, which is GPLv3: a distributed VST3 build falls under GPLv3. The CLAP build doesn't have that constraint.
