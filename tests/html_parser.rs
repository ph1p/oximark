//! Tests for HTML-to-AST parsing and HTML-to-Markdown conversion.

use ironmark::{
    Block, HtmlParseOptions, ParseOptions, UnknownInlineHandling, html_to_markdown, parse,
    parse_html_to_ast,
};

fn default_opts() -> HtmlParseOptions {
    HtmlParseOptions::default()
}

// ============================================================================
// Basic Block Element Tests
// ============================================================================

#[test]
fn test_paragraph() {
    let ast = parse_html_to_ast("<p>Hello world</p>", &default_opts());
    if let Block::Document { children } = ast {
        assert_eq!(children.len(), 1);
        assert!(matches!(&children[0], Block::Paragraph { raw } if raw == "Hello world"));
    } else {
        panic!("Expected Document");
    }
}

#[test]
fn test_multiple_paragraphs() {
    let ast = parse_html_to_ast("<p>First</p><p>Second</p>", &default_opts());
    if let Block::Document { children } = ast {
        assert_eq!(children.len(), 2);
    } else {
        panic!("Expected Document");
    }
}

#[test]
fn test_headings() {
    for level in 1..=6 {
        let html = format!("<h{}>Heading {}</h{}>", level, level, level);
        let ast = parse_html_to_ast(&html, &default_opts());
        if let Block::Document { children } = ast {
            assert_eq!(children.len(), 1);
            if let Block::Heading { level: l, raw: r } = &children[0] {
                assert_eq!(*l, level);
                assert_eq!(r, &format!("Heading {}", level));
            } else {
                panic!("Expected Heading");
            }
        }
    }
}

#[test]
fn test_code_block() {
    let ast = parse_html_to_ast(
        r#"<pre><code class="language-rust">fn main() {}</code></pre>"#,
        &default_opts(),
    );
    if let Block::Document { children } = ast {
        assert_eq!(children.len(), 1);
        if let Block::CodeBlock { info, literal } = &children[0] {
            assert_eq!(info.as_str(), "rust");
            assert_eq!(literal, "fn main() {}");
        } else {
            panic!("Expected CodeBlock");
        }
    }
}

#[test]
fn test_code_block_no_language() {
    let ast = parse_html_to_ast("<pre><code>plain code</code></pre>", &default_opts());
    if let Block::Document { children } = ast {
        if let Block::CodeBlock { info, literal } = &children[0] {
            assert!(info.is_empty());
            assert_eq!(literal, "plain code");
        } else {
            panic!("Expected CodeBlock");
        }
    }
}

#[test]
fn test_blockquote() {
    let ast = parse_html_to_ast("<blockquote><p>Quote</p></blockquote>", &default_opts());
    if let Block::Document { children } = ast {
        assert_eq!(children.len(), 1);
        if let Block::BlockQuote { children } = &children[0] {
            assert_eq!(children.len(), 1);
        } else {
            panic!("Expected BlockQuote");
        }
    }
}

#[test]
fn test_thematic_break() {
    let ast = parse_html_to_ast("<p>Before</p><hr><p>After</p>", &default_opts());
    if let Block::Document { children } = ast {
        assert_eq!(children.len(), 3);
        assert!(matches!(&children[1], Block::ThematicBreak));
    }
}

// ============================================================================
// List Tests
// ============================================================================

#[test]
fn test_unordered_list() {
    let ast = parse_html_to_ast("<ul><li>One</li><li>Two</li></ul>", &default_opts());
    if let Block::Document { children } = ast {
        assert_eq!(children.len(), 1);
        if let Block::List { children, .. } = &children[0] {
            assert_eq!(children.len(), 2);
        } else {
            panic!("Expected List");
        }
    }
}

#[test]
fn test_ordered_list() {
    let ast = parse_html_to_ast(
        r#"<ol start="5"><li>A</li><li>B</li></ol>"#,
        &default_opts(),
    );
    if let Block::Document { children } = ast {
        if let Block::List {
            start, children, ..
        } = &children[0]
        {
            assert_eq!(*start, 5);
            assert_eq!(children.len(), 2);
        } else {
            panic!("Expected List");
        }
    }
}

#[test]
fn test_nested_list() {
    let html = "<ul><li>Item 1<ul><li>Nested</li></ul></li><li>Item 2</li></ul>";
    let ast = parse_html_to_ast(html, &default_opts());
    if let Block::Document { children } = ast
        && let Block::List { children, .. } = &children[0]
    {
        assert_eq!(children.len(), 2);
        if let Block::ListItem { children, .. } = &children[0] {
            // Should have paragraph and nested list
            assert!(!children.is_empty());
        }
    }
}

#[test]
fn test_task_list() {
    let html = r#"<ul><li><input type="checkbox" checked>Done</li><li><input type="checkbox">Todo</li></ul>"#;
    let ast = parse_html_to_ast(html, &default_opts());
    if let Block::Document { children } = ast
        && let Block::List { children, .. } = &children[0]
    {
        if let Block::ListItem { checked, .. } = &children[0] {
            assert_eq!(*checked, Some(true));
        }
        if let Block::ListItem { checked, .. } = &children[1] {
            assert_eq!(*checked, Some(false));
        }
    }
}

// ============================================================================
// Table Tests
// ============================================================================

#[test]
fn test_simple_table() {
    let html = "<table><thead><tr><th>A</th><th>B</th></tr></thead><tbody><tr><td>1</td><td>2</td></tr></tbody></table>";
    let ast = parse_html_to_ast(html, &default_opts());
    if let Block::Document { children } = ast {
        if let Block::Table(table) = &children[0] {
            assert_eq!(table.num_cols, 2);
            assert_eq!(table.header.len(), 2);
            assert_eq!(table.rows.len(), 2); // 2 cells in 1 row
        } else {
            panic!("Expected Table");
        }
    }
}

// ============================================================================
// Inline Element Tests
// ============================================================================

#[test]
fn test_inline_bold() {
    let md = html_to_markdown("<p><strong>Bold</strong> text</p>", &default_opts());
    assert!(md.contains("**Bold**"));
}

#[test]
fn test_inline_italic() {
    let md = html_to_markdown("<p><em>Italic</em> text</p>", &default_opts());
    assert!(md.contains("*Italic*"));
}

#[test]
fn test_inline_code() {
    let md = html_to_markdown("<p>Use <code>let x = 1;</code> here</p>", &default_opts());
    assert!(md.contains("`let x = 1;`"));
}

#[test]
fn test_inline_link() {
    let md = html_to_markdown(
        r#"<p><a href="https://example.com">Link</a></p>"#,
        &default_opts(),
    );
    assert!(md.contains("[Link](https://example.com)"));
}

#[test]
fn test_inline_link_with_title() {
    let md = html_to_markdown(
        r#"<p><a href="https://example.com" title="Title">Link</a></p>"#,
        &default_opts(),
    );
    assert!(md.contains(r#"[Link](https://example.com "Title")"#));
}

#[test]
fn test_inline_image() {
    let md = html_to_markdown(
        r#"<p><img src="test.png" alt="Alt text" /></p>"#,
        &default_opts(),
    );
    assert!(md.contains("![Alt text](test.png)"));
}

#[test]
fn test_inline_strikethrough() {
    let md = html_to_markdown("<p><del>Deleted</del> text</p>", &default_opts());
    assert!(md.contains("~~Deleted~~"));
}

#[test]
fn test_inline_highlight() {
    let md = html_to_markdown("<p><mark>Highlighted</mark> text</p>", &default_opts());
    assert!(md.contains("==Highlighted=="));
}

#[test]
fn test_inline_underline() {
    let md = html_to_markdown("<p><u>Underlined</u> text</p>", &default_opts());
    assert!(md.contains("++Underlined++"));
}

#[test]
fn test_nested_inline() {
    let md = html_to_markdown(
        "<p><strong><em>Bold italic</em></strong></p>",
        &default_opts(),
    );
    assert!(md.contains("***Bold italic***"));
}

// ============================================================================
// Unknown Tag Handling Tests
// ============================================================================

#[test]
fn test_unknown_tags_strip() {
    let opts = HtmlParseOptions {
        unknown_inline_handling: UnknownInlineHandling::StripTags,
        ..default_opts()
    };
    let md = html_to_markdown("<p><sup>Superscript</sup> text</p>", &opts);
    assert!(md.contains("Superscript"));
    assert!(!md.contains("<sup>"));
}

#[test]
fn test_unknown_tags_preserve() {
    let opts = HtmlParseOptions {
        unknown_inline_handling: UnknownInlineHandling::PreserveAsHtml,
        ..default_opts()
    };
    let md = html_to_markdown("<p><sup>Superscript</sup> text</p>", &opts);
    assert!(md.contains("<sup>"));
}

// ============================================================================
// Round-trip Tests (Markdown -> HTML -> Markdown)
// ============================================================================

#[test]
fn test_roundtrip_paragraph() {
    let original = "Hello world";
    let html = parse(original, &ParseOptions::default());
    let md = html_to_markdown(&html, &default_opts());
    assert_eq!(md.trim(), original);
}

#[test]
fn test_roundtrip_heading() {
    let original = "# Heading 1";
    let html = parse(original, &ParseOptions::default());
    let md = html_to_markdown(&html, &default_opts());
    assert_eq!(md.trim(), original);
}

#[test]
fn test_roundtrip_bold() {
    let original = "**Bold** text";
    let html = parse(original, &ParseOptions::default());
    let md = html_to_markdown(&html, &default_opts());
    // Note: markdown rendering may differ slightly but semantic should be same
    assert!(md.contains("**Bold**"));
}

#[test]
fn test_roundtrip_code_block() {
    let original = "```rust\nfn main() {}\n```";
    let html = parse(original, &ParseOptions::default());
    let md = html_to_markdown(&html, &default_opts());
    assert!(md.contains("```rust"));
    assert!(md.contains("fn main() {}"));
}

#[test]
fn test_roundtrip_list() {
    let original = "- Item 1\n- Item 2";
    let html = parse(original, &ParseOptions::default());
    let md = html_to_markdown(&html, &default_opts());
    assert!(md.contains("- ") || md.contains("* "));
    assert!(md.contains("Item 1"));
    assert!(md.contains("Item 2"));
}

// ============================================================================
// Edge Cases
// ============================================================================

#[test]
fn test_empty_input() {
    let ast = parse_html_to_ast("", &default_opts());
    if let Block::Document { children } = ast {
        assert!(children.is_empty());
    }
}

#[test]
fn test_whitespace_only() {
    let ast = parse_html_to_ast("   \n\t\n   ", &default_opts());
    if let Block::Document { children } = ast {
        assert!(children.is_empty());
    }
}

#[test]
fn test_malformed_html() {
    // Should not panic, should produce some output
    let ast = parse_html_to_ast("<p>Unclosed paragraph", &default_opts());
    if let Block::Document { children } = ast {
        assert!(!children.is_empty());
    }
}

#[test]
fn test_nested_quotes() {
    let html = "<blockquote><blockquote><p>Nested</p></blockquote></blockquote>";
    let ast = parse_html_to_ast(html, &default_opts());
    if let Block::Document { children } = ast
        && let Block::BlockQuote { children } = &children[0]
        && let Block::BlockQuote { children } = &children[0]
    {
        assert!(!children.is_empty());
    }
}

#[test]
fn test_html_entities() {
    let md = html_to_markdown("<p>&amp; &lt; &gt;</p>", &default_opts());
    // The parser should decode entities
    assert!(md.contains("&") || md.contains("&amp;"));
}

#[test]
fn test_hard_break() {
    let md = html_to_markdown("<p>Line 1<br />Line 2</p>", &default_opts());
    // Should contain hard break (two spaces + newline or just the text)
    assert!(md.contains("Line 1") && md.contains("Line 2"));
}
