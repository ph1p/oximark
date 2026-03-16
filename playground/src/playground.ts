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

Write **markdown** on the left and see the _rendered HTML_ on the right.

## Features

- Live preview as you type
- Supports **bold**, *italic*, and \`code\`
- Links: [Example](https://example.com)
- Images: ![alt](https://placeholdit.com/200x200/dddddd/999999?font=inter)

## Code Block

\`\`\`rust
fn main() {
    println!("Hello, world!");
}
\`\`\`

## Table

| Name  | Score | Grade |
| :---- | ----: | :---: |
| Alice |    95 |   A   |
| Bob   |    82 |   B   |

## Blockquote

> Markdown is a lightweight markup language
> that you can use to add formatting to plain text.

---

1. First item
2. Second item
   - Nested bullet
   - Another one
3. Third item
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
