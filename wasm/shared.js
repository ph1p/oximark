const decoder = new TextDecoder("utf-8");

function toStr(markdown) {
  if (typeof markdown === "string") return markdown;
  if (markdown instanceof Uint8Array) return decoder.decode(markdown);
  if (markdown instanceof ArrayBuffer) return decoder.decode(new Uint8Array(markdown));
  if (ArrayBuffer.isView(markdown))
    return decoder.decode(
      new Uint8Array(markdown.buffer, markdown.byteOffset, markdown.byteLength),
    );
  throw new TypeError("markdown must be a string, Uint8Array, ArrayBuffer, or Buffer");
}

function optionArgs(markdown, options) {
  return [
    toStr(markdown),
    options?.hardBreaks,
    options?.enableHighlight,
    options?.enableStrikethrough,
    options?.enableUnderline,
    options?.enableTables,
    options?.enableAutolink,
    options?.enableTaskLists,
    options?.disableRawHtml,
    options?.enableHeadingIds,
    options?.enableHeadingAnchors,
    options?.enableIndentedCodeBlocks,
    options?.noHtmlBlocks,
    options?.noHtmlSpans,
    options?.tagFilter,
    options?.collapseWhitespace,
    options?.permissiveAtxHeaders,
    options?.enableWikiLinks,
    options?.enableLatexMath,
  ];
}

export function createParse(wasmParse, wasmParseDefault = null) {
  return function parse(markdown, options) {
    const input = toStr(markdown);
    if (options == null) {
      if (wasmParseDefault) return wasmParseDefault(input);
      return wasmParse(
        input,
        undefined,
        undefined,
        undefined,
        undefined,
        undefined,
        undefined,
        undefined,
        undefined,
        undefined,
        undefined,
        undefined,
        undefined,
        undefined,
        undefined,
        undefined,
        undefined,
        undefined,
        undefined,
      );
    }
    return wasmParse(...optionArgs(input, options));
  };
}

export function createParseToAst(wasmParseToAst) {
  return function parseToAst(markdown, options) {
    const input = toStr(markdown);
    if (options == null) {
      return wasmParseToAst(
        input,
        undefined,
        undefined,
        undefined,
        undefined,
        undefined,
        undefined,
        undefined,
        undefined,
        undefined,
        undefined,
        undefined,
        undefined,
        undefined,
        undefined,
        undefined,
        undefined,
        undefined,
        undefined,
      );
    }
    return wasmParseToAst(...optionArgs(input, options));
  };
}

export function createRenderAnsi(wasmRenderAnsi) {
  return function renderAnsi(markdown, options, ansiOptions) {
    return wasmRenderAnsi(
      ...optionArgs(markdown, options),
      // width is plain u32 (not Option): 0 = use default (80)
      ansiOptions?.width ?? 0,
      ansiOptions?.color,
      ansiOptions?.lineNumbers,
      ansiOptions?.padding ?? 0,
    );
  };
}

export function createParseHtmlToAst(wasmParseHtmlToAst) {
  return function parseHtmlToAst(html, preserveUnknownAsHtml) {
    return wasmParseHtmlToAst(html, preserveUnknownAsHtml);
  };
}

export function createHtmlToMarkdown(wasmHtmlToMarkdown) {
  return function htmlToMarkdown(html, preserveUnknownAsHtml) {
    return wasmHtmlToMarkdown(html, preserveUnknownAsHtml);
  };
}

export function createRenderMarkdown(wasmRenderMarkdown) {
  return function renderMarkdown(astJson) {
    return wasmRenderMarkdown(astJson);
  };
}
