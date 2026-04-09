use ironmark::{AnsiOptions, ParseOptions, render_ansi};

fn opts() -> ParseOptions {
    ParseOptions {
        hard_breaks: false,
        ..Default::default()
    }
}

fn plain(md: &str) -> String {
    render_ansi(
        md,
        &opts(),
        Some(&AnsiOptions {
            color: false,
            ..AnsiOptions::default()
        }),
    )
}

fn colored(md: &str) -> String {
    render_ansi(md, &opts(), None)
}

// ── defaults ──────────────────────────────────────────────────────────────────

#[test]
fn defaults_produce_ansi_output() {
    let out = colored("# Hello");
    assert!(out.contains('\x1b'), "expected ANSI escapes");
    assert!(out.contains("Hello"));
}

#[test]
fn color_false_strips_all_escapes() {
    let out = plain("# Hello\n\n**bold** and `code`");
    assert!(
        !out.contains('\x1b'),
        "unexpected ANSI escape in plain output"
    );
    assert!(out.contains("Hello"));
    assert!(out.contains("bold"));
    assert!(out.contains("code"));
}

#[test]
fn none_ansi_opts_uses_defaults() {
    let with_none = render_ansi("# Hi", &opts(), None);
    let with_default = render_ansi("# Hi", &opts(), Some(&AnsiOptions::default()));
    assert_eq!(with_none, with_default);
}

// ── headings ─────────────────────────────────────────────────────────────────

#[test]
fn headings_contain_text_no_hash_prefix() {
    for (level, marker) in [
        ("# H1", "H1"),
        ("## H2", "H2"),
        ("### H3", "H3"),
        ("#### H4", "H4"),
        ("##### H5", "H5"),
        ("###### H6", "H6"),
    ] {
        let out = plain(level);
        assert!(out.contains(marker), "missing heading text in: {out:?}");
        assert!(
            !out.trim_start().starts_with('#'),
            "hash prefix leaked: {out:?}"
        );
    }
}

// ── inline formatting ─────────────────────────────────────────────────────────

#[test]
fn bold_text_present_in_output() {
    let out = plain("**bold**");
    assert!(out.contains("bold"));
}

#[test]
fn italic_text_present_in_output() {
    let out = plain("*italic*");
    assert!(out.contains("italic"));
}

#[test]
fn inline_code_present_in_output() {
    let out = plain("`inline code`");
    assert!(out.contains("inline code"));
}

#[test]
fn strikethrough_present_in_output() {
    let out = plain("~~strike~~");
    assert!(out.contains("strike"));
}

// ── blocks ────────────────────────────────────────────────────────────────────

#[test]
fn fenced_code_block_content_present() {
    let out = plain("```rust\nfn main() {}\n```");
    assert!(out.contains("fn main()"));
}

#[test]
fn blockquote_content_present() {
    let out = plain("> quoted text");
    assert!(out.contains("quoted text"));
}

#[test]
fn unordered_list_items_present() {
    let out = plain("- alpha\n- beta\n- gamma");
    assert!(out.contains("alpha"));
    assert!(out.contains("beta"));
    assert!(out.contains("gamma"));
}

#[test]
fn ordered_list_items_present() {
    let out = plain("1. first\n2. second");
    assert!(out.contains("first"));
    assert!(out.contains("second"));
}

#[test]
fn table_cells_present() {
    let out = plain("| A | B |\n|---|---|\n| 1 | 2 |");
    assert!(out.contains('A'));
    assert!(out.contains('B'));
    assert!(out.contains('1'));
    assert!(out.contains('2'));
}

#[test]
fn thematic_break_produces_output() {
    let out = plain("---");
    assert!(!out.trim().is_empty());
}

// ── line numbers ──────────────────────────────────────────────────────────────

#[test]
fn line_numbers_appear_in_code_block() {
    let out = render_ansi(
        "```\nline one\nline two\nline three\n```",
        &opts(),
        Some(&AnsiOptions {
            color: false,
            line_numbers: true,
            ..AnsiOptions::default()
        }),
    );
    assert!(out.contains('1'));
    assert!(out.contains('2'));
    assert!(out.contains('3'));
    assert!(out.contains("line one"));
}

#[test]
fn line_numbers_absent_by_default() {
    let with_nums = render_ansi(
        "```\na\nb\n```",
        &opts(),
        Some(&AnsiOptions {
            color: false,
            line_numbers: true,
            ..AnsiOptions::default()
        }),
    );
    let without_nums = render_ansi(
        "```\na\nb\n```",
        &opts(),
        Some(&AnsiOptions {
            color: false,
            line_numbers: false,
            ..AnsiOptions::default()
        }),
    );
    assert_ne!(with_nums, without_nums);
}

// ── width / wrap ──────────────────────────────────────────────────────────────

#[test]
fn narrow_width_wraps_long_paragraph() {
    let long = "word ".repeat(30);
    let out = render_ansi(
        long.trim(),
        &opts(),
        Some(&AnsiOptions {
            color: false,
            width: 20,
            ..AnsiOptions::default()
        }),
    );
    // at width 20 a 150-char paragraph must produce multiple lines
    let lines: Vec<&str> = out.lines().collect();
    assert!(lines.len() > 1, "expected wrapping at width 20");
    for line in &lines {
        assert!(
            line.len() <= 22, // allow a little slack for the last word
            "line too long ({} chars): {line:?}",
            line.len()
        );
    }
}

// ── empty / edge cases ────────────────────────────────────────────────────────

#[test]
fn empty_input_produces_empty_output() {
    assert_eq!(plain(""), "");
}

#[test]
fn whitespace_only_input_produces_empty_output() {
    assert_eq!(plain("   \n\n\t"), "");
}
