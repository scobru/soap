// node --test --experimental-strip-types scripts/subtitles.test.mjs
import assert from "node:assert/strict";
import { test } from "node:test";
import { joinWords, splitLines, toCues, toSrt, toText, toVtt } from "../src/subtitles.ts";

const w = (text, start, end) => ({ text, start, end });

test("punctuation tokens stick to the word before", () => {
  assert.equal(joinWords(["Ciao", ",", "come", "stai", "?"]), "Ciao, come stai?");
  assert.equal(joinWords([" hello ", "(", "world", ")"]), "hello (world)");
});

test("a pause, a sentence end or the length starts a new cue", () => {
  const words = [
    w("Hello", 0, 0.4), w("there.", 0.5, 0.9),
    w("This", 1.0, 1.2), w("is", 1.3, 1.4), w("a", 1.5, 1.6), w("longer", 1.7, 2.0), w("sentence", 2.1, 2.5), w("now.", 2.6, 3.0),
    w("After", 4.5, 4.8), w("a", 4.9, 5.0), w("pause", 5.1, 5.5),
  ];
  const cues = toCues(words);
  assert.deepEqual(cues.map((c) => c.lines.join(" ")), ["Hello there. This is a longer sentence now.", "After a pause"]);
  assert.equal(cues[1].start, 4.5);
  assert.ok(cues.every((c) => c.end > c.start));
  assert.ok(cues[0].end <= cues[1].start);
});

test("cues are split before they get too long to read", () => {
  const words = Array.from({ length: 40 }, (_, i) => w(`word${i}`, i * 0.3, i * 0.3 + 0.25));
  const cues = toCues(words);
  for (const c of cues) {
    assert.ok(c.lines.length <= 2);
    assert.ok(c.lines.every((l) => l.length <= 42 + 8), c.lines.join("|"));
    assert.ok(c.end - c.start <= 6.5);
  }
  assert.equal(cues.flatMap((c) => c.lines.join(" ").split(" ")).length, 40);
});

test("long text splits into two lines near the middle", () => {
  assert.deepEqual(splitLines("short", 42), ["short"]);
  const [a, b] = splitLines("the quick brown fox jumps over the lazy dog and runs far away", 42);
  assert.ok(Math.abs(a.length - b.length) < 12);
});

test("SRT and VTT timestamps", () => {
  const cues = [{ start: 3661.5, end: 3662.25, lines: ["Uno", "due"] }];
  assert.equal(toSrt(cues), "1\n01:01:01,500 --> 01:01:02,250\nUno\ndue\n");
  assert.equal(toVtt(cues), "WEBVTT\n\n01:01:01.500 --> 01:01:02.250\nUno\ndue\n");
});

test("plain text breaks paragraphs at long pauses", () => {
  const text = toText([w("One", 0, 0.3), w("two.", 0.4, 0.7), w("Three", 3, 3.3)]);
  assert.equal(text, "One two.\n\nThree\n");
});
