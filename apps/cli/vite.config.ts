import { defineConfig } from "vite-plus";

export default defineConfig({
  build: {
    ssr: true,
    outDir: "dist",
    emptyOutDir: true,
    minify: false,
    rollupOptions: {
      input: "src/main.ts",
      output: {
        format: "esm",
        entryFileNames: "cli.mjs",
        banner: "#!/usr/bin/env node",
      },
    },
  },
  lint: {
    options: {
      typeAware: true,
      typeCheck: true,
    },
  },
  fmt: {},
});
