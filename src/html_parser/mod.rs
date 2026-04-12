//! HTML-to-AST parser for converting HTML back to Markdown.
//!
//! This module provides functionality to parse HTML and convert it back to the
//! same [`Block`](crate::Block) AST used by the Markdown parser, enabling
//! HTML-to-Markdown conversion.
//!
//! # Examples
//!
//! ```
//! use ironmark::{parse_html_to_ast, html_to_markdown, HtmlParseOptions, Block};
//!
//! // Parse HTML to AST
//! let ast = parse_html_to_ast("<h1>Hello</h1><p>World</p>", &HtmlParseOptions::default());
//!
//! // Convert HTML directly to Markdown
//! let md = html_to_markdown("<p><strong>Bold</strong> text</p>", &HtmlParseOptions::default());
//! assert!(md.contains("**Bold**"));
//! ```

mod inline;
mod parser;
mod tokenizer;

pub use parser::{HtmlParseOptions, UnknownInlineHandling, parse_html_to_ast};

use crate::renderers::markdown::render_markdown;

/// Parse HTML and render directly to Markdown.
///
/// This is a convenience function combining [`parse_html_to_ast`] and
/// [`render_markdown`](crate::render_markdown).
///
/// # Examples
///
/// ```
/// use ironmark::{html_to_markdown, HtmlParseOptions};
///
/// let md = html_to_markdown(
///     "<p><strong>Bold</strong> and <em>italic</em></p>",
///     &HtmlParseOptions::default()
/// );
/// assert_eq!(md.trim(), "**Bold** and *italic*");
/// ```
pub fn html_to_markdown(html: &str, options: &HtmlParseOptions) -> String {
    let ast = parse_html_to_ast(html, options);
    render_markdown(&ast)
}
