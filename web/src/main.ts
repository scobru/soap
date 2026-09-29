import "@fontsource/nunito/600.css";
import "@fontsource/nunito/800.css";
import type { Clear as ClearInstance, ClearResult, EnhanceOptions, LoudnessPreset } from "@desert-ant-labs/clear";
import type { Voz as VozInstance, VozResult } from "@desert-ant-labs/voz";
import { applyLanguage, getCurrentLanguage, onLanguageChange, setLanguage, t } from "./i18n";
import { toCues, toSrt, toText, toVtt } from "./subtitles";
import { encodeWav, type WavFormat } from "./wav";
import { Waveform } from "./waveform";

const MODEL_RATE = 48_000;

const $ = <T extends HTMLElement>(id: string) => document.getElementById(id) as T;

const ui = {
  modelStatus: $("model-status"),
  modelStatusText: $("model-status-text"),
  drop: $<HTMLLabelElement>("drop"),
  file: $<HTMLInputElement>("file"),
  record: $<HTMLButtonElement>("record"),
  recordTime: $("record-time"),
  sourceInfo: $("source-info"),
  strength: $<HTMLInputElement>("strength"),
  strengthOut: $<HTMLOutputElement>("strength-out"),
  loudness: $<HTMLSelectElement>("loudness"),
  customLufs: $<HTMLInputElement>("custom-lufs"),
  ceiling: $<HTMLInputElement>("ceiling"),
  maxGain: $<HTMLInputElement>("max-gain"),
  channels: $<HTMLSelectElement>("channels"),
  rate: $<HTMLSelectElement>("rate"),
  accelerator: $<HTMLSelectElement>("accelerator"),
  format: $<HTMLSelectElement>("format"),
  process: $<HTMLButtonElement>("process"),
  progress: $("progress"),
  progressBar: $("progress-bar"),
  error: $("error"),
  play: $<HTMLButtonElement>("play"),
  abOrig: $<HTMLButtonElement>("ab-orig"),
  abClean: $<HTMLButtonElement>("ab-clean"),
  clock: $("clock"),
  download: $<HTMLAnchorElement>("download"),
  stats: $("stats"),
  statLufs: $("stat-lufs"),
  statPeak: $("stat-peak"),
  statDur: $("stat-dur"),
  statRtf: $("stat-rtf"),
  txSource: $<HTMLSelectElement>("tx-source"),
  transcribe: $<HTMLButtonElement>("transcribe"),
  txProgress: $("tx-progress"),
  txProgressBar: $("tx-progress-bar"),
  txStatus: $("tx-status"),
  txError: $("tx-error"),
  transcript: $("transcript"),
  txExports: $("tx-exports"),
  dlSrt: $<HTMLAnchorElement>("dl-srt"),
  dlVtt: $<HTMLAnchorElement>("dl-vtt"),
  dlTxt: $<HTMLAnchorElement>("dl-txt"),
  txStats: $("tx-stats"),
};

type Source = { name: string; channels: Float32Array[]; sampleRate: number };

let source: Source | null = null;
let result: ClearResult | null = null;
let clear: ClearInstance | null = null;
let clearAccelerator: string | null = null;
let loading: Promise<ClearInstance> | null = null;
let busy = false;

// --- Language switch --------------------------------------------------------

document.querySelectorAll<HTMLButtonElement>(".lang-btn").forEach((btn) => {
  btn.addEventListener("click", () => {
    const lang = btn.dataset.lang as "it" | "en";
    if (lang) setLanguage(lang);
  });
});

onLanguageChange(() => {
  if (clear && clearAccelerator) {
    setModelStatus("ready", t("modelStatusReady")(clearAccelerator === "webgpu" ? "GPU" : "CPU"));
  } else if (!loading) {
    setModelStatus("idle", t("modelStatusIdle"));
  }
  ui.process.textContent = busy ? t("processButtonBusy") : t("processButton");
  ui.record.textContent = recorder ? t("recordStop") : t("recordMic");
  ui.play.textContent = player.playing ? t("pauseBtn") : t("playBtn");
  if (result) {
    ui.statRtf.textContent = t("realtimeFactor")(result.realtimeFactor.toFixed(1));
  }
  if (source) {
    const seconds = source.channels[0].length / source.sampleRate;
    ui.sourceInfo.textContent = `${source.name} · ${formatTime(seconds)} · ${source.channels.length === 2 ? "stereo" : "mono"}`;
  }
  ui.transcribe.textContent = transcribing ? t("txButtonBusy") : t("txButton");
  if (transcript) ui.txStats.textContent = t("txDone")(transcript.words.length, transcript.realtimeFactor.toFixed(0));
});

// --- Model -----------------------------------------------------------------

function setModelStatus(state: "idle" | "loading" | "ready" | "error", text: string) {
  ui.modelStatus.dataset.state = state;
  ui.modelStatusText.textContent = text;
}

async function getModel(): Promise<ClearInstance> {
  const accelerator = ui.accelerator.value as "wasm" | "webgpu";
  if (clear && clearAccelerator === accelerator) return clear;
  if (loading) return loading;

  clear?.dispose();
  clear = null;
  setModelStatus("loading", t("modelStatusLoadingRuntime"));
  loading = (async () => {
    const { Clear } = await import("@desert-ant-labs/clear");
    setModelStatus("loading", t("modelStatusDownloadingModel"));
    const model = await Clear.load({
      accelerator,
      litertWasmDir: new URL("litert/", document.baseURI).href,
      modelBaseUrl: import.meta.env.VITE_CLEAR_MODEL_BASE_URL || undefined,
      onProgress: (fraction) => {
        setModelStatus("loading", t("modelStatusDownloadingPct")(Math.round(fraction * 100)));
        setProgress(fraction);
      },
    });
    clear = model;
    clearAccelerator = accelerator;
    setModelStatus("ready", t("modelStatusReady")(accelerator === "webgpu" ? "GPU" : "CPU"));
    return model;
  })();
  try {
    return await loading;
  } catch (err) {
    setModelStatus("error", t("modelStatusError"));
    const offline = /download failed|Failed to fetch/i.test(String(err));
    throw offline
      ? new Error(t("errorHuggingFace"), { cause: err })
      : err;
  } finally {
    loading = null;
    if (!busy) ui.progress.hidden = true;
  }
}

// --- Input -----------------------------------------------------------------

async function decodeToModelRate(blob: Blob): Promise<{ channels: Float32Array[]; sampleRate: number }> {
  const ctx = new OfflineAudioContext(1, 1, MODEL_RATE);
  const buffer = await ctx.decodeAudioData(await blob.arrayBuffer());
  const channels = Array.from({ length: Math.min(2, buffer.numberOfChannels) }, (_, i) =>
    buffer.getChannelData(i).slice(),
  );
  return { channels, sampleRate: buffer.sampleRate };
}

async function loadSource(blob: Blob, name: string) {
  showError(null);
  ui.sourceInfo.textContent = t("decoding");
  try {
    const decoded = await decodeToModelRate(blob);
    source = { name, ...decoded };
    result = null;
    const seconds = decoded.channels[0].length / decoded.sampleRate;
    ui.sourceInfo.textContent =
      `${name} · ${formatTime(seconds)} · ${decoded.channels.length === 2 ? "stereo" : "mono"}`;
    ui.channels.disabled = decoded.channels.length < 2;
    if (decoded.channels.length < 2) ui.channels.value = "mono";
    player.setTracks(source.channels, decoded.sampleRate, null, 0);
    waveOrig.setAudio(source.channels);
    waveClean.setAudio(null);
    ui.download.hidden = true;
    ui.stats.hidden = true;
    clearTranscript();
    setAB("orig");
    refreshButtons();
    getModel().catch(showError);
  } catch (err) {
    source = null;
    ui.sourceInfo.textContent = "";
    showError(new Error(t("decodeError"), { cause: err }));
    refreshButtons();
  }
}

ui.file.addEventListener("change", () => {
  const f = ui.file.files?.[0];
  if (f) loadSource(f, f.name);
  ui.file.value = "";
});

for (const type of ["dragenter", "dragover"]) {
  ui.drop.addEventListener(type, (e) => {
    e.preventDefault();
    ui.drop.classList.add("over");
  });
}
for (const type of ["dragleave", "drop"]) {
  ui.drop.addEventListener(type, () => ui.drop.classList.remove("over"));
}
ui.drop.addEventListener("drop", (e) => {
  e.preventDefault();
  const f = e.dataTransfer?.files[0];
  if (f) loadSource(f, f.name);
});

let recorder: MediaRecorder | null = null;
ui.record.addEventListener("click", async () => {
  if (recorder) {
    recorder.stop();
    return;
  }
  showError(null);
  try {
    const stream = await navigator.mediaDevices.getUserMedia({
      audio: { echoCancellation: false, noiseSuppression: false, autoGainControl: false, channelCount: 1 },
    });
    const chunks: Blob[] = [];
    const rec = new MediaRecorder(stream);
    recorder = rec;
    const started = performance.now();
    const timer = setInterval(() => {
      ui.recordTime.textContent = formatTime((performance.now() - started) / 1000);
    }, 250);
    rec.ondataavailable = (e) => e.data.size && chunks.push(e.data);
    rec.onstop = () => {
      clearInterval(timer);
      stream.getTracks().forEach((t) => t.stop());
      recorder = null;
      ui.record.textContent = t("recordMic");
      ui.record.classList.remove("recording");
      ui.recordTime.textContent = "";
      const stamp = new Date().toISOString().slice(0, 19).replace(/[:T]/g, "-");
      loadSource(new Blob(chunks, { type: rec.mimeType }), `${t("recordPrefix")}-${stamp}`);
    };
    rec.start();
    ui.record.textContent = t("recordStop");
    ui.record.classList.add("recording");
  } catch (err) {
    showError(new Error(t("micError"), { cause: err }));
  }
});

// --- Settings --------------------------------------------------------------

ui.strength.addEventListener("input", () => {
  ui.strengthOut.textContent = `${ui.strength.value}%`;
  ui.strength.style.setProperty("--fill", `${ui.strength.value}%`);
});
ui.loudness.addEventListener("change", () => {
  const mastering = ui.loudness.value !== "off";
  ui.customLufs.hidden = ui.loudness.value !== "custom";
  ui.ceiling.disabled = !mastering;
  ui.maxGain.disabled = !mastering;
});
ui.accelerator.addEventListener("change", () => {
  if (ui.accelerator.value === "webgpu" && !("gpu" in navigator)) {
    showError(new Error(t("webgpuUnavailable")));
    ui.accelerator.value = "wasm";
  }
  if (clear) getModel().catch(showError);
});

function readOptions(): EnhanceOptions {
  const preset = ui.loudness.value;
  let targetLUFS: EnhanceOptions["targetLUFS"];
  if (preset === "off") targetLUFS = null;
  else if (preset === "custom") targetLUFS = Number(ui.customLufs.value);
  else targetLUFS = preset as LoudnessPreset;
  return {
    strength: Number(ui.strength.value) / 100,
    targetLUFS,
    peakCeilingDBFS: Number(ui.ceiling.value),
    maxGainDB: Number(ui.maxGain.value),
    outputSampleRate: Number(ui.rate.value),
    channelMode: ui.channels.value as "mono" | "preserve",
  };
}

// --- Processing ------------------------------------------------------------

ui.process.addEventListener("click", async () => {
  if (!source || busy) return;
  busy = true;
  refreshButtons();
  showError(null);
  player.pause();
  document.body.classList.add("foaming");
  try {
    const model = await getModel();
    setProgress(null);
    ui.process.textContent = t("processButtonBusy");
    await new Promise((r) => setTimeout(r, 30));
    const input = source.channels.length > 1 ? source.channels : source.channels[0];
    result = await model.enhance(input, source.sampleRate, readOptions());
    showResult(result);
  } catch (err) {
    showError(err);
  } finally {
    busy = false;
    document.body.classList.remove("foaming");
    ui.process.textContent = t("processButton");
    ui.progress.hidden = true;
    refreshButtons();
  }
});

let downloadUrl: string | null = null;

function showResult(r: ClearResult) {
  if (!source) return;
  const originalForPlayback = r.channelCount === 1 && source.channels.length > 1
    ? [downmix(source.channels)]
    : source.channels;
  player.setTracks(originalForPlayback, source.sampleRate, r.channels, r.sampleRate);
  waveClean.setAudio(r.channels);
  setAB("clean");

  if (downloadUrl) URL.revokeObjectURL(downloadUrl);
  downloadUrl = URL.createObjectURL(encodeWav(r.channels, r.sampleRate, ui.format.value as WavFormat));
  ui.download.href = downloadUrl;
  ui.download.download = `${source.name.replace(/\.[^.]+$/, "")}_clear.wav`;
  ui.download.hidden = false;

  ui.statLufs.textContent = r.measuredLUFS === null ? "–" : `${r.measuredLUFS.toFixed(1)} LUFS`;
  ui.statPeak.textContent = r.measuredTruePeakDBFS === null ? "–" : `${r.measuredTruePeakDBFS.toFixed(1)} dBTP`;
  ui.statDur.textContent = formatTime(r.durationSec);
  ui.statRtf.textContent = t("realtimeFactor")(r.realtimeFactor.toFixed(1));
  ui.stats.hidden = false;
  ui.txSource.disabled = false;
  ui.txSource.value = "clean";
}

ui.format.addEventListener("change", () => {
  if (result) showResult(result);
});

function downmix(channels: Float32Array[]): Float32Array {
  const out = new Float32Array(channels[0].length);
  for (const ch of channels) for (let i = 0; i < out.length; i++) out[i] += ch[i] / channels.length;
  return out;
}

// --- A/B player ------------------------------------------------------------

class ABPlayer {
  private ctx: AudioContext | null = null;
  private buffers: { orig: AudioBuffer | null; clean: AudioBuffer | null } = { orig: null, clean: null };
  private raw: { orig: [Float32Array[], number] | null; clean: [Float32Array[], number] | null } = { orig: null, clean: null };
  private node: AudioBufferSourceNode | null = null;
  private startedAt = 0;
  private offset = 0;
  side: "orig" | "clean" = "clean";

  get playing() { return this.node !== null; }

  get duration() {
    const b = this.current();
    return b ? b.length / b.sampleRate : 0;
  }

  get position() {
    if (!this.ctx || !this.node) return this.offset;
    return Math.min(this.duration, this.offset + this.ctx.currentTime - this.startedAt);
  }

  setTracks(orig: Float32Array[], origRate: number, clean: Float32Array[] | null, cleanRate: number) {
    this.stop();
    this.offset = 0;
    this.raw = { orig: [orig, origRate], clean: clean ? [clean, cleanRate] : null };
    this.buffers = { orig: null, clean: null };
  }

  hasClean() { return this.raw.clean !== null; }

  private context() {
    this.ctx ??= new AudioContext({ sampleRate: MODEL_RATE });
    return this.ctx;
  }

  private current(): AudioBuffer | null {
    const side = this.side === "clean" && this.raw.clean ? "clean" : "orig";
    const raw = this.raw[side];
    if (!raw) return null;
    if (!this.buffers[side]) {
      const [channels, rate] = raw;
      const buf = this.context().createBuffer(channels.length, channels[0].length, rate);
      channels.forEach((ch, i) => buf.copyToChannel(ch as Float32Array<ArrayBuffer>, i));
      this.buffers[side] = buf;
    }
    return this.buffers[side];
  }

  play() {
    const buf = this.current();
    if (!buf) return;
    const ctx = this.context();
    void ctx.resume();
    if (this.offset >= this.duration) this.offset = 0;
    const node = ctx.createBufferSource();
    node.buffer = buf;
    node.connect(ctx.destination);
    node.onended = () => {
      if (this.node !== node) return;
      this.offset = this.position;
      this.node = null;
      if (this.offset >= this.duration - 0.01) this.offset = this.duration;
      onPlayerChange();
    };
    node.start(0, this.offset);
    this.node = node;
    this.startedAt = ctx.currentTime;
  }

  pause() {
    if (!this.node) return;
    this.offset = this.position;
    this.stop();
  }

  private stop() {
    const node = this.node;
    this.node = null;
    node?.stop();
  }

  seek(fraction: number) {
    const wasPlaying = this.playing;
    this.pause();
    this.offset = fraction * this.duration;
    if (wasPlaying) this.play();
  }

  switchTo(side: "orig" | "clean") {
    if (side === this.side) return;
    const wasPlaying = this.playing;
    this.pause();
    this.side = side;
    if (wasPlaying) this.play();
  }
}

const player = new ABPlayer();
const accent = (name: string) => () => getComputedStyle(document.documentElement).getPropertyValue(name);
const seek = (f: number) => {
  player.seek(f);
  onPlayerChange();
};
const waveOrig = new Waveform($("wave-orig"), accent("--orig"), seek);
const waveClean = new Waveform($("wave-clean"), accent("--clean"), seek);

function setAB(side: "orig" | "clean") {
  if (side === "clean" && !player.hasClean()) side = "orig";
  player.switchTo(side);
  ui.abOrig.setAttribute("aria-checked", String(side === "orig"));
  ui.abClean.setAttribute("aria-checked", String(side === "clean"));
  ui.abClean.disabled = !player.hasClean();
  onPlayerChange();
}

function onPlayerChange() {
  ui.play.textContent = player.playing ? t("pauseBtn") : t("playBtn");
  const pos = player.position;
  const dur = player.duration;
  ui.clock.textContent = `${formatTime(pos)} / ${formatTime(dur)}`;
  const f = dur ? pos / dur : 0;
  waveOrig.setProgress(f);
  waveClean.setProgress(player.hasClean() ? f : 0);
}

function tick() {
  if (player.playing) {
    onPlayerChange();
    highlightWord(player.position);
  }
  requestAnimationFrame(tick);
}
requestAnimationFrame(tick);

ui.play.addEventListener("click", () => {
  if (player.playing) player.pause();
  else player.play();
  onPlayerChange();
});
ui.abOrig.addEventListener("click", () => setAB("orig"));
ui.abClean.addEventListener("click", () => setAB("clean"));

document.addEventListener("keydown", (e) => {
  const target = e.target as HTMLElement;
  if (target.matches("input, select, textarea") || e.metaKey || e.ctrlKey || e.altKey) return;
  if (e.code === "Space" && source) {
    e.preventDefault();
    ui.play.click();
  } else if (e.key === "a" || e.key === "A") setAB("orig");
  else if (e.key === "b" || e.key === "B") setAB("clean");
});

// --- Transcript and subtitles (Voz) -----------------------------------------

let voz: Promise<VozInstance> | null = null;
let transcript: VozResult | null = null;
let transcribing = false;
let wordSpans: HTMLSpanElement[] = [];
let currentWord = -1;
const exportUrls: string[] = [];

function getVoz(): Promise<VozInstance> {
  voz ??= (async () => {
    ui.txStatus.textContent = t("txLoading");
    const { Voz } = await import("@desert-ant-labs/voz");
    return Voz.load({
      modelBaseUrl: import.meta.env.VITE_VOZ_MODEL_BASE_URL || undefined,
      onProgress: (fraction) => {
        ui.txStatus.textContent = t("txDownloading")(Math.round(fraction * 100));
        setTxProgress(fraction);
      },
    });
  })();
  voz.catch(() => (voz = null));
  return voz;
}

ui.transcribe.addEventListener("click", async () => {
  if (!source || transcribing) return;
  transcribing = true;
  ui.transcribe.textContent = t("txButtonBusy");
  refreshButtons();
  showTxError(null);
  ui.txStatus.hidden = false;
  setTxProgress(null);
  try {
    // Transcribe the version the user picked; both share the same timeline.
    const useClean = ui.txSource.value === "clean" && result;
    const channels = useClean ? result!.channels : source.channels;
    const sampleRate = useClean ? result!.sampleRate : source.sampleRate;
    const model = await getVoz();
    setTxProgress(0);
    const done = await model.transcribe(
      { samples: channels.length > 1 ? downmix(channels) : channels[0], sampleRate },
      {
        onProgress: (fraction) => {
          ui.txStatus.textContent = t("txRunning")(Math.round(fraction * 100));
          setTxProgress(fraction);
        },
      },
    );
    showTranscript(done);
    ui.txStatus.hidden = true;
  } catch (err) {
    ui.txStatus.hidden = true;
    const offline = /download failed|Failed to fetch|NetworkError/i.test(String(err));
    showTxError(offline ? new Error(t("errorHuggingFace"), { cause: err }) : err);
  } finally {
    transcribing = false;
    ui.transcribe.textContent = t("txButton");
    ui.txProgress.hidden = true;
    refreshButtons();
  }
});

function showTranscript(r: VozResult) {
  transcript = r;
  ui.transcript.replaceChildren();
  wordSpans = [];
  currentWord = -1;
  if (!r.words.length) {
    const empty = document.createElement("p");
    empty.className = "empty";
    empty.textContent = t("txEmpty");
    ui.transcript.append(empty);
  }
  // Paragraphs at long pauses, one clickable span per word.
  let paragraph: HTMLParagraphElement | null = null;
  r.words.forEach((word, i) => {
    if (!paragraph || word.start - r.words[i - 1].end >= 1.5) {
      paragraph = document.createElement("p");
      ui.transcript.append(paragraph);
    } else if (!/^[,.;:!?%)\]}»”’…]/.test(word.text.trim())) {
      paragraph.append(" ");
    }
    const span = document.createElement("span");
    span.className = "w";
    span.textContent = word.text.trim();
    span.dataset.i = String(i);
    paragraph.append(span);
    wordSpans.push(span);
  });
  ui.transcript.hidden = false;

  const base = (source?.name ?? "onda").replace(/\.[^.]+$/, "");
  const cues = toCues(r.words);
  setExport(ui.dlSrt, toSrt(cues), `${base}.srt`, "application/x-subrip");
  setExport(ui.dlVtt, toVtt(cues), `${base}.vtt`, "text/vtt");
  setExport(ui.dlTxt, toText(r.words), `${base}.txt`, "text/plain");
  ui.txStats.textContent = t("txDone")(r.words.length, r.realtimeFactor.toFixed(0));
  ui.txExports.hidden = !r.words.length;
}

function setExport(link: HTMLAnchorElement, text: string, name: string, type: string) {
  const url = URL.createObjectURL(new Blob([text], { type: `${type};charset=utf-8` }));
  exportUrls.push(url);
  link.href = url;
  link.download = name;
}

function clearTranscript() {
  transcript = null;
  wordSpans = [];
  currentWord = -1;
  exportUrls.splice(0).forEach((url) => URL.revokeObjectURL(url));
  ui.transcript.replaceChildren();
  ui.transcript.hidden = true;
  ui.txExports.hidden = true;
  showTxError(null);
}

ui.transcript.addEventListener("click", (e) => {
  const span = (e.target as HTMLElement).closest<HTMLSpanElement>(".w");
  if (!span || !transcript || !player.duration) return;
  const word = transcript.words[Number(span.dataset.i)];
  player.seek(Math.min(1, word.start / player.duration));
  if (!player.playing) player.play();
  onPlayerChange();
  highlightWord(word.start);
});

/** Mark the word being spoken at `time`, found by binary search. */
function highlightWord(time: number) {
  if (!transcript || !wordSpans.length) return;
  const words = transcript.words;
  let lo = 0;
  let hi = words.length - 1;
  let found = -1;
  while (lo <= hi) {
    const mid = (lo + hi) >> 1;
    if (words[mid].start <= time) {
      found = mid;
      lo = mid + 1;
    } else hi = mid - 1;
  }
  if (found !== -1 && time > words[found].end + 0.5) found = -1;
  if (found === currentWord) return;
  wordSpans[currentWord]?.classList.remove("now");
  currentWord = found;
  const span = wordSpans[found];
  if (!span) return;
  span.classList.add("now");
  // Keep the spoken word in view inside the transcript box, not the page.
  const box = ui.transcript;
  const top = span.offsetTop - box.offsetTop;
  if (top < box.scrollTop || top > box.scrollTop + box.clientHeight - 40) {
    box.scrollTo({ top: top - box.clientHeight / 3, behavior: "smooth" });
  }
}

function setTxProgress(fraction: number | null) {
  ui.txProgress.hidden = false;
  ui.txProgress.classList.toggle("indeterminate", fraction === null);
  ui.txProgressBar.style.width = fraction === null ? "" : `${Math.round(fraction * 100)}%`;
}

function showTxError(err: unknown) {
  if (err == null) {
    ui.txError.hidden = true;
    return;
  }
  console.error(err);
  const message = err instanceof Error ? err.message : String(err);
  ui.txError.textContent = `${t("txError")} ${message.split("\n")[0]}`;
  ui.txError.hidden = false;
}

// --- Helpers ---------------------------------------------------------------

function refreshButtons() {
  ui.process.disabled = !source || busy;
  ui.transcribe.disabled = !source || transcribing;
  ui.txSource.disabled = !result;
  if (!result) ui.txSource.value = "orig";
  ui.play.disabled = !source;
  ui.record.disabled = busy;
}

function setProgress(fraction: number | null) {
  ui.progress.hidden = false;
  ui.progress.classList.toggle("indeterminate", fraction === null);
  ui.progressBar.style.width = fraction === null ? "" : `${Math.round(fraction * 100)}%`;
}

function showError(err: unknown) {
  if (err == null) {
    ui.error.hidden = true;
    return;
  }
  console.error(err);
  const message = err instanceof Error ? err.message : String(err);
  ui.error.textContent = message.split("\n")[0];
  ui.error.hidden = false;
}

function formatTime(seconds: number) {
  const s = Math.max(0, Math.floor(seconds));
  return `${Math.floor(s / 60)}:${String(s % 60).padStart(2, "0")}`;
}

// Initial language setup & state
applyLanguage(getCurrentLanguage());
refreshButtons();
setAB("orig");
