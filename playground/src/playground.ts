import { init, parse, parseToAst } from "ironmark";
import wasmUrl from "ironmark/ironmark.wasm?url";
import type { EditorView } from "@codemirror/view";
import { cmThemeExtension, subscribeThemeChange } from "./editor/theme";
import { highlightCodeBlocks } from "./editor/highlight";
import { formatHtml } from "./util/format-html";
import { createHtmlView, htmlThemeCompartment } from "./editor/setup";
import { AstTreeView } from "./editor/ast-tree";
import type { OutputTab, ParseOptions } from "./components/types";
import { DEFAULT_PARSE_OPTIONS } from "./components/types";

export const DEFAULT_MARKDOWN = `# Markdown Playground

Write **markdown** here and see the rendered HTML live on the right.
ironmark supports all CommonMark 0.31.2 features plus a range of extensions.

---

## Inline formatting

**bold**, *italic*, \`inline code\`, ~~strikethrough~~, ==highlight==, ++underline++

Autolinks: https://github.com/anthonynmh/ironmark and email@example.com

HTML entities: &copy; &mdash; &frac12;

## Links & images

[CommonMark spec](https://spec.commonmark.org) — standard link

[Link with title](https://commonmark.org "CommonMark")

![Placeholder image](https://placehold.co/120x40/e2e8f0/475569?text=ironmark)

Reference-style: [ironmark][repo]

[repo]: https://github.com/anthonynmh/ironmark

## Lists

- Unordered item
- Another item
  - Nested bullet
  - Second nested

1. First ordered
2. Second ordered
   1. Sub-item one
   2. Sub-item two

### Task list

- [x] CommonMark 0.31.2 (652/652)
- [x] Tables, strikethrough, highlight, underline
- [x] Wiki links and LaTeX math
- [ ] Something left to do

## Blockquote

> Markdown is a lightweight markup language that you can use to add formatting
> to plain text documents.
>
> — John Gruber

## Code

Fenced code block with syntax highlight hint:

\`\`\`rust
fn main() {
    let md = "# Hello, ironmark!";
    let html = ironmark::parse(md, &Default::default());
    println!("{html}");
}
\`\`\`

Indented code block (4-space indent):

    let x = 42;
    println!("{x}");

Inline: \`let x = 42;\`

## Table

| Parser            | Median (ns) | Relative |
| :---------------- | ----------: | :------: |
| ironmark          |         210 |   1.00×  |
| pulldown-cmark    |         310 |   1.48×  |
| comrak            |         540 |   2.57×  |

## Wiki links *(enable in Options)*

[[Getting Started]]   [[Installation Guide]]

## LaTeX math *(enable in Options)*

Inline math: $E = mc^2$ and $\\sum_{i=1}^{n} i = \\frac{n(n+1)}{2}$

Display math:

$$\\int_0^\\infty e^{-x^2} dx = \\frac{\\sqrt{\\pi}}{2}$$

## Hard breaks *(toggle in Options)*

Line one
Line two (hard break when enabled)

---

*ironmark — fast CommonMark-compliant Markdown in Rust*
`;

type InitPlaygroundArgs = {
  preview: HTMLDivElement;
  htmlSourceContainer: HTMLDivElement;
  astSourceContainer: HTMLDivElement;
  getOutputTab: () => OutputTab;
  onStatusChange: (text: string) => void;
};

export type PlaygroundController = {
  setOutputTab: (tab: OutputTab) => void;
  updateMarkdown: (markdown: string) => void;
  updateOptions: (options: ParseOptions) => void;
  attachEditorView: (editorView: EditorView) => void;
  getHtml: () => string;
  getAst: () => string;
  dispose: () => void;
};

export async function initPlayground(args: InitPlaygroundArgs): Promise<PlaygroundController> {
  await init(wasmUrl);

  let editorView: EditorView | null = null;
  let currentMarkdown = DEFAULT_MARKDOWN;
  let currentOptions: ParseOptions = { ...DEFAULT_PARSE_OPTIONS };
  const htmlState = { dirty: false, lastHtml: "", astDirty: false, lastAst: "" };
  const htmlView = createHtmlView(args.htmlSourceContainer);
  const astTree = new AstTreeView(args.astSourceContainer);
  let highlightRaf = 0;
  let htmlUpdateRaf = 0;
  let astUpdateRaf = 0;

  function flushOutputTab(tab: OutputTab) {
    if (tab === "html" && htmlState.dirty) {
      htmlState.dirty = false;
      htmlView.dispatch({
        changes: { from: 0, to: htmlView.state.doc.length, insert: formatHtml(htmlState.lastHtml) },
      });
    }

    if (tab === "ast" && htmlState.astDirty) {
      htmlState.astDirty = false;
      astTree.update(htmlState.lastAst);
    }
  }

  function parseMarkdown(md: string) {
    const o = currentOptions;
    const t0 = performance.now();
    const opts = {
      hardBreaks: o.hard_breaks,
      enableHighlight: o.enable_highlight,
      enableStrikethrough: o.enable_strikethrough,
      enableUnderline: o.enable_underline,
      enableTables: o.enable_tables,
      enableAutolink: o.enable_autolink,
      enableTaskLists: o.enable_task_lists,
      disableRawHtml: o.disable_raw_html,
      enableHeadingIds: o.enable_heading_ids,
      enableHeadingAnchors: o.enable_heading_anchors,
      enableIndentedCodeBlocks: o.enable_indented_code_blocks,
      noHtmlBlocks: o.no_html_blocks,
      noHtmlSpans: o.no_html_spans,
      tagFilter: o.tag_filter,
      collapseWhitespace: o.collapse_whitespace,
      permissiveAtxHeaders: o.permissive_atx_headers,
      enableWikiLinks: o.enable_wiki_links,
      enableLatexMath: o.enable_latex_math,
    };
    const html = parse(md, opts);
    args.onStatusChange(`${(performance.now() - t0).toFixed(2)}ms`);

    args.preview.innerHTML = html;
    htmlState.lastHtml = html;

    cancelAnimationFrame(highlightRaf);
    highlightRaf = requestAnimationFrame(() => highlightCodeBlocks(args.preview));

    cancelAnimationFrame(htmlUpdateRaf);
    if (args.getOutputTab() === "html") {
      htmlUpdateRaf = requestAnimationFrame(() => {
        htmlView.dispatch({
          changes: { from: 0, to: htmlView.state.doc.length, insert: formatHtml(html) },
        });
      });
    } else {
      htmlState.dirty = true;
    }

    const astJson = parseToAst(md, opts);
    htmlState.lastAst = astJson;

    cancelAnimationFrame(astUpdateRaf);
    if (args.getOutputTab() === "ast") {
      astUpdateRaf = requestAnimationFrame(() => {
        astTree.update(astJson);
      });
    } else {
      htmlState.astDirty = true;
    }
  }

  const onThemeChange = () => {
    const ext = cmThemeExtension();
    htmlView.dispatch({ effects: htmlThemeCompartment.reconfigure(ext) });
    parseMarkdown(currentMarkdown);
  };

  const unsubscribeTheme = subscribeThemeChange(onThemeChange);
  args.onStatusChange("");
  parseMarkdown(currentMarkdown);

  return {
    setOutputTab: flushOutputTab,
    updateMarkdown: (markdown) => {
      currentMarkdown = markdown;
      parseMarkdown(markdown);
    },
    updateOptions: (options) => {
      currentOptions = options;
      parseMarkdown(currentMarkdown);
    },
    attachEditorView: (view) => {
      editorView = view;
      editorView.focus();
    },
    getHtml: () => formatHtml(htmlState.lastHtml),
    getAst: () => htmlState.lastAst,
    dispose: () => {
      unsubscribeTheme();
      cancelAnimationFrame(highlightRaf);
      cancelAnimationFrame(htmlUpdateRaf);
      cancelAnimationFrame(astUpdateRaf);
      htmlView.destroy();
      astTree.destroy();
    },
  };
}
