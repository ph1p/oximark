export type MarkdownInput = string | Uint8Array | ArrayBuffer | ArrayBufferView;

export interface ParseOptions {
  /** When true, every newline in a paragraph becomes a hard line break (`<br />`). Default: true. */
  hardBreaks?: boolean;
  /** Enable ==highlight== syntax for `<mark>`. Default: true. */
  enableHighlight?: boolean;
  /** Enable ~~strikethrough~~ syntax for `<del>`. Default: true. */
  enableStrikethrough?: boolean;
  /** Enable ++underline++ syntax for `<u>`. Default: true. */
  enableUnderline?: boolean;
  /** Enable pipe table syntax. Default: true. */
  enableTables?: boolean;
  /** Automatically detect bare URLs and emails and wrap them in links. Default: true. */
  enableAutolink?: boolean;
  /** Enable GitHub-style task lists (`- [ ] unchecked`, `- [x] checked`). Default: true. */
  enableTaskLists?: boolean;
  /** When true, raw HTML blocks and inline HTML are both escaped (XSS prevention). Default: false. */
  disableRawHtml?: boolean;
  /** Auto-generate `id=` attributes on headings from their slugified text. Default: false. */
  enableHeadingIds?: boolean;
  /** Render an `<a class="anchor">` inside each heading (implies heading IDs). Default: false. */
  enableHeadingAnchors?: boolean;
  /** When false, 4-space-indented code blocks are disabled (treated as paragraphs). Default: true. */
  enableIndentedCodeBlocks?: boolean;
  /** Disable HTML block constructs (escape them as text). Default: false. */
  noHtmlBlocks?: boolean;
  /** Disable inline HTML spans (escape them as text). Default: false. */
  noHtmlSpans?: boolean;
  /** Enable GFM tag filter: escape dangerous tags like `<script>`, `<iframe>`, etc. Default: false. */
  tagFilter?: boolean;
  /** Collapse runs of spaces/tabs in text nodes to a single space. Default: false. */
  collapseWhitespace?: boolean;
  /** Allow ATX headings without a space after `#` (e.g. `#Heading`). Default: false. */
  permissiveAtxHeaders?: boolean;
  /** Enable `[[wiki link]]` syntax → `<a href="...">`. Default: false. */
  enableWikiLinks?: boolean;
  /** Enable `$...$` and `$$...$$` math syntax with HTML-escaped content. Default: false. */
  enableLatexMath?: boolean;
}

/**
 * Initialize the WASM module.
 *
 * - **Node.js**: This is a no-op — WASM is embedded and loaded synchronously at import time.
 * - **Browser/Bundler**: Must be called (and awaited) before using `parse()`.
 *   Optionally pass a URL or `WebAssembly.Module` to override the default WASM location.
 *   Calling `init()` multiple times is safe (subsequent calls are no-ops).
 */
export declare function init(input?: string | URL | WebAssembly.Module): Promise<void>;

/**
 * Parse Markdown to HTML.
 *
 * @param markdown - Markdown source (string or binary).
 * @param options - Optional parsing options.
 * @returns HTML string.
 */
export declare function parse(markdown: MarkdownInput, options?: ParseOptions): string;

/**
 * Parse Markdown and return the block-level AST as a JSON string.
 *
 * @param markdown - Markdown source (string or binary).
 * @param options - Optional parsing options.
 * @returns JSON string representing the AST.
 */
export declare function parseToAst(markdown: MarkdownInput, options?: ParseOptions): string;

/**
 * Options for the ANSI terminal renderer.
 */
export interface AnsiOptions {
  /**
   * Terminal column width for word-wrap, heading underlines, and thematic breaks.
   * Set to `0` to disable all width-dependent formatting. Default: `80`.
   */
  width?: number;
  /**
   * Emit ANSI 256-colour escape codes. Set to `false` for plain-text output
   * (e.g. when piping to a file or a non-colour terminal). Default: `true`.
   */
  color?: boolean;
  /**
   * Show line numbers in fenced code blocks, right-aligned to the total line
   * count and separated from the code by a `│` border. Default: `false`.
   */
  lineNumbers?: boolean;
  /**
   * Horizontal padding to add on both sides of every output line.
   * The output width remains `width`; padding reduces the available text area.
   * Also adds `ceil(padding / 2)` blank lines at the top. Default: `0`.
   */
  padding?: number;
}

/**
 * Render Markdown as ANSI-coloured terminal output.
 *
 * Produces a string containing ANSI 256-colour escape codes suitable for
 * display in a terminal emulator. Headings, code blocks, inline code,
 * blockquotes, tables, and inline formatting are all styled distinctly.
 *
 * @param markdown - Markdown source (string or binary).
 * @param options - Optional parse options (same flags as `parse()`).
 * @param ansiOptions - Optional ANSI rendering options (width, color, lineNumbers).
 * @returns String with ANSI escape codes (or plain text when `color: false`).
 *
 * @example
 * ```ts
 * import { renderAnsi } from "ironmark";
 * const out = renderAnsi("# Hello\n\n**bold** and `code`");
 * process.stdout.write(out);
 * ```
 */
export declare function renderAnsi(
  markdown: MarkdownInput,
  options?: ParseOptions,
  ansiOptions?: AnsiOptions,
): string;
