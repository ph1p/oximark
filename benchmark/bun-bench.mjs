/**
 * Bun benchmark — all WASM parsers + Bun built-in, all under the Bun runtime.
 * Run with: bun benchmark/bun-bench.mjs
 *
 * Parsers:
 *   ironmark      — ironmark WASM
 *   markdown-wasm — markdown-wasm WASM
 *   md4w          — md4w WASM
 *   bun           — Bun.markdown.html (native, not WASM)
 *
 * Fair comparison: all WASM parsers run on the same Bun runtime so JIT
 * conditions are identical. Bun built-in is labeled separately as native.
 */

import { createRequire } from "node:module";
import { join } from "node:path";

const jsonMode = process.argv[2] === "--json";
const log = jsonMode ? (...args) => process.stderr.write(args.join(" ") + "\n") : console.log;
import {
  ROOT,
  fmtNs,
  fmtBytes,
  loadSpecMarkdown,
  loadAllFeatures,
  genInlineHeavy,
  genHeadings,
  genNestedList,
  genTable,
  genCodeBlocks,
  genMixedDoc,
  genPathologicalBackticks,
  genPathologicalEmphasis,
  genManyRefLinks,
  writeHistoryJson,
} from "./generators.mjs";

// ─── Load parsers ─────────────────────────────────────────────────────────────

const require = createRequire(import.meta.url);

log("Loading parsers...");
const { parse: ironmarkParse } = await import("../wasm/node.js");
const markdownWasm = require("markdown-wasm/dist/markdown.node.js");
const { init: md4wInit, mdToHtml } = await import("md4w");
await md4wInit();

const parsers = {
  ironmark: (input) => ironmarkParse(input),
  "markdown-wasm": (input) => markdownWasm.parse(input),
  md4w: (input) => mdToHtml(input),
  bun: (input) => Bun.markdown.html(input),
};

// ─── Benchmark harness ────────────────────────────────────────────────────────

function bench(name, input, { iterations = 500, warmup = 100, batch = 10 } = {}) {
  const bytes = Buffer.byteLength(input, "utf8");
  const results = {};

  for (const [lib, fn] of Object.entries(parsers)) {
    for (let i = 0; i < warmup; i++) fn(input);

    const samples = [];
    for (let i = 0; i < iterations; i++) {
      const start = performance.now();
      for (let b = 0; b < batch; b++) fn(input);
      samples.push((performance.now() - start) / batch);
    }
    samples.sort((a, b) => a - b);

    const mid = Math.floor(samples.length / 2);
    const median_ms =
      samples.length % 2 === 0 ? (samples[mid - 1] + samples[mid]) / 2 : samples[mid];
    const p95_ms = samples[Math.floor(samples.length * 0.95)];

    results[lib] = {
      median_ns: median_ms * 1e6,
      p95_ns: p95_ms * 1e6,
    };
  }
  return { name, bytes, results };
}

// ─── Run ──────────────────────────────────────────────────────────────────────

log("Running Bun benchmarks...\n");

const specMarkdown = loadSpecMarkdown();
const allFeaturesInput = loadAllFeatures();

const sections = [
  {
    title: "CommonMark Spec",
    benches: [bench("spec (all examples)", specMarkdown, { iterations: 300, warmup: 50 })],
  },
  {
    title: "All Features",
    benches: [bench("all features", allFeaturesInput)],
  },
  {
    title: "Document Sizes",
    benches: [1_000, 10_000, 100_000].map((size) => {
      const input = genMixedDoc(Math.ceil(size / 80)).slice(0, size);
      return bench(`mixed ${fmtBytes(size)}`, input);
    }),
  },
  {
    title: "Block Types",
    benches: [
      bench("headings", genHeadings()),
      bench("nested lists", genNestedList()),
      bench("table (100×10)", genTable()),
      bench("code blocks", genCodeBlocks()),
    ],
  },
  {
    title: "Inline-heavy",
    benches: [bench("inline heavy", genInlineHeavy())],
  },
  {
    title: "Pathological",
    benches: [
      bench("backticks ×500", genPathologicalBackticks(), { iterations: 200, warmup: 50 }),
      bench("emphasis ×10k", genPathologicalEmphasis(), { iterations: 100, warmup: 20 }),
      bench("table 1k rows", genTable(1_000, 10), { iterations: 200, warmup: 50 }),
      bench("ref links ×1k", genManyRefLinks(), { iterations: 200, warmup: 50 }),
    ],
  },
];

// ─── Print results ────────────────────────────────────────────────────────────

for (const section of sections) {
  log(`── ${section.title} ${"─".repeat(Math.max(0, 50 - section.title.length))}`);
  for (const b of section.benches) {
    const entries = Object.entries(b.results).sort(([, a], [, bv]) => a.median_ns - bv.median_ns);
    const fastest = entries[0][1].median_ns;
    const header = `${b.name} (${fmtBytes(b.bytes)})`;
    log(`  ${header}`);
    for (const [lib, r] of entries) {
      const ratio = r.median_ns / fastest;
      const ratioStr = ratio < 1.01 ? "  baseline" : `  ${ratio.toFixed(2)}x slower`;
      log(
        `    ${lib.padEnd(12)} median ${fmtNs(r.median_ns).padStart(10)}   p95 ${fmtNs(r.p95_ns).padStart(10)}${ratioStr}`,
      );
    }
    log();
  }
}

// ─── Output ───────────────────────────────────────────────────────────────────

// When run standalone (not imported), write history JSON directly.
// When invoked via report.mjs, it reads the JSON from stdout.
if (jsonMode) {
  process.stdout.write(JSON.stringify(sections));
} else {
  const { filename } = writeHistoryJson(join(ROOT, "benchmark", "history"), { bun: sections });
  log(`\nResults written to benchmark/history/${filename}`);
}
