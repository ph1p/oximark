#![deny(clippy::undocumented_unsafe_blocks)]

//! # ironmark
//!
//! A fast, CommonMark 0.31.2 compliant Markdown-to-HTML parser with extensions.
//!
//! ## Usage
//!
//! ```
//! use ironmark::{parse, ParseOptions};
//!
//! // With defaults (all extensions enabled)
//! let html = parse("# Hello, **world**!", &ParseOptions::default());
//!
//! // Disable specific extensions
//! let opts = ParseOptions {
//!     enable_strikethrough: false,
//!     enable_tables: false,
//!     ..Default::default()
//! };
//! let html = parse("Plain CommonMark only.", &opts);
//! ```
//!
//! ## Security
//!
//! When rendering **untrusted** input, enable these options:
//!
//! ```
//! use ironmark::{parse, ParseOptions};
//!
//! let opts = ParseOptions {
//!     disable_raw_html: true,  // escape HTML blocks & inline HTML
//!     max_input_size: 1_000_000, // limit input to 1 MB
//!     ..Default::default()
//! };
//! let html = parse("<script>alert(1)</script>", &opts);
//! assert!(!html.contains("<script>"));
//! ```
//!
//! Additionally, `javascript:`, `vbscript:`, and `data:` URIs (except `data:image/…`)
//! are **always** stripped from link and image destinations regardless of options.
//!
//! ## Extensions
//!
//! All extensions are enabled by default via [`ParseOptions`]:
//!
//! | Syntax | HTML | Option |
//! |---|---|---|
//! | `~~text~~` | `<del>` | `enable_strikethrough` |
//! | `==text==` | `<mark>` | `enable_highlight` |
//! | `++text++` | `<u>` | `enable_underline` |
//! | `\| table \|` | `<table>` | `enable_tables` |
//! | `- [x] task` | checkbox | `enable_task_lists` |
//! | bare URLs | `<a>` | `enable_autolink` |
//! | newlines | `<br />` | `hard_breaks` |

pub mod ast;
mod block;
mod entities;
pub mod ffi;
mod html;
mod inline;
mod render;

pub use ast::{Block, ListKind, TableAlignment, TableData};
pub use block::{parse, parse_to_ast};

#[inline(always)]
pub(crate) fn is_ascii_punctuation(b: u8) -> bool {
    matches!(b, b'!'..=b'/' | b':'..=b'@' | b'['..=b'`' | b'{'..=b'~')
}

#[inline(always)]
pub(crate) fn utf8_char_len(first: u8) -> usize {
    if first < 0x80 {
        1
    } else if first < 0xE0 {
        2
    } else if first < 0xF0 {
        3
    } else {
        4
    }
}

/// Options for customizing Markdown parsing behavior.
pub struct ParseOptions {
    /// When `true`, every newline inside a paragraph becomes a hard line break (`<br />`),
    /// similar to GitHub Flavored Markdown. Default: `true`.
    pub hard_breaks: bool,
    /// Enable `==highlight==` syntax → `<mark>`. Default: `true`.
    pub enable_highlight: bool,
    /// Enable `~~strikethrough~~` syntax → `<del>`. Default: `true`.
    pub enable_strikethrough: bool,
    /// Enable `++underline++` syntax → `<u>`. Default: `true`.
    pub enable_underline: bool,
    /// Enable pipe table syntax. Default: `true`.
    pub enable_tables: bool,
    /// Automatically detect bare URLs (`https://...`) and emails (`user@example.com`)
    /// and wrap them in `<a>` tags. Default: `true`.
    pub enable_autolink: bool,
    /// Enable GitHub-style task lists (`- [ ] unchecked`, `- [x] checked`)
    /// in list items. Default: `true`.
    pub enable_task_lists: bool,
    /// When `true`, raw HTML blocks and inline HTML are escaped instead of passed
    /// through verbatim. This prevents XSS when rendering untrusted markdown.
    /// Default: `false`.
    pub disable_raw_html: bool,
    /// Maximum nesting depth for block-level containers (blockquotes, list items).
    /// Once exceeded, further nesting is treated as paragraph text.
    /// Default: `128`.
    pub max_nesting_depth: usize,
    /// Maximum input size in bytes. Inputs exceeding this limit are truncated.
    /// `0` means no limit. Default: `0`.
    pub max_input_size: usize,
}

impl Default for ParseOptions {
    fn default() -> Self {
        Self {
            hard_breaks: true,
            enable_highlight: true,
            enable_strikethrough: true,
            enable_underline: true,
            enable_tables: true,
            enable_autolink: true,
            enable_task_lists: true,
            disable_raw_html: false,
            max_nesting_depth: 128,
            max_input_size: 0,
        }
    }
}
