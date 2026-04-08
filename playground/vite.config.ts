import { defineConfig } from "vite";
import tailwindcss from "@tailwindcss/vite";
import react from "@vitejs/plugin-react";
import { resolve } from "node:path";

function benchmarkDataPlugin() {
  return {
    name: "benchmark-data",
    async buildStart() {
      const { writePlaygroundData } = await import(
        resolve(import.meta.dirname, "../benchmark/generators.mjs")
      );
      writePlaygroundData();
    },
  };
}

export default defineConfig({
  base: process.env.BASE_URL || "/",
  plugins: [
    benchmarkDataPlugin(),
    react({
      babel: {
        plugins: ["babel-plugin-react-compiler"],
      },
    }),
    tailwindcss(),
  ],
  build: {
    target: "esnext",
  },
});
