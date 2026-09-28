// Serve the LiteRT.js runtime from our own origin instead of the jsDelivr CDN
// the SDK defaults to, so the app works offline once the model is cached.
import { cpSync, mkdirSync } from "node:fs";
import { createRequire } from "node:module";
import path from "node:path";

const require = createRequire(import.meta.url);
const pkgDir = path.dirname(require.resolve("@litertjs/core/package.json"));
const dest = path.resolve(import.meta.dirname, "../public/litert");

mkdirSync(dest, { recursive: true });
cpSync(path.join(pkgDir, "wasm"), dest, { recursive: true });
console.log(`LiteRT.js runtime copied to ${path.relative(process.cwd(), dest)}`);
