/** Vite multi-page setup for main/chat/bubble/menu desktop webviews. */

import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";
import { resolve } from "node:path";

export default defineConfig({
  plugins: [react()],
  server: {
    host: "127.0.0.1",
    port: 1420,
    strictPort: true
  },
  build: {
    rollupOptions: {
      input: {
        index: resolve(__dirname, "index.html"),
        chat: resolve(__dirname, "chat.html"),
        bubble: resolve(__dirname, "bubble.html"),
        menu: resolve(__dirname, "menu.html")
      }
    }
  }
});
