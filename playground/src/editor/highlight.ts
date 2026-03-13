import { oneDarkHighlightStyle } from "@codemirror/theme-one-dark";
import { defaultHighlightStyle } from "@codemirror/language";
import { highlightCode } from "@lezer/highlight";
import type { LanguageSupport, Language } from "@codemirror/language";
import { isDark } from "./theme";
import { javascript } from "@codemirror/lang-javascript";
import { html } from "@codemirror/lang-html";
import { css } from "@codemirror/lang-css";
import { json } from "@codemirror/lang-json";
import { markdown } from "@codemirror/lang-markdown";

const langLoaders: Record<string, () => Promise<LanguageSupport>> = {
  javascript: () => Promise.resolve(javascript()),
  js: () => Promise.resolve(javascript()),
  typescript: () => Promise.resolve(javascript({ typescript: true })),
  ts: () => Promise.resolve(javascript({ typescript: true })),
  jsx: () => Promise.resolve(javascript({ jsx: true })),
  tsx: () => Promise.resolve(javascript({ jsx: true, typescript: true })),
  rust: () => import("@codemirror/lang-rust").then((m) => m.rust()),
  html: () => Promise.resolve(html()),
  css: () => Promise.resolve(css()),
  json: () => Promise.resolve(json()),
  python: () => import("@codemirror/lang-python").then((m) => m.python()),
  py: () => import("@codemirror/lang-python").then((m) => m.python()),
  yaml: () => import("@codemirror/lang-yaml").then((m) => m.yaml()),
  yml: () => import("@codemirror/lang-yaml").then((m) => m.yaml()),
  markdown: () => Promise.resolve(markdown()),
  md: () => Promise.resolve(markdown()),
};

const langCache = new Map<string, Language>();
const langLoadingCache = new Map<string, Promise<Language>>();

function ensureLanguage(name: string): Promise<Language> | undefined {
  if (langCache.has(name)) return undefined;
  let pending = langLoadingCache.get(name);
  if (pending) return pending;
  const loader = langLoaders[name];
  if (!loader) return undefined;
  pending = loader().then((support) => {
    langCache.set(name, support.language);
    langLoadingCache.delete(name);
    return support.language;
  });
  langLoadingCache.set(name, pending);
  return pending;
}

const escapeRe = /[&<>]/g;
const escapeMap: Record<string, string> = { "&": "&amp;", "<": "&lt;", ">": "&gt;" };
const escapeHtml = (s: string) => s.replace(escapeRe, (ch) => escapeMap[ch]);

function highlightCodeString(code: string, lang: string): string {
  const language = langCache.get(lang);
  if (!language) return escapeHtml(code);

  const tree = language.parser.parse(code);
  const parts: string[] = [];
  let pos = 0;
  const style = isDark() ? oneDarkHighlightStyle : defaultHighlightStyle;

  highlightCode(
    code,
    tree,
    style,
    (text, classes) => {
      const escaped = escapeHtml(text);
      parts.push(classes ? `<span class="${classes}">${escaped}</span>` : escaped);
      pos += text.length;
    },
    () => {
      parts.push("\n");
      pos++;
    },
  );

  if (pos < code.length) parts.push(escapeHtml(code.slice(pos)));
  return parts.join("");
}

function highlightBlock(block: HTMLElement, lang: string) {
  const code = block.textContent || "";
  if (code) block.innerHTML = highlightCodeString(code, lang);
}

export function highlightCodeBlocks(container: HTMLElement) {
  const blocks = container.querySelectorAll("pre code[class*='language-']");
  const pending: Promise<void>[] = [];

  for (let i = 0; i < blocks.length; i++) {
    const block = blocks[i] as HTMLElement;
    const match = block.className.match(/language-(\S+)/);
    if (!match || !langLoaders[match[1]]) continue;
    const lang = match[1];

    if (langCache.has(lang)) {
      highlightBlock(block, lang);
    } else {
      const promise = ensureLanguage(lang);
      if (promise) pending.push(promise.then(() => highlightBlock(block, lang)));
    }
  }

  if (pending.length) Promise.all(pending);
}
