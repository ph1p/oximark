import { readFileSync, writeFileSync, existsSync, readdirSync } from "node:fs";
import { createRequire } from "node:module";
import { join, resolve } from "node:path";
import { execSync } from "node:child_process";

const require = createRequire(import.meta.url);
const ROOT = resolve(import.meta.dirname, "..");
const CRITERION_DIR = join(ROOT, "target", "criterion");

// ─── Build WASM ─────────────────────────────────────────────────────

console.log("Building WASM...\n");
try {
  execSync("pnpm build", {
    cwd: ROOT,
    stdio: "inherit",
  });
} catch {
  console.error("\nWASM build failed — skipping WASM results.\n");
  process.exit(1);
}

// ─── Input generators ───────────────────────────────────────────────

function genInlineHeavy(n = 200) {
  return Array.from(
    { length: n },
    (_, i) =>
      `This has **bold**, *italic*, \`code\`, ~~strike~~, [link](http://x.com/${i}), and more.\n\n`,
  ).join("");
}

function genHeadings(n = 200) {
  return Array.from(
    { length: n },
    (_, i) => `# Heading ${i}\n\nSome paragraph text under heading ${i}.\n`,
  ).join("");
}

function genNestedList(depth = 50) {
  return Array.from({ length: depth }, (_, i) => `${"  ".repeat(i)}- item ${i}\n`).join("");
}

function genTable(rows = 100, cols = 10) {
  const header = `|${Array.from({ length: cols }, (_, c) => ` col${c} `).join("|")}|`;
  const sep = `|${Array.from({ length: cols }, () => " --- ").join("|")}|`;
  const body = Array.from(
    { length: rows },
    (_, r) => `|${Array.from({ length: cols }, (_, c) => ` r${r}c${c} `).join("|")}|`,
  ).join("\n");
  return `${header}\n${sep}\n${body}\n`;
}

function genCodeBlocks(n = 100) {
  return Array.from(
    { length: n },
    (_, i) => `\`\`\`rust\nfn func_${i}() {\n    println!("hello");\n}\n\`\`\`\n\n`,
  ).join("");
}

// ─── Run WASM benchmarks ────────────────────────────────────────────

const { parse: ironmarkParse } = await import("../wasm/node.js");
const markdownWasm = require("markdown-wasm/dist/markdown.node.js");
const { init: md4wInit, mdToHtml } = await import("md4w");
await md4wInit();

const wasmParsers = {
  ironmark: (input) => ironmarkParse(input),
  "markdown-wasm": (input) => markdownWasm.parse(input),
  md4w: (input) => mdToHtml(input),
};

function runWasmBench(name, input, iterations = 500) {
  const results = {};
  for (const [lib, fn] of Object.entries(wasmParsers)) {
    for (let i = 0; i < 50; i++) fn(input);
    const times = [];
    for (let i = 0; i < iterations; i++) {
      const start = performance.now();
      fn(input);
      times.push(performance.now() - start);
    }
    times.sort((a, b) => a - b);
    results[lib] = {
      median_ns: times[Math.floor(times.length / 2)] * 1e6,
      mean_ns: (times.reduce((a, b) => a + b, 0) / times.length) * 1e6,
    };
  }
  return { name, bytes: input.length, results };
}

console.log("\nRunning WASM benchmarks...\n");

const specJson = readFileSync(join(ROOT, "tests/spec/spec-0.31.2.json"), "utf8");
const specMarkdown = JSON.parse(specJson)
  .map((t) => t.markdown)
  .join("\n");

const wasmSections = [
  {
    title: "CommonMark Spec",
    benches: [runWasmBench("spec (all examples)", specMarkdown)],
  },
  {
    title: "Document Sizes",
    benches: [1_000, 10_000, 100_000].map((size) => {
      const base = genInlineHeavy();
      const input = base.repeat(Math.ceil(size / base.length)).slice(0, size);
      return runWasmBench(`mixed ${fmtBytes(size)}`, input);
    }),
  },
  {
    title: "Block Types",
    benches: [
      runWasmBench("headings", genHeadings()),
      runWasmBench("nested lists", genNestedList()),
      runWasmBench("table (100x10)", genTable()),
      runWasmBench("code blocks", genCodeBlocks()),
    ],
  },
  {
    title: "Inline-heavy",
    benches: [runWasmBench("inline heavy", genInlineHeavy())],
  },
];

console.log("WASM benchmarks done.\n");

// ─── Read Rust criterion results ────────────────────────────────────

const KNOWN_RUST_LIBS = ["ironmark", "pulldown_cmark", "comrak", "markdown_rs"];

function readCriterionResults() {
  if (!existsSync(CRITERION_DIR)) return [];

  const groups = new Map();

  function walkSync(dir) {
    let entries;
    try {
      entries = readdirSync(dir, { withFileTypes: true });
    } catch {
      return;
    }
    for (const e of entries) {
      if (!e.isDirectory() || e.name === "report") continue;
      const full = join(dir, e.name);
      const estPath = join(full, "new", "estimates.json");
      if (existsSync(estPath)) {
        const rel = full.slice(CRITERION_DIR.length + 1);
        const parts = rel.split("/");
        const libIdx = parts.findIndex((p) => KNOWN_RUST_LIBS.includes(p));
        if (libIdx === -1) continue;
        const groupName = parts.slice(0, libIdx).join("/");
        const libName = parts[libIdx];
        const label = parts.slice(libIdx + 1).join("/");
        const bytes = parseInt(label) || 0;

        const est = JSON.parse(readFileSync(estPath, "utf8"));
        if (!groups.has(groupName)) groups.set(groupName, { bytes, results: {} });
        groups.get(groupName).results[libName] = {
          median_ns: est.median.point_estimate,
          mean_ns: est.mean.point_estimate,
        };
      } else {
        walkSync(full);
      }
    }
  }

  walkSync(CRITERION_DIR);

  return [...groups.entries()].map(([name, data]) => ({
    name,
    bytes: data.bytes,
    results: data.results,
  }));
}

const rustResults = readCriterionResults();
const hasRust = rustResults.length > 0;

if (!hasRust) {
  console.log("No Rust criterion results found in target/criterion/.");
}

const rustSections = [];
if (hasRust) {
  const sectionOrder = [
    ["commonmark_spec", "CommonMark Spec"],
    ["document_size", "Document Sizes"],
    ["block_types", "Block Types"],
    ["inline_heavy", "Inline-heavy"],
  ];

  for (const [prefix, title] of sectionOrder) {
    const matching = rustResults.filter(
      (r) =>
        r.name === prefix || r.name.startsWith(prefix + "_") || r.name.startsWith(prefix + "/"),
    );
    if (matching.length > 0) {
      rustSections.push({
        title,
        benches: matching.map((b) => {
          let name = b.name;
          if (name.startsWith(prefix + "_")) name = name.slice(prefix.length + 1);
          else if (name.startsWith(prefix + "/")) name = name.slice(prefix.length + 1);
          else if (name === prefix) name = prefix;
          return { ...b, name };
        }),
      });
    }
  }
}

// ─── Helpers ────────────────────────────────────────────────────────

function fmtNs(ns) {
  if (ns < 1000) return `${ns.toFixed(0)} ns`;
  if (ns < 1e6) return `${(ns / 1000).toFixed(1)} \u00b5s`;
  return `${(ns / 1e6).toFixed(2)} ms`;
}

function fmtBytes(b) {
  if (b < 1024) return `${b} B`;
  if (b < 1024 * 1024) return `${(b / 1024).toFixed(1)} KB`;
  return `${(b / 1024 / 1024).toFixed(1)} MB`;
}

function throughput(bytes, ns) {
  if (!bytes || !ns) return "-";
  return `${(bytes / (1024 * 1024) / (ns / 1e9)).toFixed(1)} MB/s`;
}

function esc(s) {
  return s.replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;");
}

// ─── SVG rendering ──────────────────────────────────────────────────

const W = 880;
const PAD = 32;
const CONTENT_W = W - PAD * 2;
const CARD_PAD = 16;
const LABEL_W = 110;
const VALUE_W = 75;
const TP_W = 70;
const BAR_GAP = 12;
const BAR_W = CONTENT_W - CARD_PAD * 2 - LABEL_W - VALUE_W - TP_W - BAR_GAP * 3;
const BAR_H = 20;
const BAR_ROW_H = 28;

const RUST_LIBS = ["ironmark", "pulldown_cmark", "comrak", "markdown_rs"];
const WASM_LIBS = ["ironmark", "markdown-wasm", "md4w"];

const RUST_COLORS = {
  ironmark: "#e8590c",
  pulldown_cmark: "#1971c2",
  comrak: "#2f9e44",
  markdown_rs: "#c2185b",
};

const WASM_COLORS = {
  ironmark: "#e8590c",
  "markdown-wasm": "#7048e8",
  md4w: "#0ea5e9",
};

const RUST_LABELS = {
  ironmark: "ironmark",
  pulldown_cmark: "pulldown-cmark",
  comrak: "comrak",
  markdown_rs: "markdown-rs",
};

const WASM_LABELS = {
  ironmark: "ironmark",
  "markdown-wasm": "markdown-wasm",
  md4w: "md4w",
};

function buildSvgSection(title, subtitle, sections, libs, colorMap, labelMap) {
  let y = 0;
  const parts = [];

  // Title
  parts.push(
    `<text x="${W / 2}" y="${y + 24}" text-anchor="middle" fill="#fff" font-size="20" font-weight="700">${esc(title)}</text>`,
  );
  y += 32;
  parts.push(
    `<text x="${W / 2}" y="${y + 14}" text-anchor="middle" fill="#888" font-size="12">${esc(subtitle)}</text>`,
  );
  y += 28;

  // Legend
  const legendItems = libs.map((lib) => ({ lib, color: colorMap[lib], label: labelMap[lib] }));
  const legendItemW = 140;
  const legendTotalW = legendItems.length * legendItemW;
  let lx = (W - legendTotalW) / 2;
  for (const item of legendItems) {
    parts.push(`<rect x="${lx}" y="${y}" width="10" height="10" rx="2" fill="${item.color}"/>`);
    parts.push(
      `<text x="${lx + 16}" y="${y + 9}" fill="#aaa" font-size="11">${esc(item.label)}</text>`,
    );
    lx += legendItemW;
  }
  y += 24;

  for (const section of sections) {
    // Section title
    y += 16;
    parts.push(
      `<text x="${PAD}" y="${y + 14}" fill="#ccc" font-size="14" font-weight="600">${esc(section.title)}</text>`,
    );
    y += 24;

    for (const b of section.benches) {
      const entries = libs
        .map((lib) => (b.results[lib] ? { lib, ...b.results[lib] } : null))
        .filter(Boolean);
      if (entries.length === 0) continue;

      entries.sort((a, b) => a.median_ns - b.median_ns);

      const winner = entries[0];
      const maxTp = Math.max(
        ...entries.map((e) => (b.bytes ? b.bytes / e.median_ns : 1 / e.median_ns)),
      );

      const label = b.bytes ? `${b.name} (${fmtBytes(b.bytes)})` : b.name;

      let speedup = "";
      if (entries.length >= 2) {
        const ratio = entries[1].median_ns / winner.median_ns;
        speedup = ratio > 1.01 ? `${ratio.toFixed(1)}x faster` : "~tied";
      }

      const cardH = CARD_PAD * 2 + 22 + entries.length * BAR_ROW_H;

      // Card background
      parts.push(
        `<rect x="${PAD}" y="${y}" width="${CONTENT_W}" height="${cardH}" rx="6" fill="#1a1a1a" stroke="#2a2a2a"/>`,
      );

      // Card header
      const hy = y + CARD_PAD + 12;
      parts.push(
        `<text x="${PAD + CARD_PAD}" y="${hy}" fill="#aaa" font-size="12" font-weight="500">${esc(label)}</text>`,
      );

      // Winner badge
      const badgeText = `${labelMap[winner.lib]}  ${speedup}`;
      const badgeW = badgeText.length * 6.5 + 20;
      const badgeX = PAD + CONTENT_W - CARD_PAD - badgeW;
      parts.push(
        `<rect x="${badgeX}" y="${hy - 12}" width="${badgeW}" height="18" rx="3" fill="none" stroke="${colorMap[winner.lib]}" stroke-opacity="0.6"/>`,
      );
      parts.push(
        `<text x="${badgeX + 8}" y="${hy - 0.5}" fill="#fff" font-size="10" font-weight="600">${esc(labelMap[winner.lib])}</text>`,
      );
      if (speedup) {
        const speedColor = speedup === "~tied" ? "#888" : "#8f8";
        parts.push(
          `<text x="${badgeX + 8 + labelMap[winner.lib].length * 6.2 + 8}" y="${hy - 0.5}" fill="${speedColor}" font-size="9.5">${esc(speedup)}</text>`,
        );
      }

      // Bars
      let by = y + CARD_PAD + 28;
      for (const e of entries) {
        const tp = b.bytes ? b.bytes / e.median_ns : 1 / e.median_ns;
        const pct = tp / maxTp;
        const color = colorMap[e.lib] || "#888";
        const isWinner = e === winner;
        const labelColor = isWinner ? "#fff" : "#777";
        const labelWeight = isWinner ? "600" : "400";

        const bx = PAD + CARD_PAD;

        // Library label
        parts.push(
          `<text x="${bx + LABEL_W - 4}" y="${by + 14}" text-anchor="end" fill="${labelColor}" font-size="11" font-weight="${labelWeight}">${esc(labelMap[e.lib])}</text>`,
        );

        // Bar track
        const trackX = bx + LABEL_W + BAR_GAP;
        parts.push(
          `<rect x="${trackX}" y="${by + 2}" width="${BAR_W}" height="${BAR_H}" rx="3" fill="#252525"/>`,
        );

        // Bar fill
        const fillW = Math.max(2, BAR_W * pct);
        parts.push(
          `<rect x="${trackX}" y="${by + 2}" width="${fillW.toFixed(1)}" height="${BAR_H}" rx="3" fill="${color}"/>`,
        );

        // Median value
        const valX = trackX + BAR_W + BAR_GAP;
        parts.push(
          `<text x="${valX + VALUE_W - 4}" y="${by + 14}" text-anchor="end" fill="#e0e0e0" font-size="11">${esc(fmtNs(e.median_ns))}</text>`,
        );

        // Throughput
        const tpX = valX + VALUE_W + BAR_GAP;
        parts.push(
          `<text x="${tpX + TP_W - 4}" y="${by + 14}" text-anchor="end" fill="#666" font-size="10">${esc(throughput(b.bytes, e.median_ns))}</text>`,
        );

        by += BAR_ROW_H;
      }

      y += cardH + 8;
    }
  }

  return { svg: parts.join("\n"), height: y };
}

// ─── Build final SVG ────────────────────────────────────────────────

const svgParts = [];
let totalY = 0;

// Header
svgParts.push(
  `<text x="${W / 2}" y="${totalY + 28}" text-anchor="middle" fill="#fff" font-size="26" font-weight="700" letter-spacing="-0.5">ironmark</text>`,
);
totalY += 36;
svgParts.push(
  `<text x="${W / 2}" y="${totalY + 14}" text-anchor="middle" fill="#888" font-size="13">benchmark results</text>`,
);
totalY += 36;

if (hasRust) {
  const rust = buildSvgSection(
    "Native Rust",
    "ironmark vs pulldown-cmark vs comrak vs markdown-rs \u2014 cargo bench (criterion)",
    rustSections,
    RUST_LIBS,
    RUST_COLORS,
    RUST_LABELS,
  );
  svgParts.push(`<g transform="translate(0,${totalY})">${rust.svg}</g>`);
  totalY += rust.height + 24;
}

{
  const wasm = buildSvgSection(
    "WASM (Node.js)",
    "ironmark vs markdown-wasm vs md4w \u2014 median of 500 iterations",
    wasmSections,
    WASM_LIBS,
    WASM_COLORS,
    WASM_LABELS,
  );
  svgParts.push(`<g transform="translate(0,${totalY})">${wasm.svg}</g>`);
  totalY += wasm.height + 24;
}

// Footer
svgParts.push(
  `<text x="${W / 2}" y="${totalY + 12}" text-anchor="middle" fill="#666" font-size="10">Bars show throughput (longer is faster). Generated on ${new Date().toISOString().slice(0, 10)}.</text>`,
);
totalY += 32;

const svg = `<svg xmlns="http://www.w3.org/2000/svg" width="${W}" height="${totalY}" viewBox="0 0 ${W} ${totalY}">
<style>text { font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, Helvetica, Arial, sans-serif; }</style>
<rect width="${W}" height="${totalY}" fill="#0f0f0f"/>
${svgParts.join("\n")}
</svg>`;

const outPath = join(ROOT, "benchmark", "results.svg");
writeFileSync(outPath, svg);
console.log(`\nReport written to ${outPath}`);
