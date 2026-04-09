use crate::ParseOptions;
use crate::inline::{InlineBuffers, LinkRefMap, parse_inline_pass};

use super::AnsiOptions;
use super::constants::*;

/// Render inline Markdown content as ANSI-escaped text.
///
/// Renders to HTML via [`parse_inline_pass`] and then converts HTML tags to
/// ANSI sequences, reusing the full inline parser without a second code path.
pub(super) fn parse_inline_ansi(
    out: &mut String,
    raw: &str,
    refs: &LinkRefMap,
    opts: &ParseOptions,
    aopts: &AnsiOptions,
    bufs: &mut InlineBuffers,
) {
    let mut html = String::with_capacity(raw.len() + 16);
    parse_inline_pass(&mut html, raw, refs, opts, bufs);
    html_to_ansi(&html, out, aopts.color);
}

/// Like [`parse_inline_ansi`] but with a `base_fg` colour that is restored after
/// every closing inline tag. Used for headings so that `**bold**` inside a heading
/// renders in the heading colour rather than overriding it with `FG_STRONG`.
pub(super) fn parse_inline_ansi_heading(
    out: &mut String,
    raw: &str,
    refs: &LinkRefMap,
    opts: &ParseOptions,
    aopts: &AnsiOptions,
    bufs: &mut InlineBuffers,
    heading_fg: &str,
) {
    let mut html = String::with_capacity(raw.len() + 16);
    parse_inline_pass(&mut html, raw, refs, opts, bufs);
    html_to_ansi_inner(&html, out, aopts.color, false, Some(heading_fg));
}

/// Translate the HTML subset produced by the inline renderer to ANSI sequences.
///
/// Handles the four HTML entities (`&amp;`, `&lt;`, `&gt;`, `&quot;`),
/// formatting tags (`<strong>`, `<em>`, `<del>`, `<mark>`, `<u>`, `<code>`),
/// links (`<a href=…>`), images (`<img alt=… src=…>`), task-list checkboxes
/// (`<input …>`), and math spans (`<span class="math…">`).
///
/// When `color` is `false`, tags are dropped and entities are decoded, producing
/// plain text.
pub(super) fn html_to_ansi(html: &str, out: &mut String, color: bool) {
    html_to_ansi_inner(html, out, color, false, None);
}

/// `base_fg`: when `Some(fg)`, inline formatting tags don't override colour —
/// `RESET` restores `BOLD + fg` so the heading colour is always the base.
pub(super) fn html_to_ansi_inner(
    html: &str,
    out: &mut String,
    color: bool,
    in_code: bool,
    base_fg: Option<&str>,
) {
    let bytes = html.as_bytes();
    let len = bytes.len();
    let mut i = 0;
    let mut inside_code = in_code;

    while i < len {
        if bytes[i] == b'&' {
            if html[i..].starts_with("&amp;") {
                out.push('&');
                i += 5;
            } else if html[i..].starts_with("&lt;") {
                out.push('<');
                i += 4;
            } else if html[i..].starts_with("&gt;") {
                out.push('>');
                i += 4;
            } else if html[i..].starts_with("&quot;") {
                out.push('"');
                i += 6;
            } else {
                out.push('&');
                i += 1;
            }
            continue;
        }

        if bytes[i] != b'<' {
            // Inside a code span, replace ASCII spaces with NBSP so word-wrap
            // never breaks in the middle of inline code.
            if inside_code && bytes[i] == b' ' {
                out.push('\u{00A0}');
                i += 1;
            } else {
                let char_len = crate::utf8_char_len(bytes[i]);
                out.push_str(&html[i..i + char_len]);
                i += char_len;
            }
            continue;
        }

        let tag_end = match memchr::memchr(b'>', &bytes[i..]) {
            Some(off) => i + off + 1,
            None => {
                out.push('<');
                i += 1;
                continue;
            }
        };
        let tag = &html[i..tag_end];

        // Helper: emit a "close" — plain RESET in normal context, RESET+BOLD+fg in heading context.
        macro_rules! close_tag {
            () => {
                if let Some(fg) = base_fg {
                    out.push_str(RESET);
                    out.push_str(BOLD);
                    out.push_str(fg);
                } else {
                    out.push_str(RESET);
                }
            };
        }

        if color {
            match tag {
                "<strong>" => {
                    // In heading context suppress FG_STRONG — heading colour is the base.
                    out.push_str(BOLD);
                    if base_fg.is_none() {
                        out.push_str(FG_STRONG);
                    }
                }
                "</strong>" => close_tag!(),
                "<em>" => {
                    out.push_str(ITALIC);
                    if base_fg.is_none() {
                        out.push_str(FG_ITALIC);
                    }
                }
                "</em>" => close_tag!(),
                "<del>" => {
                    out.push_str(STRIKETHROUGH);
                    if base_fg.is_none() {
                        out.push_str(FG_DEL);
                    }
                }
                "</del>" => close_tag!(),
                "<mark>" => {
                    out.push_str(BG_MARK);
                    out.push_str(FG_MARK);
                }
                "</mark>" => close_tag!(),
                "<u>" => {
                    out.push_str(UNDERLINE);
                    if base_fg.is_none() {
                        out.push_str(FG_UNDERLINE);
                    }
                }
                "</u>" => close_tag!(),
                "<code>" => {
                    out.push_str(BG_INLINE_CODE);
                    out.push_str(FG_INLINE_CODE);
                    inside_code = true;
                }
                "</code>" => {
                    inside_code = false;
                    out.push_str(RESET);
                    // In heading context, restore heading colour after closing code span
                    if let Some(fg) = base_fg {
                        out.push_str(BOLD);
                        out.push_str(fg);
                    }
                }
                "<br />" | "<br />\n" => out.push('\n'),
                _ if tag.starts_with("<a href=") => {
                    if let Some(href_start) = tag.find("href=\"") {
                        let href_start = href_start + 6;
                        if let Some(href_end) = tag[href_start..].find('"') {
                            if let Some(close_off) = html[tag_end..].find("</a>") {
                                let link_text = &html[tag_end..tag_end + close_off];
                                let href = &tag[href_start..href_start + href_end];

                                // OSC 8 clickable hyperlink (supported by most modern terminals)
                                out.push_str("\x1b]8;;");
                                out.push_str(href);
                                out.push_str("\x1b\\");
                                out.push_str(FG_LINK);
                                out.push_str(UNDERLINE);
                                html_to_ansi_inner(link_text, out, color, false, None);
                                out.push_str(RESET);
                                out.push_str("\x1b]8;;\x1b\\");

                                // Show URL suffix only for non-anchor links
                                if !href.starts_with('#') {
                                    out.push_str(FG_LINK_URL);
                                    out.push_str(" (");
                                    let mut decoded = String::with_capacity(href.len());
                                    html_to_ansi_inner(href, &mut decoded, false, false, None);
                                    out.push_str(&decoded);
                                    out.push(')');
                                    out.push_str(RESET);
                                }

                                i = tag_end + close_off + 4;
                                continue;
                            }
                        }
                    }
                }
                "</a>" => out.push_str(RESET),
                _ if tag.starts_with("<img ") => {
                    let alt = tag.find("alt=\"").map(|p| {
                        let start = p + 5;
                        tag[start..]
                            .find('"')
                            .map(|e| &tag[start..start + e])
                            .unwrap_or("")
                    });
                    let src = tag.find("src=\"").map(|p| {
                        let start = p + 5;
                        tag[start..]
                            .find('"')
                            .map(|e| &tag[start..start + e])
                            .unwrap_or("")
                    });

                    out.push_str(FG_IMAGE);
                    out.push_str("◈");
                    if let Some(alt_text) = alt {
                        if !alt_text.is_empty() {
                            out.push(' ');
                            out.push_str(alt_text);
                        }
                    }
                    out.push_str(RESET);

                    // Show image source as dim URL (same treatment as links)
                    if let Some(src_url) = src {
                        if !src_url.is_empty() {
                            out.push_str(FG_LINK_URL);
                            out.push_str(" (");
                            out.push_str(src_url);
                            out.push(')');
                            out.push_str(RESET);
                        }
                    }
                }
                _ if tag.starts_with("<input ") => {
                    // Task list checkbox: use real Unicode symbols
                    if tag.contains("checked") {
                        out.push_str(FG_CHECKED);
                        out.push('✓');
                        out.push_str(RESET);
                    } else {
                        out.push_str(FG_UNCHECKED);
                        out.push('☐');
                        out.push_str(RESET);
                    }
                }
                _ if tag.starts_with("<span class=\"math") => out.push_str(FG_MATH),
                "</span>" => out.push_str(RESET),
                _ => {}
            }
        } else {
            // No-colour: decode entities, strip markup, render meaningful symbols as text
            match tag {
                "<br />" | "<br />\n" => out.push('\n'),
                _ if tag.starts_with("<a href=") => {
                    if let Some(close_off) = html[tag_end..].find("</a>") {
                        html_to_ansi_inner(
                            &html[tag_end..tag_end + close_off],
                            out,
                            false,
                            false,
                            None,
                        );
                        i = tag_end + close_off + 4;
                        continue;
                    }
                }
                _ if tag.starts_with("<img ") => {
                    let alt = tag.find("alt=\"").map(|p| {
                        let start = p + 5;
                        tag[start..]
                            .find('"')
                            .map(|e| &tag[start..start + e])
                            .unwrap_or("")
                    });
                    if let Some(alt_text) = alt {
                        out.push_str("[image: ");
                        out.push_str(alt_text);
                        out.push(']');
                    }
                }
                _ if tag.starts_with("<input ") => {
                    if tag.contains("checked") {
                        out.push_str("[x] ");
                    } else {
                        out.push_str("[ ] ");
                    }
                }
                _ => {}
            }
        }
        i = tag_end;
    }
}
