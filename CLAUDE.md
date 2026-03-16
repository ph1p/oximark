# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project

**ironmark** — a fast Markdown-to-HTML parser in Rust, fully CommonMark 0.31.2 compliant (652/652 spec tests). Published as both a Rust crate and an npm package (via WASM). Zero third-party parsing dependencies (only `memchr` and `rustc-hash`).

## Commands

```bash
# Rust
cargo test --offline          # run all tests (spec + integration)
cargo test --offline -- commonmark  # run only CommonMark spec tests
cargo test --offline -- <name>      # run a specific test by name
cargo bench                   # criterion benchmarks (benchmark/parse.rs)
cargo fmt                     # format Rust code
cargo clippy                  # lint (deny undocumented_unsafe_blocks)

# WASM build
pnpm setup:wasm               # install wasm32 target + wasm-bindgen-cli
pnpm build                    # release WASM build
pnpm build:dev                # debug WASM build

# JS/TS
pnpm check                    # cargo fmt --check && oxlint && oxfmt --check
pnpm fmt                      # cargo fmt && oxfmt --write
pnpm lint                     # oxlint
```

## Architecture

Two-phase pipeline: **block parsing → inline parsing → HTML rendering**.

### Phase 1: Block parsing (`src/block/`)
- `mod.rs` — public `parse()` and `parse_to_ast()` entry points; block-level structures (open-block stack)
- `parser.rs` — line-by-line block parser; container tracking (blockquotes, lists)
- `leaf_blocks.rs` — ATX/setext headings, code blocks, thematic breaks
- `html_block.rs` — HTML block detection (7 conditions)
- `link_ref_def.rs` — link reference definition extraction

### Phase 2: Inline parsing (`src/inline/`)
- `mod.rs` — inline parser entry, delimiter run algorithm (emphasis, extensions)
- `scanner.rs` — character-level scanning, autolinks, entity resolution
- `links.rs` — link/image bracket matching, reference resolution
- `render.rs` — inline content → HTML string output

### HTML rendering (`src/render.rs`)
- Stack-based block renderer that calls inline parsing for leaf block content
- Receives `&ParseOptions` to control extension behavior

### Supporting modules
- `ast.rs` — AST node types (`Block` enum, `ListKind`, `TableData`, `TableAlignment`)
- `html.rs` — HTML escaping, URI sanitization, dangerous-protocol stripping
- `entities/` — HTML5 entity lookup tables
- `lib.rs` — `ParseOptions` struct, public API re-exports

### WASM layer (`wasm/`)
- `wasm/src/lib.rs` — wasm-bindgen bindings exposing `parse()` and `parseToAst()`
- `wasm/node.js` / `wasm/web.js` — JS entry points (node: sync embedded, web: async init)
- `wasm/shared.js` — shared JS logic between node/web
- `wasm/index.d.ts` — TypeScript type definitions

### Playground (`playground/`)
- Vite + TypeScript web app for interactive testing

## Key conventions

- Two public entry points: `parse(markdown, &ParseOptions)` → HTML string, `parse_to_ast(markdown, &ParseOptions)` → `Vec<Block>`
- Optional `serde` feature flag for AST serialization
- Spec tests run with `hard_breaks: false` and `enable_autolink: false` to match CommonMark baseline
- Security options: `disable_raw_html` (escapes HTML), `max_nesting_depth` (default 128), `max_input_size` (default 0 = unlimited)
- Extension delimiters (`~~`, `==`, `++`) use `tag_size` encoding: 1=em, 2=strong, 3=del, 4=mark, 5=u
- `#![deny(clippy::undocumented_unsafe_blocks)]` enforced in lib.rs
- Rust edition 2024; release profile uses LTO + codegen-units=1 + panic=abort
- Commits follow conventional commits (semantic-release via `.releaserc.cjs`)
- Workspace members: root crate (`ironmark`) + `wasm/` crate (`ironmark-wasm`)
