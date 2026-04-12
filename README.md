# ironmark

[![CI](https://github.com/ph1p/ironmark/actions/workflows/ci.yml/badge.svg)](https://github.com/ph1p/ironmark/actions/workflows/ci.yml) [![npm](https://img.shields.io/npm/v/ironmark)](https://www.npmjs.com/package/ironmark) [![crates.io](https://img.shields.io/crates/v/ironmark)](https://crates.io/crates/ironmark)

Fast Markdown parser written in Rust with **zero third-party** parsing dependencies. Outputs HTML, AST, ANSI terminal, or Markdown. Fully compliant with [CommonMark 0.31.2](https://spec.commonmark.org/0.31.2/) (652/652 spec tests pass). Available as a Rust crate and as an npm package via WebAssembly.

## Table of Contents

- [Configuration](#configuration)
  - [Extensions](#extensions)
  - [Security](#security)
  - [Other Options](#other-options)
- [JavaScript / TypeScript](#javascript--typescript)
  - [Node.js](#nodejs)
  - [AST Output](#ast-output)
  - [HTML to Markdown](#html-to-markdown)
  - [AST to Markdown](#ast-to-markdown)
  - [ANSI Terminal Output](#ansi-terminal-output)
  - [Browser / Bundler](#browser--bundler)
- [CLI](#cli)
- [Rust](#rust)
- [C / C++](#c--c++)
- [Benchmarks](#benchmarks)
- [Development](#development)

## Configuration

### Extensions (default `true`)

| Option              | JS (`camelCase`)           | Rust (`snake_case`)           | Description                                            |
| ------------------- | -------------------------- | ----------------------------- | ------------------------------------------------------ |
| Hard breaks         | `hardBreaks`               | `hard_breaks`                 | Every newline becomes `<br />`                         |
| Highlight           | `enableHighlight`          | `enable_highlight`            | `==text==` → `<mark>`                                  |
| Strikethrough       | `enableStrikethrough`      | `enable_strikethrough`        | `~~text~~` → `<del>`                                   |
| Underline           | `enableUnderline`          | `enable_underline`            | `++text++` → `<u>`                                     |
| Tables              | `enableTables`             | `enable_tables`               | Pipe table syntax                                      |
| Autolink            | `enableAutolink`           | `enable_autolink`             | Bare URLs & emails → `<a>`                             |
| Task lists          | `enableTaskLists`          | `enable_task_lists`           | `- [ ]` / `- [x]` checkboxes                           |
| Indented code       | `enableIndentedCodeBlocks` | `enable_indented_code_blocks` | 4-space indent → `<pre><code>`                         |
| Wiki links          | `enableWikiLinks`          | `enable_wiki_links`           | `[[page]]` → `<a href="page">`                         |
| LaTeX math          | `enableLateXMath`          | `enable_latex_math`           | `$inline$` and `$$display$$` → `<span class="math-…">` |
| Heading IDs         | `enableHeadingIds`         | `enable_heading_ids`          | Auto `id=` on headings from slugified text             |
| Heading anchors     | `enableHeadingAnchors`     | `enable_heading_anchors`      | `<a class="anchor">` inside each heading (implies IDs) |
| Permissive headings | `permissiveAtxHeaders`     | `permissive_atx_headers`      | Allow `#Heading` without space after `#`               |

### Security

| Option           | JS (`camelCase`) | Rust (`snake_case`) | Default        | Description                                                        |
| ---------------- | ---------------- | ------------------- | -------------- | ------------------------------------------------------------------ |
| Disable raw HTML | `disableRawHtml` | `disable_raw_html`  | `false`        | Escape **all** HTML blocks and inline HTML                         |
| No HTML blocks   | `noHtmlBlocks`   | `no_html_blocks`    | `false`        | Escape block-level HTML only (more granular than `disableRawHtml`) |
| No HTML spans    | `noHtmlSpans`    | `no_html_spans`     | `false`        | Escape inline HTML only                                            |
| Tag filter       | `tagFilter`      | `tag_filter`        | `false`        | GFM tag filter: escape `<script>`, `<iframe>`, etc.                |
| Max nesting      | —                | `max_nesting_depth` | `128`          | Limit blockquote/list nesting depth (DoS prevention)               |
| Max input size   | —                | `max_input_size`    | `0` (no limit) | Truncate input beyond this byte count                              |

> In the WASM build, `max_nesting_depth` is fixed at `128` and `max_input_size` at `10 MB`.

Dangerous URI schemes (`javascript:`, `vbscript:`, `data:` except `data:image/…`) are **always** stripped from link and image destinations, regardless of options.

### Other Options

| Option              | JS (`camelCase`)     | Rust (`snake_case`)   | Default | Description                                       |
| ------------------- | -------------------- | --------------------- | ------- | ------------------------------------------------- |
| Collapse whitespace | `collapseWhitespace` | `collapse_whitespace` | `false` | Collapse runs of spaces/tabs in text to one space |

## JavaScript / TypeScript

```bash
npm install ironmark
# or
pnpm add ironmark
```

### Node.js

WASM is embedded and loaded synchronously — no `init()` needed:

```ts
import { parse } from "ironmark";

const html = parse("# Hello\n\nThis is **fast**.");

// safe mode for untrusted input
const safe = parse(userInput, { disableRawHtml: true });
```

### AST Output

Use `parseToAst()` when you need the block-level document structure instead of rendered HTML.

```ts
import { parseToAst } from "ironmark";

const astJson = parseToAst("# Hello\n\n- [x] done");
const ast = JSON.parse(astJson);
```

`parseToAst()` returns a JSON string for portability across JS runtimes and WASM boundaries.

### HTML to Markdown

Convert HTML back to Markdown syntax using `htmlToMarkdown()`. Useful for importing content from HTML sources or round-trip conversion.

```ts
import { htmlToMarkdown } from "ironmark";

const md = htmlToMarkdown("<h1>Hello</h1><p><strong>Bold</strong> text</p>");
// Returns: "# Hello\n\n**Bold** text"

// Preserve unknown HTML tags (e.g., <sup>, <sub>) as raw HTML in output
const md = htmlToMarkdown("<p>H<sub>2</sub>O</p>", true);
// Returns: "H<sub>2</sub>O"
```

For AST access, use `parseHtmlToAst()`:

```ts
import { parseHtmlToAst } from "ironmark";

const astJson = parseHtmlToAst("<h1>Hello</h1><p>World</p>");
const ast = JSON.parse(astJson);
```

### AST to Markdown

Render an AST back to Markdown syntax using `renderMarkdown()`. Combined with `parseToAst()` or `parseHtmlToAst()`, this enables round-trip conversion.

```ts
import { parseToAst, renderMarkdown } from "ironmark";

const ast = parseToAst("# Hello\n\n**World**");
const md = renderMarkdown(ast);
// Returns: "# Hello\n\n**World**"
```

### ANSI Terminal Output

Use `renderAnsi()` to render Markdown as coloured terminal output (ANSI 256-colour escape codes). Useful for CLI tools, terminal UIs, or any environment with a TTY.

````ts
import { renderAnsi } from "ironmark";

// Render with defaults (width 80, colour enabled)
const ansi = renderAnsi("# Hello\n\n**bold** and `code`");
process.stdout.write(ansi);

// Custom terminal width and line numbers in code blocks
const ansi = renderAnsi(
  "# Hello\n\n```rust\nfn main() {}\n```",
  {}, // parse options (same as parse())
  { width: 120, lineNumbers: true },
);
process.stdout.write(ansi);

// With padding — adds horizontal spacing on both sides
const ansi = renderAnsi("# Hello\n\n> A quote", {}, { padding: 2 });
process.stdout.write(ansi);

// Plain text — strips all ANSI codes (useful for piping to files)
const plain = renderAnsi("# Hello\n\n> quote", {}, { color: false });
````

#### ANSI options

| Option        | Type      | Default | Description                                                                                   |
| ------------- | --------- | ------- | --------------------------------------------------------------------------------------------- |
| `width`       | `number`  | `80`    | Column width for word-wrap, heading underlines, rule length. `0` = use default.               |
| `color`       | `boolean` | `true`  | Emit ANSI colour codes. `false` = plain text output.                                          |
| `lineNumbers` | `boolean` | `false` | Show line numbers in fenced code blocks.                                                      |
| `padding`     | `number`  | `0`     | Horizontal padding added to both sides of each line, plus ⌈padding/2⌉ blank lines at the top. |

### Browser / Bundler

Call `init()` once before using `parse()`. It's idempotent and optionally accepts a custom `.wasm` URL.

```ts
import { init, parse } from "ironmark";

await init();

const html = parse("# Hello\n\nThis is **fast**.");
```

#### Vite

```ts
import { init, parse } from "ironmark";
import wasmUrl from "ironmark/ironmark.wasm?url";

await init(wasmUrl);

const html = parse("# Hello\n\nThis is **fast**.");
```

## CLI

Render Markdown as coloured terminal output. Two ways to install:

### npm

```bash
npx ironmark --ansi README.md
```

Or install globally:

```bash
npm install -g ironmark
ironmark --ansi README.md
```

### Rust

Native binary — faster startup, auto-detects terminal width via `$COLUMNS` / `tput cols`.

```bash
cargo install ironmark --features cli
ironmark --ansi README.md
```

### Options

Both CLIs support the same flags (the npm CLI requires `--ansi` as the first flag):

```text
OPTIONS:
    --width N            Terminal column width (default: auto-detect, fallback 80)
    --padding N          Horizontal padding added to both sides of each line, plus ceil(padding/2) blank lines at the top (default: 0)
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
    --max-size N         Truncate input to N bytes (Rust only)
    -h, --help           Print this help and exit
    -V, --version        Print version and exit
```

### Examples

```bash
# npm
npx ironmark --ansi README.md
npx ironmark --ansi --width 120 README.md
echo '# Hello' | npx ironmark --ansi
npx ironmark --ansi --no-color README.md | less

# Rust (after cargo install ironmark --features cli)
ironmark --ansi README.md
ironmark --ansi --width 120 README.md
echo '# Hello' | ironmark --ansi
cat doc.md | ironmark --ansi --math --wiki-links
```

## Rust

```bash
cargo add ironmark
```

```rust
use ironmark::{parse, ParseOptions};

fn main() {
    // with defaults
    let html = parse("# Hello\n\nThis is **fast**.", &ParseOptions::default());

    // with custom options
    let html = parse("line one\nline two", &ParseOptions {
        hard_breaks: false,
        enable_strikethrough: false,
        ..Default::default()
    });

    // safe mode for untrusted input
    let html = parse("<script>alert(1)</script>", &ParseOptions {
        disable_raw_html: true,
        max_input_size: 1_000_000, // 1 MB
        ..Default::default()
    });
}
```

### AST Output

`parse_to_ast()` returns the typed Rust AST (`Block`) directly:

```rust
use ironmark::{Block, ParseOptions, parse_to_ast};

fn main() {
    let ast = parse_to_ast("# Hello", &ParseOptions::default());

    match ast {
        Block::Document { children } => {
            println!("top-level blocks: {}", children.len());
        }
        _ => unreachable!("root nodes always Document"),
    }
}
```

Exported AST types:

- `Block`
- `ListKind`
- `TableData`
- `TableAlignment`

### HTML to Markdown

Convert HTML back to Markdown syntax:

```rust
use ironmark::{html_to_markdown, HtmlParseOptions};

fn main() {
    let md = html_to_markdown(
        "<h1>Hello</h1><p><strong>Bold</strong> text</p>",
        &HtmlParseOptions::default(),
    );
    // Returns: "# Hello\n\n**Bold** text"
}
```

For AST access, use `parse_html_to_ast()`:

```rust
use ironmark::{parse_html_to_ast, HtmlParseOptions, UnknownInlineHandling};

fn main() {
    // Default: strip unknown tags, keep text content
    let ast = parse_html_to_ast("<p>H<sub>2</sub>O</p>", &HtmlParseOptions::default());

    // Preserve unknown tags as raw HTML
    let ast = parse_html_to_ast(
        "<p>H<sub>2</sub>O</p>",
        &HtmlParseOptions {
            unknown_inline_handling: UnknownInlineHandling::PreserveAsHtml,
            ..Default::default()
        },
    );
}
```

`HtmlParseOptions` fields:

| Field                     | Type                    | Default     | Description                           |
| ------------------------- | ----------------------- | ----------- | ------------------------------------- |
| `max_nesting_depth`       | `usize`                 | `128`       | Limit nesting depth (DoS prevention)  |
| `unknown_inline_handling` | `UnknownInlineHandling` | `StripTags` | How to handle unknown HTML tags       |
| `max_input_size`          | `usize`                 | `0`         | Truncate input beyond this byte count |

`UnknownInlineHandling` variants:

- `StripTags` — Remove unknown tags, keep text content (default)
- `PreserveAsHtml` — Keep unknown tags as raw HTML in output

### AST to Markdown

Render an AST back to Markdown syntax:

```rust
use ironmark::{parse_to_ast, render_markdown, ParseOptions};

fn main() {
    let ast = parse_to_ast("# Hello\n\n**World**", &ParseOptions::default());
    let md = render_markdown(&ast);
    // Returns: "# Hello\n\n**World**"
}
```

### ANSI Terminal Output

`render_ansi()` renders Markdown as ANSI-coloured terminal output. Pass `Some(&AnsiOptions { .. })` to control width, colour, and line numbers, or `None` for defaults.

````rust
use ironmark::{AnsiOptions, ParseOptions, render_ansi};

fn main() {
    // Defaults — width 80, colour enabled
    let out = render_ansi("# Hello\n\n**bold** and `code`", &ParseOptions::default(), None);
    print!("{out}");

    // Custom options — 120 columns, line numbers in code blocks
    let out = render_ansi(
        "# Hello\n\n```rust\nfn main() {}\n```",
        &ParseOptions::default(),
        Some(&AnsiOptions { width: 120, line_numbers: true, ..AnsiOptions::default() }),
    );
    print!("{out}");

    // With padding — adds horizontal spacing on both sides
    let out = render_ansi(
        "# Hello\n\n> A quote",
        &ParseOptions::default(),
        Some(&AnsiOptions { padding: 2, ..AnsiOptions::default() }),
    );
    print!("{out}");

    // Plain text — no ANSI codes (e.g. for writing to a file)
    let plain = render_ansi(
        "# Hello",
        &ParseOptions::default(),
        Some(&AnsiOptions { color: false, ..AnsiOptions::default() }),
    );
}
````

`AnsiOptions` fields:

| Field          | Type    | Default | Description                                                                                   |
| -------------- | ------- | ------- | --------------------------------------------------------------------------------------------- |
| `width`        | `usize` | `80`    | Column width for word-wrap, heading underlines, rule length. `0` = disable.                   |
| `color`        | `bool`  | `true`  | Emit ANSI 256-colour escape codes.                                                            |
| `line_numbers` | `bool`  | `false` | Show line numbers in fenced code blocks.                                                      |
| `padding`      | `usize` | `0`     | Horizontal padding added to both sides of each line, plus ⌈padding/2⌉ blank lines at the top. |

## C / C++

The crate compiles to a static library (`libironmark.a`) that exposes two C functions. A header is provided at `include/ironmark.h`.

### Build the library

```bash
cargo build --release
# output: target/release/libironmark.a
```

### Link

```sh
# Linux
cc -o example example.c -L target/release -l ironmark -lpthread -ldl

# macOS
cc -o example example.c -L target/release -l ironmark \
   -framework CoreFoundation -framework Security
```

### Usage

```c
#include "include/ironmark.h"
#include <stdio.h>

int main(void) {
    char *html = ironmark_parse("# Hello\n\nThis is **fast**.");
    if (html) {
        printf("%s\n", html);
        ironmark_free(html);
    }
    return 0;
}
```

**Memory contract**: `ironmark_parse` returns a heap-allocated string. You **must** free it with `ironmark_free`. Passing any other pointer to `ironmark_free` is undefined behaviour. Both functions are null-safe: `ironmark_parse(NULL)` returns `NULL`; `ironmark_free(NULL)` is a no-op.

Parsing always uses the default `ParseOptions` (all extensions enabled, `disable_raw_html` off). Options are not yet configurable through the C API.

## Benchmarks

Compares ironmark against pulldown-cmark, comrak, markdown-it, and markdown-rs. Results are available at [ph1p.js.org/ironmark/#benchmark](https://ph1p.js.org/ironmark/#benchmark).

```bash
cargo bench                          # run all benchmarks
cargo bench --features bench-md4c   # include md4c (requires: brew install md4c)
pnpm bench                          # run + update playground data
```

## Development

This project uses [pnpm](https://pnpm.io/) for package management.

### Build from source

```bash
pnpm setup:wasm
pnpm build
```

| Command           | Description            |
| ----------------- | ---------------------- |
| `pnpm setup:wasm` | Install prerequisites  |
| `pnpm build`      | Release WASM build     |
| `pnpm build:dev`  | Debug WASM build       |
| `pnpm test`       | Run Rust tests         |
| `pnpm check`      | Format check           |
| `pnpm clean`      | Remove build artifacts |

### Troubleshooting

**`wasm32-unknown-unknown target not found`** or **`wasm-bindgen not found`** — run `pnpm setup:wasm` to install all prerequisites
