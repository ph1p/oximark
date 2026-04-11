#!/usr/bin/env node
import { readFileSync } from "node:fs";
import { renderAnsi } from "./node.js";

const VERSION = JSON.parse(
  readFileSync(new URL("../package.json", import.meta.url), "utf8"),
).version;

const HELP = `ironmark — render Markdown as coloured terminal output

USAGE:
    ironmark --ansi [OPTIONS] [FILE...]

    When no FILE is given, reads from stdin. Use '-' for stdin explicitly.

OPTIONS:
    --ansi               Render as ANSI-coloured terminal output (required)
    --width N            Terminal column width (default: auto-detect, fallback 80)
    --no-color           Disable ANSI escape codes (plain text)
    -n, --line-numbers   Show line numbers in fenced code blocks
    --no-hard-breaks     Don't turn soft newlines into hard line breaks
    --no-tables          Disable pipe table syntax
    --no-highlight       Disable ==highlight== syntax
    --no-strikethrough   Disable ~~strikethrough~~ syntax
    --no-underline       Disable ++underline++ syntax
    --no-autolink        Disable bare URL auto-linking
    --no-task-lists      Disable - [x] task list syntax
    --math               Enable $inline$ and $$display$$ math
    --wiki-links         Enable [[wiki link]] syntax
    -h, --help           Print this help and exit
    -V, --version        Print version and exit

EXAMPLES:
    npx ironmark --ansi README.md
    npx ironmark --ansi --width 120 README.md
    npx ironmark --ansi --no-color README.md | less
    echo '# Hello' | npx ironmark --ansi
    cat doc.md | npx ironmark --ansi --math --wiki-links
`;

const args = process.argv.slice(2);
const files = [];
const parseOptions = {};
const ansiOptions = {};
let ansiMode = false;

for (let i = 0; i < args.length; i++) {
  switch (args[i]) {
    case "-h":
    case "--help":
      process.stdout.write(HELP);
      process.exit(0);
      break;
    case "-V":
    case "--version":
      console.log(`ironmark ${VERSION}`);
      process.exit(0);
      break;
    case "--ansi":
      ansiMode = true;
      break;
    case "--no-color":
    case "--no-colour":
      ansiOptions.color = false;
      break;
    case "-n":
    case "--line-numbers":
      ansiOptions.lineNumbers = true;
      break;
    case "--width": {
      const val = args[++i];
      if (val === undefined || Number.isNaN(Number(val))) {
        console.error("error: --width requires a numeric value");
        process.exit(2);
      }
      ansiOptions.width = Number(val);
      break;
    }
    case "--no-hard-breaks":
      parseOptions.hardBreaks = false;
      break;
    case "--no-tables":
      parseOptions.enableTables = false;
      break;
    case "--no-highlight":
      parseOptions.enableHighlight = false;
      break;
    case "--no-strikethrough":
      parseOptions.enableStrikethrough = false;
      break;
    case "--no-underline":
      parseOptions.enableUnderline = false;
      break;
    case "--no-autolink":
      parseOptions.enableAutolink = false;
      break;
    case "--no-task-lists":
      parseOptions.enableTaskLists = false;
      break;
    case "--math":
      parseOptions.enableLatexMath = true;
      break;
    case "--wiki-links":
      parseOptions.enableWikiLinks = true;
      break;
    default:
      if (args[i].startsWith("--width=")) {
        const val = Number(args[i].slice("--width=".length));
        if (Number.isNaN(val)) {
          console.error("error: --width requires a numeric value");
          process.exit(2);
        }
        ansiOptions.width = val;
      } else if (args[i].startsWith("-") && args[i] !== "-") {
        console.error(`error: unknown flag: ${args[i]}`);
        console.error("Run 'ironmark --help' for usage.");
        process.exit(2);
      } else {
        files.push(args[i]);
      }
  }
}

if (!ansiMode) {
  console.error("error: --ansi flag is required");
  console.error("Run 'ironmark --help' for usage.");
  process.exit(2);
}

// Auto-detect terminal width
if (ansiOptions.width === undefined) {
  ansiOptions.width = process.stdout.columns || 80;
}

let input = "";

if (files.length === 0) {
  input = readFileSync(0, "utf8");
} else {
  for (const file of files) {
    if (file === "-") {
      input += readFileSync(0, "utf8");
    } else {
      try {
        input += readFileSync(file, "utf8");
      } catch (err) {
        console.error(`error: ${file}: ${err.message}`);
        process.exit(1);
      }
    }
  }
}

process.stdout.write(renderAnsi(input, parseOptions, ansiOptions));
