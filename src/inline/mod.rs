mod links;
mod render;
mod scanner;

use crate::ParseOptions;
use crate::entities;
use crate::html::{encode_url_escaped_into, escape_html_into, is_dangerous_url};
use rustc_hash::FxHashMap;
use std::borrow::Cow;
use std::rc::Rc;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct LinkReference {
    pub href: Rc<str>,
    pub title: Option<Rc<str>>,
}

pub(crate) type LinkRefMap = FxHashMap<String, LinkReference>;

#[inline]
pub(crate) fn normalize_reference_label(label: &str) -> Cow<'_, str> {
    let trimmed = label.trim();
    let bytes = trimmed.as_bytes();

    {
        let mut simple = true;
        let mut prev_space = false;
        for &b in bytes {
            if b.is_ascii_uppercase() {
                simple = false;
                break;
            }
            if b == b' ' {
                if prev_space {
                    simple = false;
                    break;
                }
                prev_space = true;
            } else if b == b'\t' || b == b'\n' || b == b'\r' || b >= 0x80 {
                simple = false;
                break;
            } else {
                prev_space = false;
            }
        }
        if simple {
            return Cow::Borrowed(trimmed);
        }
    }

    let mut out = String::with_capacity(trimmed.len());
    let mut in_space = false;
    let mut i = 0;
    while i < bytes.len() {
        let b = bytes[i];
        if b < 0x80 {
            if b == b' ' || b == b'\t' || b == b'\n' || b == b'\r' {
                if !in_space {
                    out.push(' ');
                    in_space = true;
                }
                i += 1;
            } else {
                out.push(if b.is_ascii_uppercase() {
                    (b + 32) as char
                } else {
                    b as char
                });
                in_space = false;
                i += 1;
            }
        } else {
            // SAFETY: We've verified bytes[i] >= 0x80, indicating a multi-byte UTF-8 sequence.
            // `trimmed` is valid UTF-8, so we can safely slice from a char boundary at `i`.
            let ch = unsafe { trimmed.get_unchecked(i..) };
            let c = ch.chars().next().unwrap();
            let clen = c.len_utf8();
            if c.is_whitespace() {
                if !in_space {
                    out.push(' ');
                    in_space = true;
                }
            } else {
                match c {
                    'ß' | 'ẞ' => out.push_str("ss"),
                    _ => {
                        for lc in c.to_lowercase() {
                            out.push(lc);
                        }
                    }
                }
                in_space = false;
            }
            i += clen;
        }
    }
    Cow::Owned(out)
}

const SPECIAL_ANY: u8 = 1;
const SPECIAL_COMPLEX: u8 = 2;
const SCAN_FULL: u8 = 1;
const SCAN_IS_BACKTICK: u8 = 2;
const SCAN_EMPH: u8 = 4;
const SCAN_BREAK: u8 = 8;
const SCAN_ESCAPE: u8 = 16;

static SPECIAL: [u8; 256] = {
    let mut t = [0u8; 256];
    t[b'*' as usize] = SPECIAL_ANY;
    t[b'_' as usize] = SPECIAL_ANY;
    let both = SPECIAL_ANY | SPECIAL_COMPLEX;
    t[b'\\' as usize] = both;
    t[b'`' as usize] = both;
    t[b'!' as usize] = both;
    t[b'[' as usize] = both;
    t[b']' as usize] = both;
    t[b'<' as usize] = both;
    t[b'&' as usize] = both;
    t[b'\n' as usize] = both;
    t[b'~' as usize] = both;
    t[b'=' as usize] = both;
    t[b'+' as usize] = both;
    t[b':' as usize] = both;
    t[b'@' as usize] = both;
    t[b'$' as usize] = both;
    t
};

#[inline]
pub(crate) fn parse_inline_pass(
    out: &mut String,
    raw: &str,
    refs: &LinkRefMap,
    opts: &ParseOptions,
    bufs: &mut InlineBuffers,
) {
    let bytes = raw.as_bytes();
    let scan_table = &bufs.scan_table;

    let mut has_emphasis = false;
    let mut has_breaks = false;
    let mut needs_html_escape = false;
    let mut requires_full_inline = false;
    let mut backtick_hint = 0u8;
    let mut rescan_from = bytes.len();
    let mut full_only_backticks = true;

    // Fast scan: process 8 bytes at a time, ORing flags together. If no byte
    // triggers SCAN_FULL|SCAN_BREAK|SCAN_EMPH|SCAN_ESCAPE in a chunk we skip it cheaply.
    let mut i = 0;
    // Process 8 bytes per iteration: OR all flags; if any FULL bit set, fall to per-byte.
    const CHUNK: usize = 8;
    while i + CHUNK <= bytes.len() {
        let chunk = &bytes[i..i + CHUNK];
        // OR flags for all 8 bytes
        let flags = scan_table[chunk[0] as usize]
            | scan_table[chunk[1] as usize]
            | scan_table[chunk[2] as usize]
            | scan_table[chunk[3] as usize]
            | scan_table[chunk[4] as usize]
            | scan_table[chunk[5] as usize]
            | scan_table[chunk[6] as usize]
            | scan_table[chunk[7] as usize];
        if flags == 0 {
            i += CHUNK;
            continue;
        }
        // Something interesting in this chunk — process per byte.
        for (offset, &b) in chunk.iter().enumerate() {
            let f = scan_table[b as usize];
            if f & SCAN_FULL != 0 {
                requires_full_inline = true;
                if f & SCAN_IS_BACKTICK == 0 {
                    full_only_backticks = false;
                }
                if f & SCAN_IS_BACKTICK != 0 && backtick_hint < u8::MAX {
                    backtick_hint = backtick_hint.saturating_add(1);
                }
                rescan_from = i + offset + 1;
                i = bytes.len(); // signal done
                break;
            }
            if f & SCAN_EMPH != 0 {
                has_emphasis = true;
            }
            if f & SCAN_BREAK != 0 {
                has_breaks = true;
            }
            if f & SCAN_ESCAPE != 0 {
                needs_html_escape = true;
            }
        }
        if requires_full_inline {
            break;
        }
        i += CHUNK;
    }
    // Handle remaining bytes (tail < 8).
    if !requires_full_inline {
        while i < bytes.len() {
            let f = scan_table[bytes[i] as usize];
            if f & SCAN_FULL != 0 {
                requires_full_inline = true;
                if f & SCAN_IS_BACKTICK == 0 {
                    full_only_backticks = false;
                }
                if f & SCAN_IS_BACKTICK != 0 && backtick_hint < u8::MAX {
                    backtick_hint = backtick_hint.saturating_add(1);
                }
                rescan_from = i + 1;
                break;
            }
            if f & SCAN_EMPH != 0 {
                has_emphasis = true;
            }
            if f & SCAN_BREAK != 0 {
                has_breaks = true;
            }
            if f & SCAN_ESCAPE != 0 {
                needs_html_escape = true;
            }
            i += 1;
        }
    }

    if !requires_full_inline {
        if has_breaks {
            emit_breaks_and_emphasis(out, raw, bytes, opts, &mut bufs.em_delims);
        } else if has_emphasis {
            emit_emphasis_only(out, raw, bytes, &mut bufs.em_delims);
        } else if needs_html_escape || opts.collapse_whitespace {
            if opts.collapse_whitespace {
                crate::html::collapse_and_escape_into(out, raw);
            } else {
                escape_html_into(out, raw);
            }
        } else {
            out.push_str(raw);
        }
        return;
    }

    if try_emit_common_inline(&mut bufs.scratch, raw, opts) {
        out.push_str(&bufs.scratch);
        bufs.scratch.clear();
        return;
    }

    if backtick_hint != 0 && backtick_hint < 3 {
        backtick_hint = count_backtick_hint_from(bytes, rescan_from, backtick_hint);
    }

    if full_only_backticks && backtick_hint != 0 && !has_matching_backtick_runs(bytes) {
        if has_breaks {
            emit_breaks_and_emphasis(out, raw, bytes, opts, &mut bufs.em_delims);
        } else if has_emphasis {
            emit_emphasis_only(out, raw, bytes, &mut bufs.em_delims);
        } else if needs_html_escape || opts.collapse_whitespace {
            if opts.collapse_whitespace {
                crate::html::collapse_and_escape_into(out, raw);
            } else {
                escape_html_into(out, raw);
            }
        } else {
            out.push_str(raw);
        }
        return;
    }

    out.reserve(raw.len());
    if bufs.items.capacity() == 0 {
        bufs.items.reserve(raw.len() / 20 + 4);
    }
    let mut p = InlineScanner::new_with_bufs(raw, refs, opts, bufs, backtick_hint);
    p.scan_all();
    if !p.delims.is_empty() {
        p.process_emphasis(0);
    }
    p.render_to_html(out, opts);
}

#[cfg(feature = "html")]
pub fn benchmark_parse_inline(raw: &str, opts: &ParseOptions) -> String {
    let mut out = String::with_capacity(raw.len() + 16);
    let mut bufs = InlineBuffers::new();
    bufs.prepare(opts);
    parse_inline_pass(&mut out, raw, &LinkRefMap::default(), opts, &mut bufs);
    out
}

#[inline]
fn count_backtick_hint_from(bytes: &[u8], mut start: usize, mut count: u8) -> u8 {
    while start < bytes.len() && count < 3 {
        let Some(off) = memchr::memchr(b'`', &bytes[start..]) else {
            break;
        };
        count += 1;
        start += off + 1;
    }
    count
}

fn has_matching_backtick_runs(bytes: &[u8]) -> bool {
    let mut seen_short = [false; 64];
    let mut seen_long: Option<FxHashMap<u32, ()>> = None;
    let mut i = 0usize;

    while i < bytes.len() {
        let Some(off) = memchr::memchr(b'`', &bytes[i..]) else {
            break;
        };
        i += off;
        let run_start = i;
        i += 1;
        while i < bytes.len() && bytes[i] == b'`' {
            i += 1;
        }
        let run_len = i - run_start;
        if run_len < seen_short.len() {
            let slot = &mut seen_short[run_len];
            if *slot {
                return true;
            }
            *slot = true;
        } else {
            let map = seen_long.get_or_insert_with(FxHashMap::default);
            let key = run_len as u32;
            if map.contains_key(&key) {
                return true;
            }
            map.insert(key, ());
        }
    }

    false
}

fn try_emit_common_inline(out: &mut String, raw: &str, opts: &ParseOptions) -> bool {
    if raw.len() > 256 || raw.is_empty() || opts.collapse_whitespace || raw.contains('\n') {
        return false;
    }

    let bytes = raw.as_bytes();
    let len = bytes.len();
    let mut text_start = 0usize;
    // Track whether current plain-text segment needs HTML escaping.
    // We set this on '>' since '<', '&', '"' are already bailout chars above.
    let mut seg_needs_escape = false;
    let mut i = 0usize;

    out.clear();
    out.reserve(raw.len() + 16);

    // Flush text_start..at to `out`, using push_str when no escaping needed.
    macro_rules! flush_text {
        ($at:expr) => {
            if text_start < $at {
                if seg_needs_escape {
                    escape_html_into(out, &raw[text_start..$at]);
                } else {
                    // SAFETY: text_start and $at are byte positions within `raw`
                    // advanced only at ASCII boundaries in this function.
                    out.push_str(unsafe { raw.get_unchecked(text_start..$at) });
                }
            }
        };
    }

    while i < len {
        match bytes[i] {
            // HTML special: '&', '<', '"' force a full escape pass (rare in prose).
            // '!' and '_' are CommonMark specials we can't fast-path.
            b'\\' | b'&' | b'<' | b'!' | b'_' => return false,
            b'>' => {
                // '>' needs HTML escaping but is otherwise harmless for inline structure.
                seg_needs_escape = true;
                i += 1;
            }
            b':' | b'@' if opts.enable_autolink => return false,
            b'$' if opts.enable_latex_math => return false,
            b'[' => {
                flush_text!(i);
                let Some(next_i) = emit_simple_link(out, raw, opts, i, i) else {
                    return false;
                };
                i = next_i;
                text_start = i;
                seg_needs_escape = false;
            }
            b'`' => {
                flush_text!(i);
                let Some(next_i) = emit_simple_code(out, raw, i, i) else {
                    return false;
                };
                i = next_i;
                text_start = i;
                seg_needs_escape = false;
            }
            b'*' => {
                flush_text!(i);
                let Some(next_i) = emit_simple_delim(
                    out,
                    raw,
                    i,
                    i,
                    b'*',
                    "<em>",
                    "</em>",
                    "<strong>",
                    "</strong>",
                ) else {
                    return false;
                };
                i = next_i;
                text_start = i;
                seg_needs_escape = false;
            }
            b'~' if opts.enable_strikethrough => {
                flush_text!(i);
                let Some(next_i) =
                    emit_simple_double_delim(out, raw, i, i, b'~', "<del>", "</del>")
                else {
                    return false;
                };
                i = next_i;
                text_start = i;
                seg_needs_escape = false;
            }
            b'=' if opts.enable_highlight => {
                flush_text!(i);
                let Some(next_i) =
                    emit_simple_double_delim(out, raw, i, i, b'=', "<mark>", "</mark>")
                else {
                    return false;
                };
                i = next_i;
                text_start = i;
                seg_needs_escape = false;
            }
            b'+' if opts.enable_underline => {
                flush_text!(i);
                let Some(next_i) = emit_simple_double_delim(out, raw, i, i, b'+', "<u>", "</u>")
                else {
                    return false;
                };
                i = next_i;
                text_start = i;
                seg_needs_escape = false;
            }
            _ => {
                i += 1;
            }
        }
    }

    if text_start < len {
        if seg_needs_escape {
            escape_html_into(out, &raw[text_start..]);
        } else {
            // SAFETY: text_start is at an ASCII boundary within `raw`.
            out.push_str(unsafe { raw.get_unchecked(text_start..) });
        }
    }
    true
}

fn emit_simple_code(out: &mut String, raw: &str, text_start: usize, at: usize) -> Option<usize> {
    let bytes = raw.as_bytes();
    if at + 1 < bytes.len() && bytes[at + 1] == b'`' {
        return None;
    }
    let close_rel = memchr::memchr(b'`', &bytes[at + 1..])?;
    let close = at + 1 + close_rel;
    let content = &raw[at + 1..close];
    if content.is_empty()
        || content.contains('\n')
        || content.as_bytes()[0] == b' '
        || content.as_bytes()[content.len() - 1] == b' '
    {
        return None;
    }

    if text_start < at {
        escape_html_into(out, &raw[text_start..at]);
    }
    out.push_str("<code>");
    escape_html_into(out, content);
    out.push_str("</code>");
    Some(close + 1)
}

fn emit_simple_link(
    out: &mut String,
    raw: &str,
    opts: &ParseOptions,
    text_start: usize,
    at: usize,
) -> Option<usize> {
    let bytes = raw.as_bytes();
    let close_bracket_rel = memchr::memchr(b']', &bytes[at + 1..])?;
    let close_bracket = at + 1 + close_bracket_rel;
    if close_bracket + 1 >= bytes.len() || bytes[close_bracket + 1] != b'(' {
        return None;
    }
    let close_paren_rel = memchr::memchr(b')', &bytes[close_bracket + 2..])?;
    let close_paren = close_bracket + 2 + close_paren_rel;
    let label = &raw[at + 1..close_bracket];
    let dest = &raw[close_bracket + 2..close_paren];

    if !is_simple_link_label(label) || !is_simple_link_dest(dest, opts) {
        return None;
    }

    if text_start < at {
        escape_html_into(out, &raw[text_start..at]);
    }
    out.push_str("<a href=\"");
    if !is_dangerous_url(dest) {
        encode_url_escaped_into(out, dest);
    }
    out.push_str("\">");
    escape_html_into(out, label);
    out.push_str("</a>");
    Some(close_paren + 1)
}

#[allow(clippy::too_many_arguments)]
fn emit_simple_delim(
    out: &mut String,
    raw: &str,
    text_start: usize,
    at: usize,
    marker: u8,
    single_open: &str,
    single_close: &str,
    double_open: &str,
    double_close: &str,
) -> Option<usize> {
    let bytes = raw.as_bytes();
    let count = if at + 1 < bytes.len() && bytes[at + 1] == marker {
        2
    } else {
        1
    };

    let before = if at > 0 { char_before(raw, at) } else { ' ' };
    let after = if at + count < bytes.len() {
        char_at(raw, at + count)
    } else {
        ' '
    };
    let (can_open, _) = flanking(marker, before, after);
    if !can_open {
        return None;
    }

    let close = find_simple_delim_close(raw, at + count, marker, count)?;
    let content = &raw[at + count..close];
    if !is_simple_delim_content(content) {
        return None;
    }

    if text_start < at {
        escape_html_into(out, &raw[text_start..at]);
    }
    let (open, close_tag) = if count == 2 {
        (double_open, double_close)
    } else {
        (single_open, single_close)
    };
    out.push_str(open);
    escape_html_into(out, content);
    out.push_str(close_tag);
    Some(close + count)
}

fn emit_simple_double_delim(
    out: &mut String,
    raw: &str,
    text_start: usize,
    at: usize,
    marker: u8,
    open: &str,
    close_tag: &str,
) -> Option<usize> {
    let bytes = raw.as_bytes();
    if at + 1 >= bytes.len() || bytes[at + 1] != marker {
        return None;
    }
    let close = find_simple_delim_close(raw, at + 2, marker, 2)?;
    let content = &raw[at + 2..close];
    if !is_simple_delim_content(content) {
        return None;
    }

    if text_start < at {
        escape_html_into(out, &raw[text_start..at]);
    }
    out.push_str(open);
    escape_html_into(out, content);
    out.push_str(close_tag);
    Some(close + 2)
}

fn find_simple_delim_close(raw: &str, mut from: usize, marker: u8, count: usize) -> Option<usize> {
    let bytes = raw.as_bytes();
    while from < bytes.len() {
        let rel = memchr::memchr(marker, &bytes[from..])?;
        let idx = from + rel;
        if count == 2 && (idx + 1 >= bytes.len() || bytes[idx + 1] != marker) {
            from = idx + 1;
            continue;
        }
        let before = char_before(raw, idx);
        let after = if idx + count < bytes.len() {
            char_at(raw, idx + count)
        } else {
            ' '
        };
        let (_, can_close) = flanking(marker, before, after);
        if can_close {
            return Some(idx);
        }
        from = idx + 1;
    }
    None
}

fn is_simple_delim_content(content: &str) -> bool {
    !content.is_empty()
        && !content.as_bytes().iter().any(|&b| {
            matches!(
                b,
                b'\\' | b'&' | b'<' | b'!' | b'[' | b']' | b'`' | b'*' | b'_' | b'~' | b'=' | b'+'
            )
        })
}

fn is_simple_link_label(label: &str) -> bool {
    !label.is_empty()
        && !label.as_bytes().iter().any(|&b| {
            matches!(
                b,
                b'\\'
                    | b'&'
                    | b'<'
                    | b'!'
                    | b'['
                    | b']'
                    | b'`'
                    | b'*'
                    | b'_'
                    | b'~'
                    | b'='
                    | b'+'
                    | b':'
                    | b'@'
                    | b'$'
            )
        })
}

fn is_simple_link_dest(dest: &str, opts: &ParseOptions) -> bool {
    !dest.is_empty()
        && !dest.as_bytes().iter().any(|&b| {
            matches!(
                b,
                b' ' | b'\t' | b'\n' | b'<' | b'>' | b'&' | b'\\' | b'(' | b')'
            ) || (opts.enable_latex_math && b == b'$')
        })
}

#[derive(Clone, Copy)]
struct EmDelim {
    orig_start: u32,
    orig_end: u32,
    cur_start: u32,
    cur_end: u32,
    marker: u8,
    can_open: bool,
    can_close: bool,
    active: bool,
    open_em: [u8; 4],
    open_em_len: u8,
    close_em: [u8; 4],
    close_em_len: u8,
}

#[inline(never)]
fn process_em_delims(delims: &mut [EmDelim]) {
    let mut openers_bottom = [0usize; 6];
    let mut closer_idx = 0usize;
    while closer_idx < delims.len() {
        if !delims[closer_idx].active
            || !delims[closer_idx].can_close
            || delims[closer_idx].cur_end == delims[closer_idx].cur_start
        {
            closer_idx += 1;
            continue;
        }
        let cmarker = delims[closer_idx].marker;
        let ccount = (delims[closer_idx].cur_end - delims[closer_idx].cur_start) as u16;
        let bottom = openers_bottom[em_openers_index(cmarker, ccount)];

        let mut found = false;
        let mut oi = closer_idx;
        while oi > bottom {
            oi -= 1;
            if !delims[oi].active || delims[oi].marker != cmarker || !delims[oi].can_open {
                continue;
            }
            let ocount = (delims[oi].cur_end - delims[oi].cur_start) as u16;
            if ocount == 0 {
                continue;
            }
            if (delims[oi].can_close || delims[closer_idx].can_open)
                && (ocount + ccount).is_multiple_of(3)
                && (!ocount.is_multiple_of(3) || !ccount.is_multiple_of(3))
            {
                continue;
            }
            let use_count: u16 = if ocount >= 2 && ccount >= 2 { 2 } else { 1 };
            let tag = use_count as u8;

            delims[oi].cur_end -= use_count as u32;
            let idx = delims[oi].open_em_len as usize;
            if idx < 4 {
                delims[oi].open_em[idx] = tag;
                delims[oi].open_em_len += 1;
            }

            delims[closer_idx].cur_start += use_count as u32;
            let idx = delims[closer_idx].close_em_len as usize;
            if idx < 4 {
                delims[closer_idx].close_em[idx] = tag;
                delims[closer_idx].close_em_len += 1;
            }

            for delim in delims.iter_mut().take(closer_idx).skip(oi + 1) {
                delim.active = false;
            }

            if delims[oi].cur_end == delims[oi].cur_start {
                delims[oi].active = false;
            }
            if delims[closer_idx].cur_end == delims[closer_idx].cur_start {
                delims[closer_idx].active = false;
            }

            found = true;
            break;
        }
        if !found {
            openers_bottom[em_openers_index(cmarker, ccount)] = closer_idx;
            if !delims[closer_idx].can_open {
                delims[closer_idx].active = false;
            }
            closer_idx += 1;
        }
    }
}

#[inline(always)]
fn em_openers_index(marker: u8, count: u16) -> usize {
    match marker {
        b'*' => (count % 3) as usize,
        b'_' => 3 + (count % 3) as usize,
        _ => 0,
    }
}

#[inline(always)]
fn render_em_delim(out: &mut String, d: &EmDelim) {
    for j in 0..d.close_em_len as usize {
        out.push_str(if d.close_em[j] == 2 {
            "</strong>"
        } else {
            "</em>"
        });
    }
    if d.cur_start < d.cur_end {
        let marker = d.marker as char;
        for _ in 0..(d.cur_end - d.cur_start) {
            out.push(marker);
        }
    }
    for j in (0..d.open_em_len as usize).rev() {
        out.push_str(if d.open_em[j] == 2 {
            "<strong>"
        } else {
            "<em>"
        });
    }
}

#[inline]
fn scan_em_delims(raw: &str, bytes: &[u8], skip_escapes: bool, buf: &mut Vec<EmDelim>) {
    buf.clear();
    let len = bytes.len();
    let mut i = 0;
    while i < len {
        // Use memchr to skip to the next interesting byte.
        let next = if skip_escapes {
            memchr::memchr3(b'*', b'_', b'\\', &bytes[i..])
        } else {
            memchr::memchr2(b'*', b'_', &bytes[i..])
        };
        let Some(off) = next else { break };
        i += off;
        let b = bytes[i];
        if b == b'\\' {
            // skip_escapes must be true if we got here
            if i + 1 < len && crate::is_ascii_punctuation(bytes[i + 1]) {
                i += 2;
            } else {
                i += 1;
            }
            continue;
        }
        let run_start = i;
        i += 1;
        while i < len && bytes[i] == b {
            i += 1;
        }
        let before = if run_start > 0 {
            char_before(raw, run_start)
        } else {
            ' '
        };
        let after = if i < len { char_at(raw, i) } else { ' ' };
        let (can_open, can_close) = flanking(b, before, after);
        buf.push(EmDelim {
            orig_start: run_start as u32,
            orig_end: i as u32,
            cur_start: run_start as u32,
            cur_end: i as u32,
            marker: b,
            can_open,
            can_close,
            active: true,
            open_em: [0; 4],
            open_em_len: 0,
            close_em: [0; 4],
            close_em_len: 0,
        });
    }
}

#[inline]
fn emit_emphasis_only(out: &mut String, raw: &str, bytes: &[u8], em_buf: &mut Vec<EmDelim>) {
    scan_em_delims(raw, bytes, false, em_buf);
    if em_buf.is_empty() {
        escape_html_into(out, raw);
        return;
    }
    process_em_delims(em_buf);

    let mut text_pos = 0usize;
    for d in em_buf.iter() {
        if text_pos < d.orig_start as usize {
            escape_html_into(out, &raw[text_pos..d.orig_start as usize]);
        }
        render_em_delim(out, d);
        text_pos = d.orig_end as usize;
    }
    if text_pos < bytes.len() {
        escape_html_into(out, &raw[text_pos..]);
    }
}

fn emit_breaks_and_emphasis(
    out: &mut String,
    raw: &str,
    bytes: &[u8],
    opts: &ParseOptions,
    em_buf: &mut Vec<EmDelim>,
) {
    scan_em_delims(raw, bytes, true, em_buf);
    if !em_buf.is_empty() {
        process_em_delims(em_buf);
    }

    let len = bytes.len();
    let mut text_pos = 0usize;
    let mut di = 0usize;

    while text_pos < len {
        let next_delim_pos = if di < em_buf.len() {
            em_buf[di].orig_start as usize
        } else {
            len
        };
        emit_text_with_breaks(out, raw, bytes, &mut text_pos, next_delim_pos, opts);

        if di < em_buf.len() && text_pos == em_buf[di].orig_start as usize {
            render_em_delim(out, &em_buf[di]);
            text_pos = em_buf[di].orig_end as usize;
            di += 1;
        }
    }
}

#[inline]
fn emit_text_with_breaks(
    out: &mut String,
    raw: &str,
    bytes: &[u8],
    text_pos: &mut usize,
    end: usize,
    opts: &ParseOptions,
) {
    let mut seg_start = *text_pos;
    let mut i = *text_pos;
    while i < end {
        // Jump to next interesting byte using memchr2.
        let off = memchr::memchr2(b'\n', b'\\', &bytes[i..end]);
        match off {
            None => break,
            Some(off) => i += off,
        }
        let b = bytes[i];
        if b == b'\n' {
            let mut text_end = i;
            let is_hard = (i >= seg_start + 2 && bytes[i - 1] == b' ' && bytes[i - 2] == b' ')
                || (i > seg_start && bytes[i - 1] == b'\\');
            if is_hard && i > seg_start && bytes[i - 1] == b'\\' {
                text_end = i - 1;
            } else {
                while text_end > seg_start && bytes[text_end - 1] == b' ' {
                    text_end -= 1;
                }
            }
            if seg_start < text_end {
                escape_html_into(out, &raw[seg_start..text_end]);
            }
            if is_hard || opts.hard_breaks {
                out.push_str("<br />\n");
            } else {
                out.push('\n');
            }
            i += 1;
            seg_start = i;
        } else {
            // b == b'\\'
            if i + 1 < end {
                let next = bytes[i + 1];
                if next == b'\n' {
                    i += 1;
                    continue;
                }
                if crate::is_ascii_punctuation(next) {
                    if seg_start < i {
                        escape_html_into(out, &raw[seg_start..i]);
                    }
                    match next {
                        b'&' => out.push_str("&amp;"),
                        b'<' => out.push_str("&lt;"),
                        b'>' => out.push_str("&gt;"),
                        b'"' => out.push_str("&quot;"),
                        _ => out.push_str(&raw[i + 1..i + 2]),
                    }
                    i += 2;
                    seg_start = i;
                } else {
                    i += 1;
                }
            } else {
                i += 1;
            }
        }
    }
    if seg_start < end {
        escape_html_into(out, &raw[seg_start..end]);
    }
    *text_pos = end;
}

pub(crate) struct InlineBuffers {
    items: Vec<InlineItem>,
    delims: Vec<usize>,
    brackets: Vec<BracketInfo>,
    links: Vec<LinkInfo>,
    em_delims: Vec<EmDelim>,
    pub(crate) scan_table: [u8; 256],
    scan_key: u8,
    pub(crate) scratch: String,
}

impl InlineBuffers {
    pub(crate) fn new() -> Self {
        Self {
            items: Vec::with_capacity(64),
            delims: Vec::with_capacity(16),
            brackets: Vec::with_capacity(8),
            links: Vec::with_capacity(8),
            em_delims: Vec::with_capacity(16),
            scan_table: build_scan_table(DEFAULT_SCAN_KEY),
            scan_key: DEFAULT_SCAN_KEY,
            scratch: String::new(),
        }
    }

    #[inline]
    pub(crate) fn prepare(&mut self, opts: &ParseOptions) {
        let key = scan_key(opts);
        if self.scan_key != key {
            self.scan_table = build_scan_table(key);
            self.scan_key = key;
        }
    }
}

#[inline(always)]
fn scan_key(opts: &ParseOptions) -> u8 {
    (opts.enable_strikethrough as u8)
        | ((opts.enable_highlight as u8) << 1)
        | ((opts.enable_underline as u8) << 2)
        | ((opts.enable_autolink as u8) << 3)
        | ((opts.enable_latex_math as u8) << 4)
}

const DEFAULT_SCAN_KEY: u8 = 0b0_1111;

fn build_scan_table(key: u8) -> [u8; 256] {
    let mut t = [0u8; 256];
    t[b'*' as usize] = SCAN_EMPH;
    t[b'_' as usize] = SCAN_EMPH;
    t[b'\\' as usize] = SCAN_BREAK;
    t[b'\n' as usize] = SCAN_BREAK;
    t[b'>' as usize] = SCAN_ESCAPE;
    t[b'"' as usize] = SCAN_ESCAPE;
    t[b'`' as usize] = SCAN_FULL | SCAN_IS_BACKTICK;
    t[b'!' as usize] = SCAN_FULL;
    t[b'[' as usize] = SCAN_FULL;
    t[b']' as usize] = SCAN_FULL;
    t[b'<' as usize] = SCAN_FULL;
    t[b'&' as usize] = SCAN_FULL;
    if key & 1 != 0 {
        t[b'~' as usize] = SCAN_FULL;
    }
    if key & 2 != 0 {
        t[b'=' as usize] = SCAN_FULL;
    }
    if key & 4 != 0 {
        t[b'+' as usize] = SCAN_FULL;
    }
    if key & 8 != 0 {
        t[b':' as usize] = SCAN_FULL;
        t[b'@' as usize] = SCAN_FULL;
    }
    if key & 16 != 0 {
        t[b'$' as usize] = SCAN_FULL;
    }
    t
}

#[derive(Clone, Debug)]
struct SmallEmVec {
    data: [u8; 4],
    len: u8,
}

impl SmallEmVec {
    #[inline(always)]
    const fn new() -> Self {
        Self {
            data: [0; 4],
            len: 0,
        }
    }
    #[inline(always)]
    fn push(&mut self, val: u8) {
        if (self.len as usize) < 4 {
            self.data[self.len as usize] = val;
            self.len += 1;
        }
    }
    #[inline(always)]
    fn as_slice(&self) -> &[u8] {
        &self.data[..self.len as usize]
    }
}

#[derive(Clone, Debug)]
enum LinkDest {
    Range(u32, u32),
    Owned(Rc<str>),
}

/// Link title — either a byte range into the original input (no escapes/entities)
/// or an owned string (escape/entity processing was needed).
#[derive(Clone, Debug)]
enum LinkTitle {
    Range(u32, u32),
    Owned(Rc<str>),
}

#[derive(Clone, Debug)]
struct LinkInfo {
    dest: LinkDest,
    title: Option<LinkTitle>,
    is_image: bool,
}

#[derive(Clone, Debug)]
enum InlineItem {
    TextRange(usize, usize),
    TextOwned(Box<str>),
    TextStatic(&'static str),
    TextInline {
        buf: [u8; 8],
        len: u8,
    },
    RawHtml(usize, usize),
    Autolink(u32, u32, bool),
    Code(Box<str>),
    /// Code span stored as byte range into input (unescaped). Escaped at render time.
    CodeRange(u32, u32),
    HardBreak,
    SoftBreak,
    DelimRun {
        kind: u8,
        count: u16,
        can_open: bool,
        can_close: bool,
        open_em: SmallEmVec,
        close_em: SmallEmVec,
    },
    BracketOpen {
        is_image: bool,
    },
    LinkStart(u16),
    LinkEnd,
    /// `[[text]]` wiki link. The range covers the text between the `[[` and `]]`.
    WikiLink(usize, usize),
    /// `$...$` inline math. Range covers content (excluding delimiters).
    MathInline(usize, usize),
    /// `$$...$$` display math. Range covers content (excluding delimiters).
    MathDisplay(usize, usize),
}

#[derive(Clone, Debug)]
struct BracketInfo {
    item_idx: usize,
    is_image: bool,
    delim_bottom: usize,
    active: bool,
    text_pos: usize,
}

struct InlineScanner<'a> {
    input: &'a str,
    bytes: &'a [u8],
    pos: usize,
    refs: &'a LinkRefMap,
    opts: &'a ParseOptions,
    items: &'a mut Vec<InlineItem>,
    delims: &'a mut Vec<usize>,
    brackets: &'a mut Vec<BracketInfo>,
    links: &'a mut Vec<LinkInfo>,
    /// `backtick_last[n]` = last byte offset of a run of length `n` (`u32::MAX` = absent).
    /// Covers lengths 1–63; longer runs go into the overflow map (only allocated when needed).
    /// Pre-populated at construction for O(1) bail in `scan_code_span`.
    has_backtick_index: bool,
    backtick_last: [u32; 64],
    backtick_last_long: Option<rustc_hash::FxHashMap<u32, u32>>,
}

impl<'a> InlineScanner<'a> {
    fn new_with_bufs(
        input: &'a str,
        refs: &'a LinkRefMap,
        opts: &'a ParseOptions,
        bufs: &'a mut InlineBuffers,
        backtick_hint: u8,
    ) -> Self {
        bufs.items.clear();
        bufs.delims.clear();
        bufs.brackets.clear();
        bufs.links.clear();
        // Only build the backtick index when the pre-scan saw several backticks.
        // Tiny one-code-span paragraphs are cheaper to handle with local memchr scans.
        let has_backtick_index = backtick_hint >= 3;
        let (backtick_last, backtick_last_long) = if has_backtick_index {
            Self::build_backtick_last(input.as_bytes())
        } else {
            ([u32::MAX; 64], None)
        };
        Self {
            input,
            bytes: input.as_bytes(),
            pos: 0,
            refs,
            opts,
            items: &mut bufs.items,
            delims: &mut bufs.delims,
            brackets: &mut bufs.brackets,
            links: &mut bufs.links,
            has_backtick_index,
            backtick_last,
            backtick_last_long,
        }
    }

    /// Single forward pass using memchr: record the last byte offset of each backtick run length.
    fn build_backtick_last(bytes: &[u8]) -> ([u32; 64], Option<rustc_hash::FxHashMap<u32, u32>>) {
        let mut last = [u32::MAX; 64];
        let mut long: Option<rustc_hash::FxHashMap<u32, u32>> = None;
        let mut i = 0;
        while let Some(rel) = memchr::memchr(b'`', &bytes[i..]) {
            let start = i + rel;
            i = start;
            let mut run = 0usize;
            while i < bytes.len() && bytes[i] == b'`' {
                run += 1;
                i += 1;
            }
            if run < 64 {
                last[run] = start as u32;
            } else {
                long.get_or_insert_with(rustc_hash::FxHashMap::default)
                    .insert(run as u32, start as u32);
            }
        }
        (last, long)
    }
}

pub(super) use crate::is_ascii_punctuation;
pub(super) use crate::utf8_char_len;

static EMAIL_LOCAL: [bool; 256] = {
    let mut t = [false; 256];
    let ranges: &[(u8, u8)] = &[(b'0', b'9'), (b'A', b'Z'), (b'a', b'z')];
    let mut r = 0;
    while r < 3 {
        let mut i = ranges[r].0;
        while i <= ranges[r].1 {
            t[i as usize] = true;
            i += 1;
        }
        r += 1;
    }
    let extra = b".!#$%&'*+/=?^_`{|}~-";
    let mut j = 0;
    while j < extra.len() {
        t[extra[j] as usize] = true;
        j += 1;
    }
    t
};

#[inline(always)]
fn is_email_local_char(b: u8) -> bool {
    EMAIL_LOCAL[b as usize]
}

#[inline(always)]
fn is_punctuation_char(c: char) -> bool {
    if c.is_ascii() {
        return is_ascii_punctuation(c as u8);
    }
    matches!(c as u32,
        0x00A0..=0x00BF | 0x2000..=0x206F | 0x2E00..=0x2E7F |
        0x3000..=0x303F | 0xFE30..=0xFE6F | 0xFF01..=0xFF0F |
        0xFF1A..=0xFF20 | 0xFF3B..=0xFF40 | 0xFF5B..=0xFF65 |
        0x2100..=0x214F | 0x2190..=0x21FF | 0x2200..=0x22FF |
        0x2300..=0x23FF | 0x2500..=0x257F | 0x25A0..=0x25FF |
        0x2600..=0x26FF | 0x2700..=0x27BF | 0x20A0..=0x20CF
    )
}

#[inline(always)]
fn flanking(marker: u8, before: char, after: char) -> (bool, bool) {
    let left_flanking = !after.is_whitespace()
        && (!is_punctuation_char(after) || before.is_whitespace() || is_punctuation_char(before));
    let right_flanking = !before.is_whitespace()
        && (!is_punctuation_char(before) || after.is_whitespace() || is_punctuation_char(after));
    if marker == b'_' {
        (
            left_flanking && (!right_flanking || is_punctuation_char(before)),
            right_flanking && (!left_flanking || is_punctuation_char(after)),
        )
    } else {
        (left_flanking, right_flanking)
    }
}

#[inline(always)]
fn char_before(s: &str, byte_pos: usize) -> char {
    if byte_pos == 0 {
        return ' ';
    }
    let bytes = s.as_bytes();
    let prev = bytes[byte_pos - 1];
    if prev < 0x80 {
        return prev as char;
    }
    let mut i = byte_pos - 1;
    while i > 0 && (bytes[i] & 0xC0) == 0x80 {
        i -= 1;
    }
    s[i..byte_pos].chars().next().unwrap_or(' ')
}

#[inline(always)]
fn char_at(s: &str, byte_pos: usize) -> char {
    if byte_pos >= s.len() {
        return ' ';
    }
    let b = s.as_bytes()[byte_pos];
    if b < 0x80 {
        return b as char;
    }
    let len = utf8_char_len(b);
    let end = (byte_pos + len).min(s.len());
    s[byte_pos..end].chars().next().unwrap_or(' ')
}
