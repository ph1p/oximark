mod html_block;
mod leaf_blocks;
mod link_ref_def;
mod parser;

use html_block::*;
use leaf_blocks::*;
use link_ref_def::*;

use crate::ParseOptions;
use crate::ast::{Block, ListKind, TableAlignment};
use crate::entities;
use crate::html::trim_cr;
use compact_str::CompactString;
use smallvec::SmallVec;
use std::borrow::Cow;

#[cfg(feature = "html")]
use crate::inline::InlineBuffers;
use crate::inline::LinkRefMap;
#[cfg(feature = "html")]
use crate::render::render_block;

/// Parse a Markdown string and return the rendered HTML.
///
/// # Examples
///
/// ```
/// use ironmark::{render_html, ParseOptions};
///
/// let html = render_html("**bold** and *italic*", &ParseOptions::default());
/// assert!(html.contains("<strong>bold</strong>"));
/// ```
#[cfg(feature = "html")]
pub fn render_html(markdown: &str, options: &ParseOptions) -> String {
    let markdown = if options.max_input_size > 0 && markdown.len() > options.max_input_size {
        // Truncate at a valid UTF-8 boundary
        let mut end = options.max_input_size;
        while end > 0 && !markdown.is_char_boundary(end) {
            end -= 1;
        }
        &markdown[..end]
    } else {
        markdown
    };
    let mut parser = BlockParser::new(markdown, options);
    let doc = parser.parse();
    let refs = parser.ref_defs;
    let mut out = if markdown.len() <= 256 {
        String::with_capacity(markdown.len() + 32)
    } else {
        String::with_capacity(markdown.len() * 2)
    };
    let mut bufs = InlineBuffers::new();
    bufs.prepare(options);
    render_block(&doc, &refs, &mut out, options, &mut bufs);
    out
}

/// Parse a Markdown string and return the block-level AST.
///
/// This returns the raw AST without rendering to HTML, useful for
/// programmatic inspection or transformation of the document structure.
///
/// # Examples
///
/// ```
/// use ironmark::{parse_markdown, ParseOptions, Block};
///
/// let ast = parse_markdown("# Hello", &ParseOptions::default());
/// match &ast {
///     Block::Document { children } => {
///         assert_eq!(children.len(), 1);
///     }
///     _ => panic!("expected Document"),
/// }
/// ```
pub fn parse_markdown(markdown: &str, options: &ParseOptions) -> Block {
    let markdown = if options.max_input_size > 0 && markdown.len() > options.max_input_size {
        let mut end = options.max_input_size;
        while end > 0 && !markdown.is_char_boundary(end) {
            end -= 1;
        }
        &markdown[..end]
    } else {
        markdown
    };
    let mut parser = BlockParser::new(markdown, options);
    parser.parse()
}

pub fn benchmark_parse_table_row(line: &str, num_cols: usize) -> Vec<CompactString> {
    parse_table_row(line, num_cols).into_vec()
}

#[derive(Clone, Debug)]
struct Line<'a> {
    raw: &'a str,
    col_offset: usize,
    byte_offset: usize,
    partial_spaces: usize,
    cached_ns_col: usize,
    cached_ns_off: usize,
    cached_ns_byte: u8,
}

impl<'a> Line<'a> {
    fn new(raw: &'a str) -> Self {
        Self {
            raw,
            col_offset: 0,
            byte_offset: 0,
            partial_spaces: 0,
            cached_ns_col: 0,
            cached_ns_off: 0,
            cached_ns_byte: 0,
        }
    }

    fn remainder(&self) -> &'a str {
        if self.byte_offset >= self.raw.len() {
            ""
        } else {
            &self.raw[self.byte_offset..]
        }
    }

    #[inline(always)]
    fn is_blank(&mut self) -> bool {
        if self.partial_spaces > 0 {
            return false;
        }
        let (_, ns_off, ns_byte) = self.peek_nonspace_col();
        ns_byte == 0 && ns_off >= self.raw.len()
    }

    #[inline]
    fn skip_indent(&mut self, max: usize) -> usize {
        let bytes = self.raw.as_bytes();
        let mut cols = 0;
        if self.partial_spaces > 0 {
            let consume = self.partial_spaces.min(max);
            cols += consume;
            self.col_offset += consume;
            self.partial_spaces -= consume;
            if cols >= max {
                return cols;
            }
        }
        let remaining = max - cols;
        let end = (self.byte_offset + remaining).min(bytes.len());
        if end > self.byte_offset {
            let mut fast_end = self.byte_offset;
            while fast_end < end && bytes[fast_end] == b' ' {
                fast_end += 1;
            }
            let fast_count = fast_end - self.byte_offset;
            if fast_count >= remaining {
                self.byte_offset += remaining;
                self.col_offset += remaining;
                return max;
            }
            if fast_count > 0 {
                cols += fast_count;
                self.byte_offset += fast_count;
                self.col_offset += fast_count;
            }
        }
        while self.byte_offset < bytes.len() && cols < max {
            match bytes[self.byte_offset] {
                b' ' => {
                    cols += 1;
                    self.byte_offset += 1;
                    self.col_offset += 1;
                }
                b'\t' => {
                    let tab_width = 4 - (self.col_offset % 4);
                    if cols + tab_width > max {
                        let consume = max - cols;
                        self.partial_spaces = tab_width - consume;
                        self.col_offset += consume;
                        self.byte_offset += 1;
                        cols += consume;
                        break;
                    }
                    cols += tab_width;
                    self.byte_offset += 1;
                    self.col_offset += tab_width;
                }
                _ => break,
            }
        }
        cols
    }

    fn advance_columns(&mut self, n: usize) {
        let bytes = self.raw.as_bytes();
        let mut cols = 0;
        while self.byte_offset < bytes.len() && cols < n {
            match bytes[self.byte_offset] {
                b' ' => {
                    cols += 1;
                    self.byte_offset += 1;
                    self.col_offset += 1;
                }
                b'\t' => {
                    let tab_width = 4 - (self.col_offset % 4);
                    cols += tab_width;
                    self.byte_offset += 1;
                    self.col_offset += tab_width;
                }
                _ => {
                    cols += 1;
                    self.byte_offset += 1;
                    self.col_offset += 1;
                }
            }
        }
    }

    #[inline(always)]
    fn peek_nonspace_col(&mut self) -> (usize, usize, u8) {
        if self.cached_ns_off >= self.byte_offset
            && (self.cached_ns_byte != 0 || self.cached_ns_off >= self.raw.len())
        {
            return (self.cached_ns_col, self.cached_ns_off, self.cached_ns_byte);
        }
        let bytes = self.raw.as_bytes();
        let mut col = self.col_offset;
        let mut off = self.byte_offset;
        if self.partial_spaces > 0 {
            col += self.partial_spaces;
        }
        while off < bytes.len() {
            match bytes[off] {
                b' ' => {
                    col += 1;
                    off += 1;
                }
                b'\t' => {
                    col += 4 - (col % 4);
                    off += 1;
                }
                b => {
                    self.cached_ns_col = col;
                    self.cached_ns_off = off;
                    self.cached_ns_byte = b;
                    return (col, off, b);
                }
            }
        }
        self.cached_ns_col = col;
        self.cached_ns_off = off;
        self.cached_ns_byte = 0;
        (col, off, 0)
    }

    fn advance_to_nonspace(&mut self) {
        self.partial_spaces = 0;
        let (col, off, _) = self.peek_nonspace_col();
        self.col_offset = col;
        self.byte_offset = off;
    }

    fn remainder_with_partial(&self) -> Cow<'a, str> {
        if self.partial_spaces > 0 {
            static SPACES: &str = "    ";
            let rem = self.remainder();
            let mut s = String::with_capacity(self.partial_spaces + rem.len());
            s.push_str(&SPACES[..self.partial_spaces]);
            s.push_str(rem);
            Cow::Owned(s)
        } else {
            Cow::Borrowed(self.remainder())
        }
    }
}

#[derive(Clone, Debug)]
struct FencedCodeData {
    fence_char: u8,
    fence_len: usize,
    fence_indent: usize,
    info: CompactString,
}

type TableRow = SmallVec<[CompactString; 8]>;

#[derive(Clone, Debug)]
struct TableData {
    alignments: SmallVec<[TableAlignment; 8]>,
    header: TableRow,
    rows: Vec<TableRow>,
}

#[derive(Clone, Debug)]
enum OpenBlockType {
    Document,
    BlockQuote,
    ListItem {
        content_col: usize,
        started_blank: bool,
    },
    FencedCode(Box<FencedCodeData>),
    IndentedCode,
    HtmlBlock {
        end_condition: HtmlBlockEnd,
    },
    Paragraph,
    Table(Box<TableData>),
}

#[derive(Copy, Clone, Debug, PartialEq)]
enum HtmlBlockEnd {
    EndTag(&'static str),
    Comment,
    ProcessingInstruction,
    Declaration,
    Cdata,
    BlankLine,
}

#[derive(Clone, Debug)]
struct OpenBlock {
    block_type: OpenBlockType,
    content: String,
    children: SmallVec<[Block; 4]>,
    had_blank_in_item: bool,
    list_has_blank_between: bool,
    content_has_newline: bool,
    checked: Option<bool>,
    list_start: u32,
    list_kind: Option<ListKind>,
}

impl OpenBlock {
    #[inline]
    fn new(block_type: OpenBlockType) -> Self {
        Self {
            block_type,
            content: String::new(),
            children: SmallVec::new(),
            had_blank_in_item: false,
            list_has_blank_between: false,
            content_has_newline: false,
            checked: None,
            list_start: 0,
            list_kind: None,
        }
    }

    #[inline]
    fn with_content_capacity(block_type: OpenBlockType, cap: usize) -> Self {
        Self {
            content: String::with_capacity(cap),
            ..Self::new(block_type)
        }
    }

    #[inline]
    fn new_list_item(content_col: usize, started_blank: bool) -> Self {
        Self {
            block_type: OpenBlockType::ListItem {
                content_col,
                started_blank,
            },
            content: String::new(),
            children: SmallVec::new(),
            had_blank_in_item: false,
            list_has_blank_between: false,
            content_has_newline: false,
            checked: None,
            list_start: 0,
            list_kind: None,
        }
    }
}

pub(crate) struct BlockParser<'a> {
    input: &'a str,
    pub(crate) ref_defs: LinkRefMap,
    open: Vec<OpenBlock>,
    enable_tables: bool,
    enable_task_lists: bool,
    open_blockquotes: usize,
    list_indent_sum: usize,
    last_list_item_idx: Option<usize>,
    max_nesting_depth: usize,
    enable_indented_code_blocks: bool,
    permissive_atx_headers: bool,
    no_html_blocks: bool,
}

impl<'a> BlockParser<'a> {
    pub fn new(input: &'a str, options: &ParseOptions) -> Self {
        let mut doc = OpenBlock::new(OpenBlockType::Document);
        let estimated_blocks = (input.len() / 50).clamp(8, 256);
        doc.children = SmallVec::with_capacity(estimated_blocks);
        let mut open = Vec::with_capacity(16);
        open.push(doc);
        Self {
            input,
            ref_defs: LinkRefMap::default(),
            open,
            enable_tables: options.enable_tables,
            enable_task_lists: options.enable_task_lists,
            open_blockquotes: 0,
            list_indent_sum: 0,
            last_list_item_idx: None,
            max_nesting_depth: options.max_nesting_depth,
            enable_indented_code_blocks: options.enable_indented_code_blocks,
            permissive_atx_headers: options.permissive_atx_headers,
            no_html_blocks: options.no_html_blocks || options.disable_raw_html,
        }
    }

    pub fn parse(&mut self) -> Block {
        let input = self.input;
        let bytes = input.as_bytes();
        let len = bytes.len();
        let mut start = 0;
        while start < len {
            let end = memchr_newline(bytes, start);
            let raw_line = &input[start..end];
            let raw_line = trim_cr(raw_line);
            let line = Line::new(raw_line);
            self.process_line(line);

            if self.open.len() == 2
                && let OpenBlockType::FencedCode(ref fc_data) = self.open[1].block_type
                && fc_data.fence_indent == 0
            {
                let fc = fc_data.fence_char;
                let fl = fc_data.fence_len;
                start = end + 1;
                start = self.bulk_scan_fenced_code(input, bytes, start, len, fc, fl);
                continue;
            }

            start = end + 1;
        }
        while self.open.len() > 1 {
            self.close_top_block();
        }
        let doc = self.open.pop().unwrap();
        Block::Document {
            children: doc.children.into_vec(),
        }
    }

    #[inline(never)]
    fn bulk_scan_fenced_code(
        &mut self,
        input: &str,
        bytes: &[u8],
        start: usize,
        len: usize,
        fence_char: u8,
        fence_len: usize,
    ) -> usize {
        let content_start = start;
        let mut pos = start;
        let mut has_cr = false;

        while pos < len {
            let line_end = memchr_newline(bytes, pos);
            let check_end = if line_end > pos && bytes[line_end - 1] == b'\r' {
                has_cr = true;
                line_end - 1
            } else {
                line_end
            };

            if is_closing_fence(&bytes[pos..check_end], fence_char, fence_len) {
                if pos > content_start {
                    self.push_bulk_content(input, content_start, pos, has_cr);
                }
                self.close_top_block();
                return line_end + 1;
            }

            pos = line_end + 1;
        }

        if len > content_start {
            self.push_bulk_content(input, content_start, len, has_cr);
            let content = &mut self.open[1].content;
            if !content.ends_with('\n') {
                content.push('\n');
            }
        }
        pos
    }

    #[inline]
    fn push_bulk_content(&mut self, input: &str, start: usize, end: usize, has_cr: bool) {
        let content = &mut self.open[1].content;
        if !has_cr {
            // SAFETY: `start..end` comes from newline scanning over `input` and is in-bounds.
            content.push_str(unsafe { input.get_unchecked(start..end) });
        } else {
            // SAFETY: same bounds guarantee as above.
            let s = unsafe { input.get_unchecked(start..end) };
            content.reserve(s.len());
            for chunk in s.split('\r') {
                content.push_str(chunk);
            }
        }
    }

    fn mark_blank_on_list_items(&mut self) {
        if let Some(idx) = self.last_list_item_idx {
            if self.open_blockquotes == 0 {
                self.open[idx].had_blank_in_item = true;
                return;
            }
            // A blockquote exists somewhere; check if one sits between the list
            // item and the tip (if so, the blank belongs to the blockquote, not
            // the list item).
            let len = self.open.len();
            for i in (idx + 1)..len {
                if matches!(self.open[i].block_type, OpenBlockType::BlockQuote) {
                    return;
                }
            }
            self.open[idx].had_blank_in_item = true;
        }
    }

    #[inline]
    fn close_top_block(&mut self) {
        let block = self.open.pop().unwrap();
        match &block.block_type {
            OpenBlockType::BlockQuote => {
                self.open_blockquotes -= 1;
            }
            OpenBlockType::ListItem { content_col, .. } => {
                self.list_indent_sum -= content_col;
                // The item we just popped was last_list_item_idx; the next one
                // must be strictly below that index, so search only that prefix.
                self.last_list_item_idx = match self.last_list_item_idx {
                    Some(idx) if idx > 0 => self.open[..idx]
                        .iter()
                        .rposition(|b| matches!(b.block_type, OpenBlockType::ListItem { .. })),
                    _ => None,
                };
            }
            _ => {}
        }
        let finalized = self.finalize_block(block);
        if let Some(block) = finalized {
            let parent = self.open.last_mut().unwrap();
            parent.children.push(block);
        }
    }
}
