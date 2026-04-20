mod slug;

pub use slug::benchmark_heading_slug;
pub(crate) use slug::heading_slug_into;

use crate::ParseOptions;
use crate::ast::{Block, ListKind, TableAlignment};
use crate::html::{
    collapse_and_escape_into, encode_url_escaped_into, escape_html_into, gfm_tag_is_filtered,
};
use crate::inline::{InlineBuffers, LinkRefMap, parse_inline_pass};

#[inline(always)]
fn emit_checkbox(out: &mut String, checked: Option<bool>) {
    match checked {
        Some(true) => out.push_str("<input type=\"checkbox\" checked=\"\" disabled=\"\" /> "),
        Some(false) => out.push_str("<input type=\"checkbox\" disabled=\"\" /> "),
        None => {}
    }
}

enum Work<'a> {
    Block(&'a Block),
    TightListItem(&'a Block),
    TightBlock(&'a Block),
    CloseTag(&'static str),
}

pub(crate) fn render_block(
    block: &Block,
    refs: &LinkRefMap,
    out: &mut String,
    opts: &ParseOptions,
    bufs: &mut InlineBuffers,
) {
    if let Block::Document { children } = block
        && let [child] = children.as_slice()
    {
        render_single_child_doc(child, refs, out, opts, bufs);
        return;
    }

    let mut stack: Vec<Work<'_>> = Vec::with_capacity(32);
    stack.push(Work::Block(block));

    while let Some(work) = stack.pop() {
        match work {
            Work::CloseTag(tag) => out.push_str(tag),
            Work::TightListItem(block) => {
                render_tight_list_item(block, refs, out, opts, bufs, &mut stack);
            }
            Work::TightBlock(block) => {
                if let Block::Paragraph { raw } = block {
                    push_inline_or_plain(out, raw, refs, opts, bufs);
                } else {
                    render_one(block, refs, out, opts, bufs, &mut stack);
                }
            }
            Work::Block(block) => {
                render_one(block, refs, out, opts, bufs, &mut stack);
            }
        }
    }
}

#[inline]
fn render_single_child_doc(
    child: &Block,
    refs: &LinkRefMap,
    out: &mut String,
    opts: &ParseOptions,
    bufs: &mut InlineBuffers,
) {
    match child {
        Block::Paragraph { raw } => {
            out.push_str("<p>");
            parse_inline_pass(out, raw, refs, opts, bufs);
            out.push_str("</p>\n");
        }
        _ => {
            let mut stack = Vec::with_capacity(8);
            stack.push(Work::Block(child));
            while let Some(work) = stack.pop() {
                match work {
                    Work::CloseTag(tag) => out.push_str(tag),
                    Work::TightListItem(block) => {
                        render_tight_list_item(block, refs, out, opts, bufs, &mut stack);
                    }
                    Work::TightBlock(block) => {
                        if let Block::Paragraph { raw } = block {
                            push_inline_or_plain(out, raw, refs, opts, bufs);
                        } else {
                            render_one(block, refs, out, opts, bufs, &mut stack);
                        }
                    }
                    Work::Block(block) => render_one(block, refs, out, opts, bufs, &mut stack),
                }
            }
        }
    }
}

fn list_close_tag(kind: &ListKind) -> &'static str {
    match kind {
        ListKind::Bullet(_) => "</ul>\n",
        ListKind::Ordered(_) => "</ol>\n",
    }
}

#[inline(always)]
fn emit_list_open(out: &mut String, kind: &ListKind, start: u32) {
    match kind {
        ListKind::Bullet(_) => out.push_str("<ul>\n"),
        ListKind::Ordered(_) => {
            if start == 1 {
                out.push_str("<ol>\n");
            } else {
                use std::fmt::Write;
                out.push_str("<ol start=\"");
                let _ = write!(out, "{}", start);
                out.push_str("\">\n");
            }
        }
    }
}

#[inline]
fn render_one<'a>(
    block: &'a Block,
    refs: &LinkRefMap,
    out: &mut String,
    opts: &ParseOptions,
    bufs: &mut InlineBuffers,
    stack: &mut Vec<Work<'a>>,
) {
    match block {
        Block::Document { children } => {
            for child in children.iter().rev() {
                stack.push(Work::Block(child));
            }
        }
        Block::ThematicBreak => out.push_str("<hr />\n"),
        Block::Heading { level, raw } => {
            static TAGS: [&str; 7] = ["", "h1", "h2", "h3", "h4", "h5", "h6"];
            let l = *level as usize;
            let tag = TAGS[l];
            out.push('<');
            out.push_str(tag);
            let mut slug = std::mem::take(&mut bufs.scratch);
            let use_slug = opts.enable_heading_ids || opts.enable_heading_anchors;
            if use_slug {
                heading_slug_into(&mut slug, raw);
                if !slug.is_empty() {
                    out.push_str(" id=\"");
                    escape_html_into(out, &slug);
                    out.push('"');
                }
            }
            out.push('>');
            parse_inline_pass(out, raw, refs, opts, bufs);
            if opts.enable_heading_anchors && !slug.is_empty() {
                out.push_str(" <a class=\"anchor\" href=\"#");
                encode_url_escaped_into(out, &slug);
                out.push_str("\">¶</a>");
            }
            out.push_str("</");
            out.push_str(tag);
            out.push_str(">\n");
            bufs.scratch = slug;
        }
        Block::Paragraph { raw } => {
            out.push_str("<p>");
            parse_inline_pass(out, raw, refs, opts, bufs);
            out.push_str("</p>\n");
        }
        Block::CodeBlock { info, literal } => {
            if info.is_empty() {
                out.push_str("<pre><code>");
            } else {
                let lang = match memchr::memchr3(b' ', b'\t', b'\n', info.as_bytes()) {
                    Some(0) => "",
                    // SAFETY: `pos` is returned by memchr and is within `info`.
                    Some(pos) => unsafe { info.get_unchecked(..pos) },
                    None => info,
                };
                if lang.is_empty() {
                    out.push_str("<pre><code>");
                } else {
                    out.push_str("<pre><code class=\"language-");
                    escape_html_into(out, lang);
                    out.push_str("\">");
                }
            }
            escape_html_into(out, literal);
            out.push_str("</code></pre>\n");
        }
        Block::HtmlBlock { literal } => {
            let escape_it = opts.disable_raw_html
                || opts.no_html_blocks
                || (opts.tag_filter && gfm_tag_is_filtered(literal));
            if escape_it {
                escape_html_into(out, literal);
            } else {
                out.push_str(literal);
            }
            if !literal.ends_with('\n') {
                out.push('\n');
            }
        }
        Block::BlockQuote { children } => {
            out.push_str("<blockquote>\n");
            stack.push(Work::CloseTag("</blockquote>\n"));
            for child in children.iter().rev() {
                stack.push(Work::Block(child));
            }
        }
        Block::List {
            kind,
            start,
            tight,
            children,
        } => {
            if *tight && children.len() == 1 {
                render_nested_tight_list(
                    kind,
                    *start,
                    children,
                    InlineCtx { refs, opts },
                    out,
                    bufs,
                    stack,
                );
                return;
            }
            emit_list_open(out, kind, *start);
            stack.push(Work::CloseTag(list_close_tag(kind)));
            if *tight {
                for item in children.iter().rev() {
                    stack.push(Work::TightListItem(item));
                }
            } else {
                for item in children.iter().rev() {
                    stack.push(Work::Block(item));
                }
            }
        }
        Block::ListItem { children, checked } => {
            out.push_str("<li>");
            emit_checkbox(out, *checked);
            if !children.is_empty() {
                out.push('\n');
                stack.push(Work::CloseTag("</li>\n"));
                for child in children.iter().rev() {
                    stack.push(Work::Block(child));
                }
            } else {
                out.push_str("</li>\n");
            }
        }
        Block::Table(td) => {
            let alignments = &td.alignments;
            let header = &td.header;
            let num_cols = td.num_cols;
            let all_none = alignments.iter().all(|a| *a == TableAlignment::None);
            out.push_str("<table>\n<thead>\n<tr>\n");
            for (i, cell) in header.iter().enumerate() {
                let align = if all_none {
                    TableAlignment::None
                } else {
                    alignments.get(i).copied().unwrap_or(TableAlignment::None)
                };
                render_table_cell(out, cell.as_str(), "th", align, refs, opts, bufs);
            }
            out.push_str("</tr>\n</thead>\n");
            let num_rows = td.rows.len().checked_div(num_cols).unwrap_or(0);
            if num_rows > 0 {
                out.push_str("<tbody>\n");
                if all_none {
                    for row in td.rows.chunks_exact(num_cols) {
                        out.push_str("<tr>\n");
                        for cell in row {
                            out.push_str("<td>");
                            push_inline_or_plain(out, cell.as_str(), refs, opts, bufs);
                            out.push_str("</td>\n");
                        }
                        out.push_str("</tr>\n");
                    }
                } else {
                    for row in td.rows.chunks_exact(num_cols) {
                        out.push_str("<tr>\n");
                        for (c, cell) in row.iter().enumerate() {
                            let align = alignments.get(c).copied().unwrap_or(TableAlignment::None);
                            render_table_cell(out, cell.as_str(), "td", align, refs, opts, bufs);
                        }
                        out.push_str("</tr>\n");
                    }
                }
                out.push_str("</tbody>\n");
            }
            out.push_str("</table>\n");
        }
    }
}

/// Returns `true` if `s` needs no inline parsing or HTML escaping.
/// Uses the pre-built scan_table (same table as `parse_inline_pass`) so we
/// do a single O(n) pass instead of multiple memchr SIMD scans.
#[inline(always)]
fn is_trivially_plain(s: &str, scan_table: &[u8; 256]) -> bool {
    s.as_bytes()
        .iter()
        .all(|&b| scan_table[b as usize] == 0 && b >= b' ')
}

/// For trivially plain text, push directly; otherwise run the full inline pass.
#[inline(always)]
fn push_inline_or_plain(
    out: &mut String,
    raw: &str,
    refs: &LinkRefMap,
    opts: &ParseOptions,
    bufs: &mut InlineBuffers,
) {
    if is_trivially_plain(raw, &bufs.scan_table) {
        if opts.collapse_whitespace {
            collapse_and_escape_into(out, raw);
        } else {
            out.push_str(raw);
        }
    } else {
        parse_inline_pass(out, raw, refs, opts, bufs);
    }
}

#[derive(Copy, Clone)]
struct InlineCtx<'a> {
    refs: &'a LinkRefMap,
    opts: &'a ParseOptions,
}

#[inline(never)]
fn render_nested_tight_list<'a>(
    kind: &ListKind,
    start: u32,
    children: &'a [Block],
    inline: InlineCtx<'_>,
    out: &mut String,
    bufs: &mut InlineBuffers,
    stack: &mut Vec<Work<'a>>,
) {
    const MAX_DEPTH: usize = 64;
    let mut close_tags: [&'static str; MAX_DEPTH] = [""; MAX_DEPTH];
    let mut depth: usize = 0;

    let mut cur_kind = kind;
    let mut cur_start = start;
    let mut cur_children: &'a [Block] = children;

    loop {
        let Block::ListItem {
            children: item_children,
            checked,
        } = &cur_children[0]
        else {
            emit_list_open(out, cur_kind, cur_start);
            stack.push(Work::CloseTag(list_close_tag(cur_kind)));
            stack.push(Work::Block(&cur_children[0]));
            break;
        };

        match cur_kind {
            ListKind::Bullet(_) => out.push_str("<ul>\n<li>"),
            ListKind::Ordered(_) => {
                emit_list_open(out, cur_kind, cur_start);
                out.push_str("<li>");
            }
        }
        emit_checkbox(out, *checked);

        if item_children.len() == 2
            && depth < MAX_DEPTH
            && let (
                Block::Paragraph { raw },
                Block::List {
                    kind: inner_kind,
                    start: inner_start,
                    tight: true,
                    children: inner_children,
                },
            ) = (&item_children[0], &item_children[1])
            && inner_children.len() == 1
        {
            push_inline_or_plain(out, raw, inline.refs, inline.opts, bufs);
            out.push('\n');
            close_tags[depth] = list_close_tag(cur_kind);
            depth += 1;
            cur_kind = inner_kind;
            cur_start = *inner_start;
            cur_children = inner_children;
            continue;
        }

        if item_children.len() == 1
            && let Block::Paragraph { raw } = &item_children[0]
        {
            push_inline_or_plain(out, raw, inline.refs, inline.opts, bufs);
            // Reserve for unwind: "</li>\n" (6) + close tag (~6) per level
            let total_close_bytes = (depth + 1) * 12;
            out.reserve(total_close_bytes);
            debug_assert!(
                total_close_bytes >= 12,
                "close bytes must cover at least one level"
            );
            // SAFETY: reserved enough capacity, all bytes are ASCII.
            // Each nesting level writes at most "</li>\n" (6) + "</ul>\n"/"</ol>\n" (6) = 12 bytes.
            unsafe {
                let buf = out.as_mut_vec();
                debug_assert!(buf.capacity() - buf.len() >= total_close_bytes);
                let mut ptr = buf.as_mut_ptr().add(buf.len());

                macro_rules! write_bytes {
                    ($s:expr) => {
                        std::ptr::copy_nonoverlapping($s.as_ptr(), ptr, $s.len());
                        ptr = ptr.add($s.len());
                    };
                }

                write_bytes!(b"</li>\n");
                write_bytes!(list_close_tag(cur_kind).as_bytes());
                let mut i = depth;
                while i > 0 {
                    i -= 1;
                    write_bytes!(b"</li>\n");
                    write_bytes!(close_tags[i].as_bytes());
                }
                buf.set_len(ptr.offset_from(buf.as_ptr()) as usize);
            }
            return;
        }

        {
            let mut i = 0;
            while i < depth {
                stack.push(Work::CloseTag(close_tags[i]));
                stack.push(Work::CloseTag("</li>\n"));
                i += 1;
            }
        }
        stack.push(Work::CloseTag(list_close_tag(cur_kind)));
        stack.push(Work::CloseTag("</li>\n"));

        let mut prev_was_para = false;
        for (idx, child) in item_children.iter().enumerate() {
            match child {
                Block::Paragraph { raw } => {
                    push_inline_or_plain(out, raw, inline.refs, inline.opts, bufs);
                    prev_was_para = true;
                }
                _ => {
                    if prev_was_para || idx == 0 {
                        out.push('\n');
                    }
                    for remaining in item_children[idx..].iter().rev() {
                        stack.push(Work::TightBlock(remaining));
                    }
                    return;
                }
            }
        }
        return;
    }

    out.reserve(depth * 12);
    let mut i = depth;
    while i > 0 {
        i -= 1;
        out.push_str("</li>\n");
        out.push_str(close_tags[i]);
    }
}

#[inline]
fn render_tight_list_item<'a>(
    block: &'a Block,
    refs: &LinkRefMap,
    out: &mut String,
    opts: &ParseOptions,
    bufs: &mut InlineBuffers,
    stack: &mut Vec<Work<'a>>,
) {
    let Block::ListItem { children, checked } = block else {
        render_one(block, refs, out, opts, bufs, stack);
        return;
    };

    out.push_str("<li>");
    emit_checkbox(out, *checked);

    if children.len() == 1
        && let Block::Paragraph { raw } = &children[0]
    {
        push_inline_or_plain(out, raw, refs, opts, bufs);
        out.push_str("</li>\n");
        return;
    }

    stack.push(Work::CloseTag("</li>\n"));
    let mut prev_was_para = false;
    for (idx, child) in children.iter().enumerate() {
        match child {
            Block::Paragraph { raw } => {
                push_inline_or_plain(out, raw, refs, opts, bufs);
                prev_was_para = true;
            }
            _ => {
                if prev_was_para || idx == 0 {
                    out.push('\n');
                }
                for remaining in children[idx..].iter().rev() {
                    stack.push(Work::TightBlock(remaining));
                }
                return;
            }
        }
    }
}

#[inline]
fn render_table_cell(
    out: &mut String,
    content: &str,
    tag: &str,
    align: TableAlignment,
    refs: &LinkRefMap,
    opts: &ParseOptions,
    bufs: &mut InlineBuffers,
) {
    let (open, close) = match (tag, align) {
        ("th", TableAlignment::None) => ("<th>", "</th>\n"),
        ("td", TableAlignment::None) => ("<td>", "</td>\n"),
        ("th", TableAlignment::Left) => ("<th style=\"text-align: left\">", "</th>\n"),
        ("th", TableAlignment::Right) => ("<th style=\"text-align: right\">", "</th>\n"),
        ("th", TableAlignment::Center) => ("<th style=\"text-align: center\">", "</th>\n"),
        ("td", TableAlignment::Left) => ("<td style=\"text-align: left\">", "</td>\n"),
        ("td", TableAlignment::Right) => ("<td style=\"text-align: right\">", "</td>\n"),
        ("td", TableAlignment::Center) => ("<td style=\"text-align: center\">", "</td>\n"),
        _ => {
            out.push('<');
            out.push_str(tag);
            match align {
                TableAlignment::Left => out.push_str(" style=\"text-align: left\""),
                TableAlignment::Right => out.push_str(" style=\"text-align: right\""),
                TableAlignment::Center => out.push_str(" style=\"text-align: center\""),
                TableAlignment::None => {}
            }
            out.push('>');
            push_inline_or_plain(out, content, refs, opts, bufs);
            out.push_str("</");
            out.push_str(tag);
            out.push_str(">\n");
            return;
        }
    };
    out.push_str(open);
    push_inline_or_plain(out, content, refs, opts, bufs);
    out.push_str(close);
}
