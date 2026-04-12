//! HTML tokenizer for parsing HTML into tokens.
//!
//! This tokenizer produces a stream of HTML tokens (start tags, end tags, text, etc.)
//! that the parser can consume to build the AST.

use std::borrow::Cow;

/// An HTML token produced by the tokenizer.
#[derive(Clone, Debug, PartialEq)]
pub enum HtmlToken<'a> {
    /// A start tag like `<p>` or `<a href="...">`.
    StartTag {
        /// Tag name (lowercase).
        name: Cow<'a, str>,
        /// Attribute name-value pairs.
        attrs: Vec<(Cow<'a, str>, Cow<'a, str>)>,
        /// Whether this is a self-closing tag like `<br />`.
        self_closing: bool,
    },
    /// An end tag like `</p>`.
    EndTag {
        /// Tag name (lowercase).
        name: Cow<'a, str>,
    },
    /// Text content between tags.
    Text(Cow<'a, str>),
    /// An HTML comment `<!-- ... -->`.
    Comment(Cow<'a, str>),
    /// A DOCTYPE declaration.
    Doctype(Cow<'a, str>),
}

/// HTML tokenizer that produces tokens from an HTML string.
pub struct HtmlTokenizer<'a> {
    input: &'a str,
    bytes: &'a [u8],
    pos: usize,
}

impl<'a> HtmlTokenizer<'a> {
    /// Create a new tokenizer for the given HTML input.
    pub fn new(input: &'a str) -> Self {
        Self {
            input,
            bytes: input.as_bytes(),
            pos: 0,
        }
    }

    /// Get the next token, or None if at end of input.
    pub fn next_token(&mut self) -> Option<HtmlToken<'a>> {
        if self.pos >= self.bytes.len() {
            return None;
        }

        if self.bytes[self.pos] == b'<' {
            self.parse_tag_or_comment()
        } else {
            self.parse_text()
        }
    }

    /// Parse text content until we hit a `<` or end of input.
    fn parse_text(&mut self) -> Option<HtmlToken<'a>> {
        let start = self.pos;

        while self.pos < self.bytes.len() && self.bytes[self.pos] != b'<' {
            self.pos += 1;
        }

        if self.pos > start {
            let text = &self.input[start..self.pos];
            // Decode HTML entities
            let decoded = decode_entities(text);
            Some(HtmlToken::Text(decoded))
        } else {
            None
        }
    }

    /// Parse a tag, comment, or DOCTYPE starting with `<`.
    fn parse_tag_or_comment(&mut self) -> Option<HtmlToken<'a>> {
        debug_assert_eq!(self.bytes[self.pos], b'<');
        self.pos += 1; // Skip '<'

        if self.pos >= self.bytes.len() {
            return Some(HtmlToken::Text(Cow::Borrowed("<")));
        }

        // Check for comment: <!--
        if self.bytes[self.pos..].starts_with(b"!--") {
            return self.parse_comment();
        }

        // Check for DOCTYPE: <!DOCTYPE
        if self.bytes[self.pos..].starts_with(b"!DOCTYPE")
            || self.bytes[self.pos..].starts_with(b"!doctype")
        {
            return self.parse_doctype();
        }

        // Check for CDATA (treat as text)
        if self.bytes[self.pos..].starts_with(b"![CDATA[") {
            return self.parse_cdata();
        }

        // Check for end tag: </
        if self.bytes[self.pos] == b'/' {
            return self.parse_end_tag();
        }

        // Start tag
        self.parse_start_tag()
    }

    /// Parse a comment `<!-- ... -->`.
    fn parse_comment(&mut self) -> Option<HtmlToken<'a>> {
        self.pos += 3; // Skip '!--'
        let start = self.pos;

        // Find closing -->
        while self.pos + 2 < self.bytes.len() {
            if &self.bytes[self.pos..self.pos + 3] == b"-->" {
                let comment = &self.input[start..self.pos];
                self.pos += 3; // Skip '-->'
                return Some(HtmlToken::Comment(Cow::Borrowed(comment)));
            }
            self.pos += 1;
        }

        // Unclosed comment - consume rest as comment
        self.pos = self.bytes.len();
        Some(HtmlToken::Comment(Cow::Borrowed(&self.input[start..])))
    }

    /// Parse a DOCTYPE declaration.
    fn parse_doctype(&mut self) -> Option<HtmlToken<'a>> {
        let start = self.pos - 1; // Include the '<'

        // Find closing >
        while self.pos < self.bytes.len() && self.bytes[self.pos] != b'>' {
            self.pos += 1;
        }

        if self.pos < self.bytes.len() {
            self.pos += 1; // Skip '>'
        }

        Some(HtmlToken::Doctype(Cow::Borrowed(
            &self.input[start..self.pos],
        )))
    }

    /// Parse a CDATA section as text.
    fn parse_cdata(&mut self) -> Option<HtmlToken<'a>> {
        self.pos += 8; // Skip '![CDATA['
        let start = self.pos;

        // Find closing ]]>
        while self.pos + 2 < self.bytes.len() {
            if &self.bytes[self.pos..self.pos + 3] == b"]]>" {
                let text = &self.input[start..self.pos];
                self.pos += 3; // Skip ']]>'
                return Some(HtmlToken::Text(Cow::Borrowed(text)));
            }
            self.pos += 1;
        }

        // Unclosed CDATA
        self.pos = self.bytes.len();
        Some(HtmlToken::Text(Cow::Borrowed(&self.input[start..])))
    }

    /// Parse an end tag `</name>`.
    fn parse_end_tag(&mut self) -> Option<HtmlToken<'a>> {
        self.pos += 1; // Skip '/'

        self.skip_whitespace();
        let name = self.parse_tag_name();

        if name.is_empty() {
            // Invalid end tag, treat as text
            return Some(HtmlToken::Text(Cow::Borrowed("</")));
        }

        self.skip_whitespace();

        // Skip to closing >
        while self.pos < self.bytes.len() && self.bytes[self.pos] != b'>' {
            self.pos += 1;
        }

        if self.pos < self.bytes.len() {
            self.pos += 1; // Skip '>'
        }

        Some(HtmlToken::EndTag {
            name: Cow::Owned(name.to_ascii_lowercase()),
        })
    }

    /// Parse a start tag `<name ...>` or `<name ... />`.
    fn parse_start_tag(&mut self) -> Option<HtmlToken<'a>> {
        self.skip_whitespace();
        let name = self.parse_tag_name();

        if name.is_empty() {
            // Invalid tag, treat as text
            return Some(HtmlToken::Text(Cow::Borrowed("<")));
        }

        let mut attrs = Vec::new();
        let mut self_closing = false;

        // Parse attributes
        loop {
            self.skip_whitespace();

            if self.pos >= self.bytes.len() {
                break;
            }

            let b = self.bytes[self.pos];

            if b == b'>' {
                self.pos += 1;
                break;
            }

            if b == b'/' {
                self.pos += 1;
                self.skip_whitespace();
                if self.pos < self.bytes.len() && self.bytes[self.pos] == b'>' {
                    self.pos += 1;
                    self_closing = true;
                }
                break;
            }

            // Parse attribute
            if let Some((attr_name, attr_value)) = self.parse_attribute() {
                attrs.push((attr_name, attr_value));
            } else {
                // Skip invalid character
                self.pos += 1;
            }
        }

        // Void elements are always self-closing
        if is_void_element(&name) {
            self_closing = true;
        }

        Some(HtmlToken::StartTag {
            name: Cow::Owned(name.to_ascii_lowercase()),
            attrs,
            self_closing,
        })
    }

    /// Parse a tag name.
    fn parse_tag_name(&mut self) -> String {
        let start = self.pos;

        while self.pos < self.bytes.len() {
            let b = self.bytes[self.pos];
            if b.is_ascii_alphanumeric() || b == b'-' || b == b'_' || b == b':' {
                self.pos += 1;
            } else {
                break;
            }
        }

        self.input[start..self.pos].to_string()
    }

    /// Parse an attribute `name="value"` or `name='value'` or `name=value` or `name`.
    fn parse_attribute(&mut self) -> Option<(Cow<'a, str>, Cow<'a, str>)> {
        let name_start = self.pos;

        // Parse attribute name
        while self.pos < self.bytes.len() {
            let b = self.bytes[self.pos];
            if b.is_ascii_alphanumeric()
                || b == b'-'
                || b == b'_'
                || b == b':'
                || b == b'.'
                || b == b'@'
            {
                self.pos += 1;
            } else {
                break;
            }
        }

        if self.pos == name_start {
            return None;
        }

        let name = &self.input[name_start..self.pos];

        self.skip_whitespace();

        // Check for =
        if self.pos >= self.bytes.len() || self.bytes[self.pos] != b'=' {
            // Boolean attribute (no value)
            return Some((Cow::Owned(name.to_ascii_lowercase()), Cow::Borrowed("")));
        }

        self.pos += 1; // Skip '='
        self.skip_whitespace();

        // Parse value
        let value = if self.pos < self.bytes.len() {
            let quote = self.bytes[self.pos];
            if quote == b'"' || quote == b'\'' {
                self.pos += 1;
                let value_start = self.pos;

                while self.pos < self.bytes.len() && self.bytes[self.pos] != quote {
                    self.pos += 1;
                }

                let value = &self.input[value_start..self.pos];

                if self.pos < self.bytes.len() {
                    self.pos += 1; // Skip closing quote
                }

                decode_entities(value)
            } else {
                // Unquoted value
                let value_start = self.pos;
                while self.pos < self.bytes.len() {
                    let b = self.bytes[self.pos];
                    if b.is_ascii_whitespace() || b == b'>' || b == b'/' {
                        break;
                    }
                    self.pos += 1;
                }
                let value = &self.input[value_start..self.pos];
                decode_entities(value)
            }
        } else {
            Cow::Borrowed("")
        };

        Some((Cow::Owned(name.to_ascii_lowercase()), value))
    }

    /// Skip whitespace characters.
    fn skip_whitespace(&mut self) {
        while self.pos < self.bytes.len() && self.bytes[self.pos].is_ascii_whitespace() {
            self.pos += 1;
        }
    }
}

/// Check if a tag name is a void element (self-closing).
fn is_void_element(name: &str) -> bool {
    matches!(
        name.to_ascii_lowercase().as_str(),
        "area"
            | "base"
            | "br"
            | "col"
            | "embed"
            | "hr"
            | "img"
            | "input"
            | "link"
            | "meta"
            | "param"
            | "source"
            | "track"
            | "wbr"
    )
}

/// Decode HTML entities in a string.
fn decode_entities(s: &str) -> Cow<'_, str> {
    if !s.contains('&') {
        return Cow::Borrowed(s);
    }

    let mut result = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();

    while let Some(c) = chars.next() {
        if c == '&' {
            let mut entity = String::new();
            let mut found_semi = false;

            for ch in chars.by_ref() {
                if ch == ';' {
                    found_semi = true;
                    break;
                }
                if entity.len() > 10 || (!ch.is_ascii_alphanumeric() && ch != '#') {
                    // Not a valid entity
                    break;
                }
                entity.push(ch);
            }

            if found_semi && let Some(decoded) = decode_entity(&entity) {
                result.push(decoded);
                continue;
            }

            // Not a valid entity, output as-is
            result.push('&');
            result.push_str(&entity);
            if found_semi {
                result.push(';');
            }
        } else {
            result.push(c);
        }
    }

    Cow::Owned(result)
}

/// Decode a single HTML entity (without the & and ;).
fn decode_entity(entity: &str) -> Option<char> {
    // Numeric entities
    if let Some(rest) = entity.strip_prefix('#') {
        let codepoint = if let Some(hex) = rest.strip_prefix('x').or_else(|| rest.strip_prefix('X'))
        {
            u32::from_str_radix(hex, 16).ok()?
        } else {
            rest.parse::<u32>().ok()?
        };
        return char::from_u32(codepoint);
    }

    // Named entities (common ones)
    Some(match entity {
        "amp" => '&',
        "lt" => '<',
        "gt" => '>',
        "quot" => '"',
        "apos" => '\'',
        "nbsp" => '\u{00A0}',
        "copy" => '\u{00A9}',
        "reg" => '\u{00AE}',
        "trade" => '\u{2122}',
        "mdash" => '\u{2014}',
        "ndash" => '\u{2013}',
        "ldquo" => '\u{201C}',
        "rdquo" => '\u{201D}',
        "lsquo" => '\u{2018}',
        "rsquo" => '\u{2019}',
        "hellip" => '\u{2026}',
        "bull" => '\u{2022}',
        "middot" => '\u{00B7}',
        "laquo" => '\u{00AB}',
        "raquo" => '\u{00BB}',
        "euro" => '\u{20AC}',
        "pound" => '\u{00A3}',
        "yen" => '\u{00A5}',
        "cent" => '\u{00A2}',
        "deg" => '\u{00B0}',
        "plusmn" => '\u{00B1}',
        "times" => '\u{00D7}',
        "divide" => '\u{00F7}',
        "frac12" => '\u{00BD}',
        "frac14" => '\u{00BC}',
        "frac34" => '\u{00BE}',
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tokenize(html: &str) -> Vec<HtmlToken<'_>> {
        let mut tokenizer = HtmlTokenizer::new(html);
        let mut tokens = Vec::new();
        while let Some(token) = tokenizer.next_token() {
            tokens.push(token);
        }
        tokens
    }

    #[test]
    fn test_simple_tag() {
        let tokens = tokenize("<p>Hello</p>");
        assert_eq!(tokens.len(), 3);
        assert!(matches!(&tokens[0], HtmlToken::StartTag { name, .. } if name == "p"));
        assert!(matches!(&tokens[1], HtmlToken::Text(t) if t == "Hello"));
        assert!(matches!(&tokens[2], HtmlToken::EndTag { name } if name == "p"));
    }

    #[test]
    fn test_attributes() {
        let tokens = tokenize(r#"<a href="https://example.com" title='Test'>Link</a>"#);
        assert_eq!(tokens.len(), 3);
        if let HtmlToken::StartTag { name, attrs, .. } = &tokens[0] {
            assert_eq!(name, "a");
            assert_eq!(attrs.len(), 2);
            assert_eq!(attrs[0].0, "href");
            assert_eq!(attrs[0].1, "https://example.com");
            assert_eq!(attrs[1].0, "title");
            assert_eq!(attrs[1].1, "Test");
        } else {
            panic!("Expected StartTag");
        }
    }

    #[test]
    fn test_self_closing() {
        let tokens = tokenize("<br /><hr><img src='test.png' />");
        assert_eq!(tokens.len(), 3);
        assert!(
            matches!(&tokens[0], HtmlToken::StartTag { name, self_closing, .. } if name == "br" && *self_closing)
        );
        assert!(
            matches!(&tokens[1], HtmlToken::StartTag { name, self_closing, .. } if name == "hr" && *self_closing)
        );
        assert!(
            matches!(&tokens[2], HtmlToken::StartTag { name, self_closing, .. } if name == "img" && *self_closing)
        );
    }

    #[test]
    fn test_comment() {
        let tokens = tokenize("<!-- This is a comment --><p>Text</p>");
        assert_eq!(tokens.len(), 4);
        assert!(matches!(&tokens[0], HtmlToken::Comment(c) if c == " This is a comment "));
    }

    #[test]
    fn test_entities() {
        let tokens = tokenize("<p>&amp; &lt; &gt; &quot;</p>");
        if let HtmlToken::Text(t) = &tokens[1] {
            assert_eq!(t, "& < > \"");
        } else {
            panic!("Expected Text");
        }
    }

    #[test]
    fn test_nested_tags() {
        // <div>, <p>, "Hello ", <strong>, "world", </strong>, </p>, </div> = 8 tokens
        let tokens = tokenize("<div><p>Hello <strong>world</strong></p></div>");
        assert_eq!(tokens.len(), 8);
    }
}
