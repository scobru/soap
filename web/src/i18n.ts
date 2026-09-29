export type Language = "it" | "en";

export interface Translations {
  pageTitle: string;
  pageDescription: string;
  brandSubtitle: string;
  modelStatusIdle: string;
  modelStatusLoadingRuntime: string;
  modelStatusDownloadingModel: string;
  modelStatusDownloadingPct: (pct: number) => string;
  modelStatusReady: (acc: string) => string;
  modelStatusError: string;
  errorHuggingFace: string;
  sectionSource: string;
  dropTitle: string;
  dropSubtitle: string;
  recordMic: string;
  recordStop: string;
  recordPrefix: string;
  micError: string;
  decoding: string;
  decodeError: string;
  sectionSettings: string;
  labelStrength: string;
  helpStrength: string;
  labelLoudness: string;
  loudnessApple: string;
  loudnessSpotify: string;
  loudnessBroadcast: string;
  loudnessCustom: string;
  loudnessOff: string;
  labelCeiling: string;
  labelMaxGain: string;
  labelChannels: string;
  channelMono: string;
  channelPreserve: string;
  helpChannels: string;
  labelRate: string;
  labelAccelerator: string;
  accWasm: string;
  accWebgpu: string;
  webgpuUnavailable: string;
  labelFormat: string;
  processButton: string;
  processButtonBusy: string;
  sectionCompare: string;
  tagOrig: string;
  tagClean: string;
  playBtn: string;
  pauseBtn: string;
  abOrigBtn: string;
  abCleanBtn: string;
  downloadWav: string;
  statInputLufs: string;
  statOutputPeak: string;
  statDuration: string;
  statSpeed: string;
  realtimeFactor: (rtf: string) => string;
  keyboardHint: string;
  footDev: string;
  footRepo: string;
  footPlugin: string;
  footCredits: string;
  sectionTranscript: string;
  txSourceLabel: string;
  txSourceClean: string;
  txSourceOrig: string;
  txButton: string;
  txButtonBusy: string;
  txHelp: string;
  txLoading: string;
  txDownloading: (pct: number) => string;
  txRunning: (pct: number) => string;
  txDone: (words: number, rtf: string) => string;
  txEmpty: string;
  txError: string;
  dlSrt: string;
  dlVtt: string;
  dlTxt: string;
}

export const dict: Record<Language, Translations> = {
  it: {
    pageTitle: "Onda: voce, testo e sottotitoli",
    pageDescription: "Onda: pulisci, trascrivi e sottotitola la voce nel browser con i modelli Clear e Voz, tutto in locale.",
    brandSubtitle: "Pulisci, trascrivi e sottotitola la voce. L'audio non lascia mai il tuo dispositivo.",
    modelStatusIdle: "Modello non caricato",
    modelStatusLoadingRuntime: "Caricamento runtime…",
    modelStatusDownloadingModel: "Download modello…",
    modelStatusDownloadingPct: (pct: number) => `Download modello ${pct}%`,
    modelStatusReady: (acc: string) => `Modello pronto · ${acc}`,
    modelStatusError: "Errore nel caricamento del modello",
    errorHuggingFace: "Impossibile scaricare il modello da Hugging Face. Controlla la connessione e riprova.",
    sectionSource: "1 · Sorgente",
    dropTitle: "Trascina qui un file audio",
    dropSubtitle: "oppure clicca per sceglierlo (WAV, MP3, M4A, OGG, FLAC, video…)",
    recordMic: "● Registra dal microfono",
    recordStop: "■ Stop",
    recordPrefix: "registrazione",
    micError: "Accesso al microfono negato o non disponibile.",
    decoding: "Decodifica in corso…",
    decodeError: "Impossibile decodificare questo file. Prova con WAV, MP3 o M4A.",
    sectionSettings: "2 · Pulisci con Soap",
    labelStrength: "Intensità",
    helpStrength: "Miscela tra originale e voce pulita. Abbassala se il risultato suona troppo processato.",
    labelLoudness: "Loudness di destinazione",
    loudnessApple: "Apple Podcasts · −19 LUFS",
    loudnessSpotify: "Spotify / YouTube · −14 LUFS",
    loudnessBroadcast: "Broadcast EBU R128 · −23 LUFS",
    loudnessCustom: "Personalizzata…",
    loudnessOff: "Nessuna (livello del modello)",
    labelCeiling: "True peak max (dBTP)",
    labelMaxGain: "Guadagno max (dB)",
    labelChannels: "Canali",
    channelMono: "Mono",
    channelPreserve: "Mantieni stereo",
    helpChannels: "Lo stereo richiede un passaggio per canale (~1,8×).",
    labelRate: "Sample rate uscita",
    labelAccelerator: "Accelerazione",
    accWasm: "CPU (WebAssembly)",
    accWebgpu: "GPU (WebGPU)",
    webgpuUnavailable: "WebGPU non è disponibile in questo browser: uso la CPU.",
    labelFormat: "Formato WAV",
    processButton: "Pulisci la voce",
    processButtonBusy: "Insapono…",
    sectionCompare: "3 · Confronto",
    tagOrig: "Originale",
    tagClean: "Pulito",
    playBtn: "▶ Play",
    pauseBtn: "❚❚ Pausa",
    abOrigBtn: "A · Originale",
    abCleanBtn: "B · Pulito",
    downloadWav: "Scarica WAV",
    statInputLufs: "Loudness in ingresso",
    statOutputPeak: "True peak in uscita",
    statDuration: "Durata",
    statSpeed: "Velocità",
    realtimeFactor: (rtf: string) => `${rtf}× tempo reale`,
    keyboardHint: "<kbd>Spazio</kbd> play/pausa, <kbd>A</kbd>/<kbd>B</kbd> per passare tra originale e pulito senza perdere la posizione.",
    footDev: "<strong>Onda</strong> · Sviluppato da <a href=\"https://scobrudot.dev\" target=\"_blank\" rel=\"noopener\"><strong>scobru</strong> (Francesco Bruno)</a>",
    footRepo: "Repository GitHub",
    footPlugin: "Plugin VST3 / CLAP / AU",
    footCredits: "Miglioramento della voce con <a href=\"https://desertant.com/models/clear/\" target=\"_blank\" rel=\"noopener\">Clear</a> e riconoscimento vocale con <a href=\"https://desertant.com/models/voz/\" target=\"_blank\" rel=\"noopener\">Voz</a> di Desert Ant Labs, sotto la <a href=\"https://license.desertant.com/1.0\" target=\"_blank\" rel=\"noopener\">Desert Ant Labs Source-Available License</a>. I modelli vengono scaricati da Hugging Face al primo uso e poi restano in cache locale; l'audio non lascia mai il tuo dispositivo.",
    sectionTranscript: "4 · Trascrizione e sottotitoli",
    txSourceLabel: "Audio da trascrivere",
    txSourceClean: "Versione pulita",
    txSourceOrig: "Originale",
    txButton: "Trascrivi",
    txButtonBusy: "Trascrivo…",
    txHelp: "Voz riconosce 25 lingue, tra cui italiano e inglese. Al primo uso scarica il modello (circa 390 MB), poi resta in cache.",
    txLoading: "Carico Voz…",
    txDownloading: (pct: number) => `Scarico Voz… ${pct}%`,
    txRunning: (pct: number) => `Trascrivo… ${pct}%`,
    txDone: (words: number, rtf: string) => `${words} parole · ${rtf}× tempo reale`,
    txEmpty: "Nessuna parola riconosciuta.",
    txError: "La trascrizione non è riuscita.",
    dlSrt: "Sottotitoli SRT",
    dlVtt: "WebVTT",
    dlTxt: "Testo",
  },
  en: {
    pageTitle: "Onda: voice, text and subtitles",
    pageDescription: "Onda: clean up, transcribe and subtitle voice recordings in the browser with the Clear and Voz models, fully on-device.",
    brandSubtitle: "Clean up, transcribe and subtitle your voice. Audio never leaves your device.",
    modelStatusIdle: "Model not loaded",
    modelStatusLoadingRuntime: "Loading runtime…",
    modelStatusDownloadingModel: "Downloading model…",
    modelStatusDownloadingPct: (pct: number) => `Downloading model ${pct}%`,
    modelStatusReady: (acc: string) => `Model ready · ${acc}`,
    modelStatusError: "Failed to load model",
    errorHuggingFace: "Could not download model from Hugging Face. Check your internet connection and retry.",
    sectionSource: "1 · Source",
    dropTitle: "Drop an audio file here",
    dropSubtitle: "or click to select (WAV, MP3, M4A, OGG, FLAC, video…)",
    recordMic: "● Record from microphone",
    recordStop: "■ Stop",
    recordPrefix: "recording",
    micError: "Microphone access denied or unavailable.",
    decoding: "Decoding audio…",
    decodeError: "Could not decode this file. Try WAV, MP3, or M4A.",
    sectionSettings: "2 · Clean up with Soap",
    labelStrength: "Intensity",
    helpStrength: "Blend between original and clean voice. Lower it if the output sounds over-processed.",
    labelLoudness: "Target loudness",
    loudnessApple: "Apple Podcasts · −19 LUFS",
    loudnessSpotify: "Spotify / YouTube · −14 LUFS",
    loudnessBroadcast: "Broadcast EBU R128 · −23 LUFS",
    loudnessCustom: "Custom…",
    loudnessOff: "None (model level)",
    labelCeiling: "Max true peak (dBTP)",
    labelMaxGain: "Max gain (dB)",
    labelChannels: "Channels",
    channelMono: "Mono",
    channelPreserve: "Preserve stereo",
    helpChannels: "Stereo requires one pass per channel (~1.8×).",
    labelRate: "Output sample rate",
    labelAccelerator: "Acceleration",
    accWasm: "CPU (WebAssembly)",
    accWebgpu: "GPU (WebGPU)",
    webgpuUnavailable: "WebGPU is not available in this browser: falling back to CPU.",
    labelFormat: "WAV format",
    processButton: "Clean voice",
    processButtonBusy: "Cleaning…",
    sectionCompare: "3 · Comparison",
    tagOrig: "Original",
    tagClean: "Clean",
    playBtn: "▶ Play",
    pauseBtn: "❚❚ Pause",
    abOrigBtn: "A · Original",
    abCleanBtn: "B · Clean",
    downloadWav: "Download WAV",
    statInputLufs: "Input loudness",
    statOutputPeak: "Output true peak",
    statDuration: "Duration",
    statSpeed: "Speed",
    realtimeFactor: (rtf: string) => `${rtf}× realtime`,
    keyboardHint: "<kbd>Space</kbd> play/pause, <kbd>A</kbd>/<kbd>B</kbd> to toggle between original and clean without losing position.",
    footDev: "<strong>Onda</strong> · Developed by <a href=\"https://scobrudot.dev\" target=\"_blank\" rel=\"noopener\"><strong>scobru</strong> (Francesco Bruno)</a>",
    footRepo: "GitHub Repository",
    footPlugin: "VST3 / CLAP / AU Plugin",
    footCredits: "Voice enhancement by <a href=\"https://desertant.com/models/clear/\" target=\"_blank\" rel=\"noopener\">Clear</a> and speech recognition by <a href=\"https://desertant.com/models/voz/\" target=\"_blank\" rel=\"noopener\">Voz</a> from Desert Ant Labs, under the <a href=\"https://license.desertant.com/1.0\" target=\"_blank\" rel=\"noopener\">Desert Ant Labs Source-Available License</a>. Models are downloaded from Hugging Face on first use and cached locally; audio never leaves your device.",
    sectionTranscript: "4 · Transcript and subtitles",
    txSourceLabel: "Audio to transcribe",
    txSourceClean: "Clean version",
    txSourceOrig: "Original",
    txButton: "Transcribe",
    txButtonBusy: "Transcribing…",
    txHelp: "Voz recognizes 25 languages, English and Italian among them. The first time it downloads the model (about 390 MB), then keeps it cached.",
    txLoading: "Loading Voz…",
    txDownloading: (pct: number) => `Downloading Voz… ${pct}%`,
    txRunning: (pct: number) => `Transcribing… ${pct}%`,
    txDone: (words: number, rtf: string) => `${words} words · ${rtf}× realtime`,
    txEmpty: "No words recognized.",
    txError: "Transcription failed.",
    dlSrt: "SRT subtitles",
    dlVtt: "WebVTT",
    dlTxt: "Text",
  },
};

const STORAGE_KEY = "soap-lang";

export function getInitialLanguage(): Language {
  try {
    const saved = localStorage.getItem(STORAGE_KEY);
    if (saved === "it" || saved === "en") return saved;
  } catch {}
  const browserLang = (navigator.language || "").toLowerCase();
  return browserLang.startsWith("it") ? "it" : "en";
}

let currentLanguage: Language = getInitialLanguage();
const listeners: Array<(lang: Language) => void> = [];

export function getCurrentLanguage(): Language {
  return currentLanguage;
}

export function t<K extends keyof Translations>(key: K): Translations[K] {
  return dict[currentLanguage][key];
}

export function onLanguageChange(fn: (lang: Language) => void) {
  listeners.push(fn);
}

export function setLanguage(lang: Language) {
  currentLanguage = lang;
  try {
    localStorage.setItem(STORAGE_KEY, lang);
  } catch {}
  applyLanguage(lang);
  for (const fn of listeners) {
    fn(lang);
  }
}

export function applyLanguage(lang: Language) {
  document.documentElement.setAttribute("lang", lang);
  document.title = dict[lang].pageTitle;
  const metaDesc = document.querySelector('meta[name="description"]');
  if (metaDesc) metaDesc.setAttribute("content", dict[lang].pageDescription);

  // Update text nodes marked with data-i18n
  document.querySelectorAll<HTMLElement>("[data-i18n]").forEach((el) => {
    const key = el.dataset.i18n as keyof Translations;
    const val = dict[lang][key];
    if (typeof val === "string") {
      el.textContent = val;
    }
  });

  // Update HTML nodes marked with data-i18n-html
  document.querySelectorAll<HTMLElement>("[data-i18n-html]").forEach((el) => {
    const key = el.dataset.i18nHtml as keyof Translations;
    const val = dict[lang][key];
    if (typeof val === "string") {
      el.innerHTML = val;
    }
  });

  // Update select option texts
  document.querySelectorAll<HTMLOptionElement>("option[data-i18n]").forEach((opt) => {
    const key = opt.dataset.i18n as keyof Translations;
    const val = dict[lang][key];
    if (typeof val === "string") {
      opt.textContent = val;
    }
  });

  // Update language switcher buttons
  document.querySelectorAll<HTMLButtonElement>(".lang-btn").forEach((btn) => {
    const isCurrent = btn.dataset.lang === lang;
    btn.setAttribute("aria-pressed", String(isCurrent));
  });
}
