use ironmark::{
    AnsiOptions, ParseOptions, parse as ironmark_parse, parse_to_ast as ironmark_parse_to_ast,
    render_ansi as ironmark_render_ansi,
};
use wasm_bindgen::prelude::*;

/// Maximum input size for WASM: 10 MB
const WASM_MAX_INPUT_SIZE: usize = 10 * 1024 * 1024;

fn build_options(
    hard_breaks: Option<bool>,
    enable_highlight: Option<bool>,
    enable_strikethrough: Option<bool>,
    enable_underline: Option<bool>,
    enable_tables: Option<bool>,
    enable_autolink: Option<bool>,
    enable_task_lists: Option<bool>,
    disable_raw_html: Option<bool>,
    enable_heading_ids: Option<bool>,
    enable_heading_anchors: Option<bool>,
    enable_indented_code_blocks: Option<bool>,
    no_html_blocks: Option<bool>,
    no_html_spans: Option<bool>,
    tag_filter: Option<bool>,
    collapse_whitespace: Option<bool>,
    permissive_atx_headers: Option<bool>,
    enable_wiki_links: Option<bool>,
    enable_latex_math: Option<bool>,
) -> ParseOptions {
    ParseOptions {
        hard_breaks: hard_breaks.unwrap_or(true),
        enable_highlight: enable_highlight.unwrap_or(true),
        enable_strikethrough: enable_strikethrough.unwrap_or(true),
        enable_underline: enable_underline.unwrap_or(true),
        enable_tables: enable_tables.unwrap_or(true),
        enable_autolink: enable_autolink.unwrap_or(true),
        enable_task_lists: enable_task_lists.unwrap_or(true),
        disable_raw_html: disable_raw_html.unwrap_or(false),
        max_nesting_depth: 128,
        max_input_size: WASM_MAX_INPUT_SIZE,
        enable_heading_ids: enable_heading_ids.unwrap_or(false),
        enable_heading_anchors: enable_heading_anchors.unwrap_or(false),
        enable_indented_code_blocks: enable_indented_code_blocks.unwrap_or(true),
        no_html_blocks: no_html_blocks.unwrap_or(false),
        no_html_spans: no_html_spans.unwrap_or(false),
        tag_filter: tag_filter.unwrap_or(false),
        collapse_whitespace: collapse_whitespace.unwrap_or(false),
        permissive_atx_headers: permissive_atx_headers.unwrap_or(false),
        enable_wiki_links: enable_wiki_links.unwrap_or(false),
        enable_latex_math: enable_latex_math.unwrap_or(false),
    }
}

#[wasm_bindgen]
pub fn parse(
    markdown: &str,
    hard_breaks: Option<bool>,
    enable_highlight: Option<bool>,
    enable_strikethrough: Option<bool>,
    enable_underline: Option<bool>,
    enable_tables: Option<bool>,
    enable_autolink: Option<bool>,
    enable_task_lists: Option<bool>,
    disable_raw_html: Option<bool>,
    enable_heading_ids: Option<bool>,
    enable_heading_anchors: Option<bool>,
    enable_indented_code_blocks: Option<bool>,
    no_html_blocks: Option<bool>,
    no_html_spans: Option<bool>,
    tag_filter: Option<bool>,
    collapse_whitespace: Option<bool>,
    permissive_atx_headers: Option<bool>,
    enable_wiki_links: Option<bool>,
    enable_latex_math: Option<bool>,
) -> String {
    ironmark_parse(
        markdown,
        &build_options(
            hard_breaks,
            enable_highlight,
            enable_strikethrough,
            enable_underline,
            enable_tables,
            enable_autolink,
            enable_task_lists,
            disable_raw_html,
            enable_heading_ids,
            enable_heading_anchors,
            enable_indented_code_blocks,
            no_html_blocks,
            no_html_spans,
            tag_filter,
            collapse_whitespace,
            permissive_atx_headers,
            enable_wiki_links,
            enable_latex_math,
        ),
    )
}

#[wasm_bindgen(js_name = "parseToAst")]
pub fn parse_to_ast(
    markdown: &str,
    hard_breaks: Option<bool>,
    enable_highlight: Option<bool>,
    enable_strikethrough: Option<bool>,
    enable_underline: Option<bool>,
    enable_tables: Option<bool>,
    enable_autolink: Option<bool>,
    enable_task_lists: Option<bool>,
    disable_raw_html: Option<bool>,
    enable_heading_ids: Option<bool>,
    enable_heading_anchors: Option<bool>,
    enable_indented_code_blocks: Option<bool>,
    no_html_blocks: Option<bool>,
    no_html_spans: Option<bool>,
    tag_filter: Option<bool>,
    collapse_whitespace: Option<bool>,
    permissive_atx_headers: Option<bool>,
    enable_wiki_links: Option<bool>,
    enable_latex_math: Option<bool>,
) -> Result<String, JsValue> {
    let ast = ironmark_parse_to_ast(
        markdown,
        &build_options(
            hard_breaks,
            enable_highlight,
            enable_strikethrough,
            enable_underline,
            enable_tables,
            enable_autolink,
            enable_task_lists,
            disable_raw_html,
            enable_heading_ids,
            enable_heading_anchors,
            enable_indented_code_blocks,
            no_html_blocks,
            no_html_spans,
            tag_filter,
            collapse_whitespace,
            permissive_atx_headers,
            enable_wiki_links,
            enable_latex_math,
        ),
    );
    serde_json::to_string(&ast)
        .map_err(|err| JsValue::from_str(&format!("AST serialization failed: {err}")))
}

/// Render Markdown as ANSI-coloured terminal output.
///
/// Produces a string containing ANSI 256-colour escape codes suitable for
/// display in a terminal emulator. Use `color: false` to get plain text
/// (all ANSI codes stripped).
///
/// @param markdown - Markdown source string.
/// @param options - Optional parse options (same as `parse()`).
/// @param width - Terminal column width for word-wrap and underlines (default: 80, 0 = use default).
/// @param color - Emit ANSI colour codes (default: true).
/// @param lineNumbers - Show line numbers in fenced code blocks (default: false).
#[wasm_bindgen(js_name = "renderAnsi")]
pub fn render_ansi(
    markdown: &str,
    hard_breaks: Option<bool>,
    enable_highlight: Option<bool>,
    enable_strikethrough: Option<bool>,
    enable_underline: Option<bool>,
    enable_tables: Option<bool>,
    enable_autolink: Option<bool>,
    enable_task_lists: Option<bool>,
    disable_raw_html: Option<bool>,
    enable_heading_ids: Option<bool>,
    enable_heading_anchors: Option<bool>,
    enable_indented_code_blocks: Option<bool>,
    no_html_blocks: Option<bool>,
    no_html_spans: Option<bool>,
    tag_filter: Option<bool>,
    collapse_whitespace: Option<bool>,
    permissive_atx_headers: Option<bool>,
    enable_wiki_links: Option<bool>,
    enable_latex_math: Option<bool>,
    // Plain u32, not Option<u32>: Option<u32> generates i32.trunc_sat_f64_u
    // (nontrapping-fptoint) which wasm-opt rejects without --enable-nontrapping-fptoint.
    // 0 means "use default (80)".
    width: u32,
    color: Option<bool>,
    line_numbers: Option<bool>,
) -> String {
    let parse_opts = build_options(
        hard_breaks,
        enable_highlight,
        enable_strikethrough,
        enable_underline,
        enable_tables,
        enable_autolink,
        enable_task_lists,
        disable_raw_html,
        enable_heading_ids,
        enable_heading_anchors,
        enable_indented_code_blocks,
        no_html_blocks,
        no_html_spans,
        tag_filter,
        collapse_whitespace,
        permissive_atx_headers,
        enable_wiki_links,
        enable_latex_math,
    );
    let ansi_opts = AnsiOptions {
        width: if width == 0 { 80 } else { width as usize },
        color: color.unwrap_or(true),
        line_numbers: line_numbers.unwrap_or(false),
        ..AnsiOptions::default()
    };
    ironmark_render_ansi(markdown, &parse_opts, Some(&ansi_opts))
}
