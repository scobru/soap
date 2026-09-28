# Clear Voice

Voice cleanup (denoise, dereverb, loudness normalization) built on
[Clear](https://desertant.com/models/clear/) from Desert Ant Labs, a fine-tuned
DeepFilterNet 3 that runs entirely on-device.

## Web interface (`web/`)

A static browser app: audio stays on the user's machine, and inference runs in
WebAssembly (LiteRT.js, XNNPACK CPU) or WebGPU.

```bash
cd web
npm install
npm run dev      # http://localhost:5173
npm run build    # static site in web/dist/
```

Features:

- Load any file the browser can decode (WAV, MP3, M4A, OGG, FLAC, video audio tracks), or record from the mic with the browser's DSP turned off
- Settings: strength (original/clean blend), LUFS target (Apple Podcasts −19, Spotify/YouTube −14, EBU R128 −23, custom, off), true-peak ceiling, max gain, mono or stereo, 48/44.1 kHz output, CPU or WebGPU
- Before/after comparison with waveforms, click-to-seek, and A/B switching (`A`/`B` keys) that keeps the playback position
- Export WAV as 16-bit PCM or 32-bit float

### Deployment notes

- The model weights (`desert-ant-labs/clear` on Hugging Face, pinned by the SDK) are downloaded on first use and cached by the browser. To serve them yourself, set `VITE_CLEAR_MODEL_BASE_URL` at build time.
- The LiteRT.js runtime is copied into `public/litert/` (the `copy-litert` script) and served from your own origin instead of jsDelivr.
- For the multi-threaded wasm runtime, the host must send `Cross-Origin-Opener-Policy: same-origin` and `Cross-Origin-Embedder-Policy: credentialless`. The Vite dev and preview servers already do. Without them it still works, just slower.
- The SDK core weighs about 46 MB (about 19 MB gzipped).

## License and attribution

Clear is distributed under the [Desert Ant Labs Source-Available License](https://license.desertant.com/1.0).
It is not an OSI open-source license:

- Free up to 100,000 monthly active devices per platform, per model. Above that you need a commercial license.
- Desert Ant Labs must be credited in the app (the web UI does this in the footer).
- You may not use the model or its outputs to train competing models.
- The SDK sends Desert Ant Labs an active-device count (usage telemetry). It never sends the audio.
