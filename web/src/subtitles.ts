// Subtitles from word timestamps: group words into readable cues, then write
// them as SRT, WebVTT or plain text.

export type Word = { text: string; start: number; end: number };
export type Cue = { start: number; end: number; lines: string[] };

export type CueOptions = {
  /** Characters per line; two lines per cue at most. */
  lineChars: number;
  /** Longest a cue stays on screen, in seconds. */
  maxDuration: number;
  /** A pause at least this long (seconds) always starts a new cue. */
  pause: number;
  /** Shortest time a cue stays on screen, in seconds. */
  minDuration: number;
};

export const DEFAULT_CUE_OPTIONS: CueOptions = { lineChars: 42, maxDuration: 6, pause: 0.7, minDuration: 0.8 };

/** Join words into text, keeping punctuation tokens attached to the word before. */
export function joinWords(words: readonly string[]): string {
  let out = "";
  for (const raw of words) {
    const w = raw.trim();
    if (!w) continue;
    out += out && !/^[,.;:!?%)\]}»”’…]/.test(w) && !/[(\[{«“‘]$/.test(out) ? ` ${w}` : w;
  }
  return out;
}

export function toCues(words: readonly Word[], options: Partial<CueOptions> = {}): Cue[] {
  const o = { ...DEFAULT_CUE_OPTIONS, ...options };
  const groups: Word[][] = [];
  let current: Word[] = [];
  const text = (ws: Word[]) => joinWords(ws.map((w) => w.text));

  for (const word of words) {
    if (!word.text.trim()) continue;
    const last = current[current.length - 1];
    if (last) {
      const next = [...current, word];
      const tooLong = text(next).length > o.lineChars * 2;
      const tooSlow = word.end - current[0].start > o.maxDuration;
      const paused = word.start - last.end >= o.pause;
      // End a cue at a sentence once it is long enough to read on its own.
      const sentence = /[.!?…]$/.test(last.text.trim()) && text(current).length >= o.lineChars / 2;
      if (tooLong || tooSlow || paused || sentence) {
        groups.push(current);
        current = [];
      }
    }
    current.push(word);
  }
  if (current.length) groups.push(current);

  return groups.map((ws, i) => {
    const start = ws[0].start;
    const nextStart = groups[i + 1]?.[0].start ?? Infinity;
    const end = Math.min(Math.max(ws[ws.length - 1].end, start + o.minDuration), nextStart);
    return { start, end: Math.max(end, start), lines: splitLines(text(ws), o.lineChars) };
  });
}

/** One line, or two split at the space nearest the middle. */
export function splitLines(text: string, lineChars: number): string[] {
  if (text.length <= lineChars) return [text];
  const middle = text.length / 2;
  let best = -1;
  for (let i = text.indexOf(" "); i !== -1; i = text.indexOf(" ", i + 1)) {
    if (best === -1 || Math.abs(i - middle) < Math.abs(best - middle)) best = i;
  }
  return best === -1 ? [text] : [text.slice(0, best), text.slice(best + 1)];
}

function timestamp(seconds: number, separator: "," | "."): string {
  const ms = Math.max(0, Math.round(seconds * 1000));
  const pad = (n: number, width = 2) => String(n).padStart(width, "0");
  const h = Math.floor(ms / 3_600_000);
  const m = Math.floor(ms / 60_000) % 60;
  const s = Math.floor(ms / 1000) % 60;
  return `${pad(h)}:${pad(m)}:${pad(s)}${separator}${pad(ms % 1000, 3)}`;
}

export function toSrt(cues: readonly Cue[]): string {
  return cues
    .map((c, i) => `${i + 1}\n${timestamp(c.start, ",")} --> ${timestamp(c.end, ",")}\n${c.lines.join("\n")}\n`)
    .join("\n");
}

export function toVtt(cues: readonly Cue[]): string {
  const body = cues.map((c) => `${timestamp(c.start, ".")} --> ${timestamp(c.end, ".")}\n${c.lines.join("\n")}\n`);
  return ["WEBVTT\n", ...body].join("\n");
}

/** The transcript as paragraphs, a new one after a long pause. */
export function toText(words: readonly Word[], paragraphPause = 1.5): string {
  const paragraphs: string[][] = [[]];
  words.forEach((w, i) => {
    if (i > 0 && w.start - words[i - 1].end >= paragraphPause) paragraphs.push([]);
    paragraphs[paragraphs.length - 1].push(w.text);
  });
  return paragraphs.map((p) => joinWords(p)).filter(Boolean).join("\n\n") + "\n";
}
