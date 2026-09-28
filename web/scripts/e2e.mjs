// End-to-end check against the real Clear model: serve the production build,
// load a noisy file, clean it, and confirm the result and export appear.
// Usage: node scripts/e2e.mjs <noisy.wav>   (after `npm run build`)
import { spawn } from "node:child_process";
import { chromium } from "playwright";

const input = process.argv[2];
if (!input) throw new Error("usage: node scripts/e2e.mjs <noisy.wav>");

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
  const status = await page.textContent("#model-status-text");
  if (!/pront/i.test(status)) throw new Error(`model did not load: ${status} / ${await page.textContent("#error")}`);

  await page.click("#process");
  await page.waitForSelector("#download:not([hidden])", { timeout: 180_000 });
  const lufs = await page.textContent("#stat-lufs");
  const speed = await page.textContent("#stat-rtf");
  console.log(`cleaned: input ${lufs}, ${speed}`);
  if (!/LUFS/.test(lufs)) throw new Error("no loudness measurement in the result");
} finally {
  await browser.close();
  server.kill();
}
