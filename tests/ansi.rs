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

#[test]
fn padding_adds_horizontal_spaces_to_each_line() {
    let long = "word ".repeat(25);
    let out = render_ansi(
        long.trim(),
        &opts(),
        Some(&AnsiOptions {
            color: false,
            width: 20,
            padding: 2,
            ..AnsiOptions::default()
        }),
    );

    let lines: Vec<&str> = out.lines().collect();
    assert!(lines.len() > 1, "expected wrapping with padding");

    // Skip blank lines at the top (top padding)
    let content_lines: Vec<&str> = lines
        .into_iter()
        .skip_while(|line| line.trim().is_empty())
        .collect();

    for line in &content_lines {
        assert!(line.starts_with("  "), "expected left padding: {line:?}");
        assert!(line.ends_with("  "), "expected right padding: {line:?}");
        assert!(line.len() <= 24, "line too long: {}", line.len());
    }
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

// ── responsive tables ────────────────────────────────────────────────────────

fn render_width(md: &str, width: usize) -> String {
    render_ansi(
        md,
        &opts(),
        Some(&AnsiOptions {
            color: false,
            width,
            ..AnsiOptions::default()
        }),
    )
}

#[test]
fn table_fits_within_terminal_width() {
    // A table with short cells should fit without wrapping at width 80.
    let out = render_width("| A | B |\n|---|---|\n| 1 | 2 |", 80);
    for line in out.lines() {
        assert!(
            line.chars().count() <= 80,
            "table line exceeds width 80 ({} cols): {line:?}",
            line.chars().count()
        );
    }
}

#[test]
fn wide_table_constrained_to_terminal_width() {
    // Build a table that would naturally exceed 40 columns.
    let md = "| Column One Header | Column Two Header |\n|---|---|\n| some long value | another long val |";
    let out = render_width(md, 40);
    for line in out.lines() {
        if line.is_empty() {
            continue;
        }
        assert!(
            line.chars().count() <= 40,
            "table line exceeds width 40 ({} cols): {line:?}",
            line.chars().count()
        );
    }
}

#[test]
fn wide_table_preserves_all_content() {
    let md = "| Name | Description |\n|---|---|\n| Alice | A person |";
    let out = render_width(md, 30);
    assert!(out.contains("Name"), "header 'Name' missing");
    assert!(out.contains("Description"), "header 'Description' missing");
    assert!(out.contains("Alice"), "cell 'Alice' missing");
    assert!(out.contains("person"), "cell content 'person' missing");
}

#[test]
fn narrow_table_wraps_cell_content() {
    // Force very narrow width so cells must wrap.
    let md = "| Header | Value |\n|---|---|\n| some longer text here | short |";
    let out = render_width(md, 30);
    // All content should still be present.
    assert!(out.contains("Header"));
    assert!(out.contains("Value"));
    assert!(out.contains("short"));
    // The word "longer" from "some longer text here" should be present
    // (possibly on a wrapped line).
    assert!(out.contains("longer"), "wrapped content missing");
}

#[test]
fn wrapped_cell_creates_multiline_row() {
    // A cell with long content at narrow width should produce multiple lines
    // within that logical row.
    let md = "| Col |\n|---|\n| one two three four five six seven eight |";
    let out = render_width(md, 20);
    // Count how many lines contain the row border character inside content area.
    // A wrapped row should have multiple │...│ lines for the same logical row.
    let content_lines: Vec<_> = out
        .lines()
        .filter(|l| l.starts_with('│') || l.contains('│'))
        .filter(|l| !l.contains('─')) // exclude border lines
        .collect();
    // Should have header (1 line) + data row (multiple lines due to wrapping)
    assert!(
        content_lines.len() >= 2,
        "expected multi-line row from wrapping, got {} lines:\n{}",
        content_lines.len(),
        out
    );
    // All words should be present
    for word in [
        "one", "two", "three", "four", "five", "six", "seven", "eight",
    ] {
        assert!(out.contains(word), "missing word '{word}' in:\n{out}");
    }
}

#[test]
fn many_columns_constrained() {
    let md = "| A | B | C | D | E | F |\n|---|---|---|---|---|---|\n| 1 | 2 | 3 | 4 | 5 | 6 |";
    let out = render_width(md, 40);
    for line in out.lines() {
        if line.is_empty() {
            continue;
        }
        assert!(
            line.chars().count() <= 40,
            "table line exceeds width 40 ({} cols): {line:?}",
            line.chars().count()
        );
    }
    // All cells should still be present.
    for ch in ['A', 'B', 'C', 'D', 'E', 'F', '1', '2', '3', '4', '5', '6'] {
        assert!(out.contains(ch), "missing cell content: {ch}");
    }
}

#[test]
fn many_columns_extreme_narrow() {
    // 6 columns at width 20 — MIN_COL*6=18 + chrome=19 = 37 > 20, so this
    // hits the infeasible branch. Verify no panic and content is present.
    let md = "| A | B | C | D | E | F |\n|---|---|---|---|---|---|\n| 1 | 2 | 3 | 4 | 5 | 6 |";
    let out = render_width(md, 20);
    for ch in ['A', 'B', 'C', 'D', 'E', 'F'] {
        assert!(out.contains(ch), "missing cell content: {ch}");
    }
}

#[test]
fn table_unconstrained_when_width_zero() {
    // width=0 disables width-dependent formatting — table should render at
    // natural size.
    let md = "| Long Column Name | Another Long Column |\n|---|---|\n| value | data |";
    let zero = render_width(md, 0);
    let wide = render_width(md, 200);
    assert_eq!(zero, wide, "width=0 should not constrain the table");
}

#[test]
fn long_word_char_wraps_in_table_cell() {
    // A single long word should be character-wrapped, not overflow.
    let md = "| Name |\n|---|\n| Strikethrough |";
    let out = render_width(md, 15);
    // All characters of "Strikethrough" should be present
    for ch in [
        'S', 't', 'r', 'i', 'k', 'e', 't', 'h', 'r', 'o', 'u', 'g', 'h',
    ] {
        assert!(out.contains(ch), "missing char '{ch}' in:\n{out}");
    }
    // Lines should not exceed width
    for line in out.lines() {
        if line.is_empty() {
            continue;
        }
        assert!(
            line.chars().count() <= 15,
            "line exceeds width 15 ({} cols): {line:?}",
            line.chars().count()
        );
    }
}

#[test]
fn long_camel_case_wraps_correctly() {
    // Test the exact case from the user's example: camelCase names in narrow columns
    let md = "| Option | JS |\n|---|---|\n| Strikethrough | enableStrikethrough |";
    let out = render_width(md, 30);
    // Content present
    assert!(out.contains("Strikethrough") || out.contains("Strike"));
    assert!(out.contains("enable") || out.contains("Strik"));
    // Width constraint respected
    for line in out.lines() {
        if line.is_empty() {
            continue;
        }
        assert!(
            line.chars().count() <= 30,
            "line exceeds width 30 ({} cols): {line:?}",
            line.chars().count()
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
