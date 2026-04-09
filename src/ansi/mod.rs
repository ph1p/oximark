//! ANSI terminal renderer for Markdown.
//!
//! Renders a parsed Markdown AST as coloured, formatted terminal output using
//! ANSI escape codes. Suitable for `cat`-like tools or CLI help text.
//!
//! # Entry points
//!
//! - [`render_ansi`] — render with optional [`AnsiOptions`] (defaults used when `None`).
//!
//! # Module layout
//!
//! | Module | Contents |
//! |---|---|
//! | `constants` | ANSI 256-colour escape constants |
//! | `inline` | HTML-to-ANSI tag translator, inline renderer |
//! | `renderer` | [`AnsiRenderer`] block-level renderer |
//! | `wrap` | [`wrap_ansi`], [`visible_len`], [`expand_tabs`] helpers |
//!
//! # Examples
//!
//! ```
//! use ironmark::{render_ansi, AnsiOptions, ParseOptions};
//!
//! // Simple usage — defaults (width 80, colour on)
//! let out = render_ansi("# Hello\n\n**bold** and *italic*", &ParseOptions::default(), None);
//! assert!(out.contains("Hello"));
//!
//! // With custom width and disabled colour (e.g. for piping)
//! let opts = AnsiOptions { width: 80, color: false, ..AnsiOptions::default() };
//! let plain = render_ansi("# Hello", &ParseOptions::default(), Some(&opts));
//! assert!(!plain.contains('\x1b'));
//! ```

mod constants;
mod inline;
mod renderer;
mod wrap;

use crate::ParseOptions;
use crate::inline::InlineBuffers;
use renderer::AnsiRenderer;

// ── Options ───────────────────────────────────────────────────────────────────

/// Display options for the ANSI terminal renderer.
///
/// Pass to [`render_ansi`] to control how the output looks.
///
/// # Defaults
///
/// ```
/// use ironmark::AnsiOptions;
/// let opts = AnsiOptions::default();
/// assert_eq!(opts.width, 80);
/// assert!(opts.color);
/// assert!(!opts.line_numbers);
/// ```
#[derive(Clone, Debug)]
pub struct AnsiOptions {
    /// Terminal column width used for:
    /// - paragraph word-wrapping
    /// - heading underline length
    /// - thematic break length
    ///
    /// Set to `0` to disable all width-dependent formatting.
    /// Default: `80`.
    pub width: usize,

    /// When `false`, all ANSI escape codes are omitted and the output is plain
    /// text. Useful for piping to files or non-colour terminals.
    /// Default: `true`.
    pub color: bool,

    /// Show line numbers in fenced code blocks.
    ///
    /// Line numbers are right-aligned to the total line count, rendered in
    /// a dim border colour with a `│` separator before the code content.
    /// Default: `false`.
    pub line_numbers: bool,
}

impl Default for AnsiOptions {
    fn default() -> Self {
        Self {
            width: 80,
            color: true,
            line_numbers: false,
        }
    }
}

// ── Public entry points ───────────────────────────────────────────────────────

/// Parse `markdown` and render it as ANSI-coloured terminal output.
///
/// Pass `Some(&AnsiOptions { .. })` to control terminal width, colour, or line
/// numbers. Pass `None` to use the defaults (width 80, colour enabled, no line
/// numbers).
///
/// # Examples
///
/// ```
/// use ironmark::{render_ansi, AnsiOptions, ParseOptions};
///
/// // Defaults
/// let out = render_ansi("# Hello\n\n**bold**", &ParseOptions::default(), None);
/// assert!(out.contains("Hello"));
/// assert!(out.contains('\x1b'));
///
/// // Plain text (no ANSI escapes)
/// let opts = AnsiOptions { color: false, ..AnsiOptions::default() };
/// let plain = render_ansi("# Hello\n\n> quote", &ParseOptions::default(), Some(&opts));
/// assert!(!plain.contains('\x1b'));
///
/// // Line numbers in code blocks
/// let opts = AnsiOptions { line_numbers: true, ..AnsiOptions::default() };
/// let out = render_ansi("```rust\nfn main() {}\n```", &ParseOptions::default(), Some(&opts));
/// assert!(out.contains('1'));
/// ```
pub fn render_ansi(markdown: &str, options: &ParseOptions, aopts: Option<&AnsiOptions>) -> String {
    let default_aopts;
    let aopts = match aopts {
        Some(a) => a,
        None => {
            default_aopts = AnsiOptions::default();
            &default_aopts
        }
    };
    let markdown = if options.max_input_size > 0 && markdown.len() > options.max_input_size {
        let mut end = options.max_input_size;
        while end > 0 && !markdown.is_char_boundary(end) {
            end -= 1;
        }
        &markdown[..end]
    } else {
        markdown
    };

    let mut parser = crate::block::BlockParser::new(markdown, options);
    let doc = parser.parse();
    let refs = parser.ref_defs;
    let mut out = String::with_capacity(markdown.len() * 2);
    let mut bufs = InlineBuffers::new();

    let mut renderer = AnsiRenderer {
        refs: &refs,
        opts: options,
        aopts,
        bufs: &mut bufs,
        out: &mut out,
        list_depth: 0,
        list_counters: Vec::new(),
        prev_was_heading: false,
    };
    renderer.render_block(&doc);
    out
}
