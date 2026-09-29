// End-to-end check against the real models: serve the production build, load
// a noisy file, clean it with Clear and confirm the result and export appear;
// then, given a spoken file, clean it and transcribe it with Voz.
// Usage: node scripts/e2e.mjs <noisy.wav> [speech.wav]   (after `npm run build`)
import { spawn } from "node:child_process";
import { chromium } from "playwright";

const [input, speech] = process.argv.slice(2);
if (!input) throw new Error("usage: node scripts/e2e.mjs <noisy.wav> [speech.wav]");

const server = spawn("npx", ["vite", "preview", "--port", "4173", "--strictPort"], { stdio: "inherit" });
const browser = await chromium.launch();
try {
  const page = await browser.newPage();
  page.on("console", (m) => m.type() === "error" && console.log("console:", m.text()));
  for (let i = 0; ; i++) {
    try {
      await page.goto("http://localhost:4173/");
      break;
    } catch (e) {
      if (i > 30) throw e;
      await new Promise((r) => setTimeout(r, 500));
    }
  }
  await page.setInputFiles("#file", input);
  await page.waitForFunction(
    () => ["ready", "error"].includes(document.getElementById("model-status").dataset.state),
    null,
    { timeout: 180_000 },
  );
  // The state, not the text: the UI follows the browser language (EN or IT).
  const state = await page.getAttribute("#model-status", "data-state");
  if (state !== "ready") {
    const status = await page.textContent("#model-status-text");
    throw new Error(`model did not load: ${status} / ${await page.textContent("#error")}`);
  }

  await page.click("#process");
  await page.waitForSelector("#download:not([hidden])", { timeout: 180_000 });
  const lufs = await page.textContent("#stat-lufs");
  const speed = await page.textContent("#stat-rtf");
  console.log(`cleaned: input ${lufs}, ${speed}`);
  if (!/LUFS/.test(lufs)) throw new Error("no loudness measurement in the result");

  if (speech) {
    await page.setInputFiles("#file", speech);
    await page.waitForFunction(() => !document.getElementById("process").disabled, null, { timeout: 60_000 });
    await page.click("#process");
    await page.waitForSelector("#download:not([hidden])", { timeout: 180_000 });
    if ((await page.inputValue("#tx-source")) !== "clean") throw new Error("the clean version is not offered for transcription");
    await page.click("#transcribe");
    // Voz downloads ~390 MB the first time and runs on the CPU without a GPU.
    await page.waitForFunction(
      () => !document.getElementById("transcript").hidden || !document.getElementById("tx-error").hidden,
      null,
      { timeout: 600_000 },
    );
    const error = await page.textContent("#tx-error");
    if (await page.isVisible("#tx-error")) throw new Error(`transcription failed: ${error}`);
    const text = (await page.textContent("#transcript")).trim();
    const words = await page.locator("#transcript .w").count();
    console.log(`transcribed ${words} words: ${text}`);
    if (!/hello|world|test|transcri/i.test(text)) throw new Error("the transcript does not match the speech");
    const srt = await page.evaluate(async () => (await fetch(document.getElementById("dl-srt").href)).text());
    if (!/^1\n\d\d:\d\d:\d\d,\d{3} --> /.test(srt)) throw new Error(`bad SRT: ${srt.slice(0, 80)}`);
    console.log(`SRT:\n${srt.split("\n\n")[0]}`);
  }
} finally {
  await browser.close();
  server.kill();
}
