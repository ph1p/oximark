import { isDark, subscribeThemeChange } from "./theme";

const MAX_INLINE_LENGTH = 60;
const DEFAULT_EXPAND_DEPTH = 2;

type JsonValue = string | number | boolean | null | JsonValue[] | { [key: string]: JsonValue };

export class AstTreeView {
  readonly dom: HTMLElement;
  private unsubscribeTheme: (() => void) | null = null;

  constructor(parent: HTMLElement) {
    this.dom = document.createElement("div");
    this.dom.className = "ast-tree";
    parent.appendChild(this.dom);
    this.unsubscribeTheme = subscribeThemeChange(() => {
      this.dom.classList.toggle("ast-tree--dark", isDark());
    });
    this.dom.classList.toggle("ast-tree--dark", isDark());
  }

  update(json: string) {
    let data: JsonValue;
    try {
      data = JSON.parse(json);
    } catch {
      this.dom.textContent = json;
      return;
    }
    this.dom.textContent = "";
    this.dom.appendChild(renderValue(data, 0));
  }

  destroy() {
    this.unsubscribeTheme?.();
    this.dom.remove();
  }
}

function inlinePreview(value: JsonValue): string {
  const json = JSON.stringify(value);
  if (json.length <= MAX_INLINE_LENGTH) return json;
  if (Array.isArray(value)) return `[... ${value.length} items]`;
  const keys = Object.keys(value as object);
  return `{... ${keys.length} keys}`;
}

function renderValue(value: JsonValue, depth: number): HTMLElement | Text {
  if (value === null) return spanCls("ast-null", "null");
  switch (typeof value) {
    case "string":
      return spanCls("ast-string", JSON.stringify(value));
    case "number":
      return spanCls("ast-number", String(value));
    case "boolean":
      return spanCls("ast-bool", String(value));
  }

  if (Array.isArray(value)) {
    if (value.length === 0) return spanCls("ast-bracket", "[]");
    return renderCollapsible("[", "]", value.length, depth, (container) => {
      for (let i = 0; i < value.length; i++) {
        const row = document.createElement("div");
        row.className = "ast-row";
        row.appendChild(renderValue(value[i], depth + 1));
        if (i < value.length - 1) row.appendChild(spanCls("ast-comma", ","));
        container.appendChild(row);
      }
    });
  }

  const entries = Object.entries(value as Record<string, JsonValue>);
  if (entries.length === 0) return spanCls("ast-bracket", "{}");
  return renderCollapsible("{", "}", entries.length, depth, (container) => {
    for (let i = 0; i < entries.length; i++) {
      const [key, val] = entries[i];
      const row = document.createElement("div");
      row.className = "ast-row";
      row.appendChild(spanCls("ast-key", JSON.stringify(key)));
      row.appendChild(spanCls("ast-colon", ": "));
      row.appendChild(renderValue(val, depth + 1));
      if (i < entries.length - 1) row.appendChild(spanCls("ast-comma", ","));
      container.appendChild(row);
    }
  });
}

function renderCollapsible(
  openBracket: string,
  closeBracket: string,
  count: number,
  depth: number,
  populate: (container: HTMLElement) => void,
): HTMLElement {
  const wrapper = document.createElement("span");
  wrapper.className = "ast-collapsible";

  const toggle = document.createElement("span");
  toggle.className = "ast-toggle";
  toggle.setAttribute("role", "button");
  toggle.setAttribute("tabindex", "0");

  const open = spanCls("ast-bracket", openBracket);
  const close = spanCls("ast-bracket", closeBracket);

  const preview = spanCls("ast-preview", "");

  const children = document.createElement("div");
  children.className = "ast-children";

  let expanded = depth < DEFAULT_EXPAND_DEPTH;
  let populated = false;

  function ensurePopulated() {
    if (!populated) {
      populated = true;
      populate(children);
    }
  }

  function applyState() {
    toggle.textContent = expanded ? "\u25BE" : "\u25B8";
    preview.textContent = expanded
      ? ""
      : ` ${inlinePreview({ _: count }).replace(
          /.*/,
          `${count} ${count === 1 ? "item" : "items"}`,
        )} `;
    preview.style.display = expanded ? "none" : "";
    children.style.display = expanded ? "" : "none";
    close.style.display = expanded ? "" : "none";
  }

  if (expanded) ensurePopulated();
  applyState();

  const handleToggle = () => {
    expanded = !expanded;
    if (expanded) ensurePopulated();
    applyState();
  };
  toggle.addEventListener("click", handleToggle);
  toggle.addEventListener("keydown", (e) => {
    if (e.key === "Enter" || e.key === " ") {
      e.preventDefault();
      handleToggle();
    }
  });

  wrapper.appendChild(toggle);
  wrapper.appendChild(open);
  wrapper.appendChild(preview);
  wrapper.appendChild(children);
  wrapper.appendChild(close);

  return wrapper;
}

function spanCls(cls: string, text: string): HTMLSpanElement {
  const el = document.createElement("span");
  el.className = cls;
  el.textContent = text;
  return el;
}
