import { defineConfig } from "vite";

// Cross-origin isolation lets LiteRT.js pick its multi-threaded wasm build.
// `credentialless` keeps the Hugging Face model download working without CORP headers.
const isolation = {
  "Cross-Origin-Opener-Policy": "same-origin",
  "Cross-Origin-Embedder-Policy": "credentialless",
};

export default defineConfig({
  base: "./",
  build: { target: "es2022" },
  // Pre-bundling would break the SDK's `new URL("ClearWeb.wasm", import.meta.url)`.
  optimizeDeps: {
    exclude: ["@desert-ant-labs/clear", "@desert-ant-labs/core", "@litertjs/core"],
  },
  server: { headers: isolation },
  preview: { headers: isolation },
});
