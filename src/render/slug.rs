/// Generate a URL-safe slug from heading raw markdown text.
/// Strips markdown syntax, lowercases, replaces spaces/hyphens/dots with `-`.
/// Uses inline buffer for short slugs to avoid heap allocation.
pub(crate) fn heading_slug_into(slug: &mut String, raw: &str) {
    let bytes = raw.as_bytes();
    let len = bytes.len();
    slug.clear();

    // Fast path: check if heading is already slug-safe (common for simple headings).
    // A slug-safe heading has only ASCII alphanumerics and dashes (no leading/trailing dash).
    if len <= 64 {
        let mut is_safe = true;
        let mut has_content = false;
        for (idx, &b) in bytes.iter().enumerate() {
            if b.is_ascii_alphanumeric() {
                has_content = true;
            } else if b == b'-' {
                // Dash is ok if not leading/trailing and has content
                if idx == 0 || idx == len - 1 || !has_content {
                    is_safe = false;
                    break;
                }
            } else {
                is_safe = false;
                break;
            }
        }
        if is_safe && has_content {
            // Still need to lowercase
            let mut all_lower = true;
            for &b in bytes {
                if b.is_ascii_uppercase() {
                    all_lower = false;
                    break;
                }
            }
            if all_lower {
                slug.push_str(raw);
                return;
            }
            slug.push_str(raw);
            // SAFETY: lowercasing ASCII preserves UTF-8 validity
            unsafe {
                slug.as_mut_vec()
                    .iter_mut()
                    .for_each(|b| *b = b.to_ascii_lowercase())
            };
            return;
        }
    }

    // Slow path: process character by character
    slug.reserve(len.saturating_sub(slug.capacity()));
    let mut i = 0;
    let mut prev_dash = true; // start true to avoid leading dash

    while i < len {
        let b = bytes[i];
        match b {
            b'*' | b'_' | b'~' | b'=' | b'+' | b'`' => {
                i += 1;
            }
            b'<' => {
                if let Some(close) = memchr::memchr(b'>', &bytes[i..]) {
                    i += close + 1;
                } else {
                    i += 1;
                }
            }
            b'\\' => {
                i += 1;
            }
            b'[' | b']' | b'!' | b'(' | b')' => {
                i += 1;
            }
            // Spaces, hyphens, dots → single dash separator
            b' ' | b'\t' | b'-' | b'.' => {
                if !prev_dash && !slug.is_empty() {
                    slug.push('-');
                    prev_dash = true;
                }
                i += 1;
            }
            // ASCII alphanumeric → lowercase
            b if b.is_ascii_alphanumeric() => {
                slug.push(b.to_ascii_lowercase() as char);
                prev_dash = false;
                i += 1;
            }
            // Multi-byte UTF-8
            b if b >= 0x80 => {
                let char_len = crate::utf8_char_len(b);
                // SAFETY: We've verified b >= 0x80 indicating a multi-byte sequence.
                // The input `raw` is valid UTF-8, so the char boundary at `i` is valid.
                let c = unsafe { raw.get_unchecked(i..) }
                    .chars()
                    .next()
                    .unwrap_or(' ');
                if c.is_alphanumeric() {
                    for lc in c.to_lowercase() {
                        slug.push(lc);
                    }
                    prev_dash = false;
                } else {
                    // Non-alphanumeric Unicode → dash separator
                    if !prev_dash && !slug.is_empty() {
                        slug.push('-');
                        prev_dash = true;
                    }
                }
                i += char_len;
            }
            // Other ASCII punctuation → skip
            _ => {
                i += 1;
            }
        }
    }

    // Trim trailing dash
    while slug.ends_with('-') {
        slug.pop();
    }
}

pub fn benchmark_heading_slug(raw: &str) -> String {
    let mut slug = String::with_capacity(raw.len());
    heading_slug_into(&mut slug, raw);
    slug
}
