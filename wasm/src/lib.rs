use ironmark::{ParseOptions, parse as ironmark_parse, parse_to_ast as ironmark_parse_to_ast};
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
        ),
    );
    serde_json::to_string(&ast)
        .map_err(|err| JsValue::from_str(&format!("AST serialization failed: {err}")))
}
