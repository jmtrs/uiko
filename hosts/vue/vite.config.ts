import { defineConfig } from "vite";

export default defineConfig({
  server: {
    proxy: {
      "/__uiko": {
        target: "http://127.0.0.1:3001",
        changeOrigin: false,
      },
    },
  },
});
