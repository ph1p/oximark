// ── ANSI escape sequences ─────────────────────────────────────────────────────

pub(super) const RESET: &str = "\x1b[0m";
pub(super) const BOLD: &str = "\x1b[1m";
pub(super) const ITALIC: &str = "\x1b[3m";
pub(super) const UNDERLINE: &str = "\x1b[4m";
pub(super) const STRIKETHROUGH: &str = "\x1b[9m";

// Inline formatting colours
pub(super) const FG_STRONG: &str = "\x1b[38;5;231m"; // #FFFFFF bright white — bold stands out clean
pub(super) const FG_ITALIC: &str = "\x1b[38;5;195m"; // #D7FFFF icy light cyan — distinct from body
pub(super) const FG_DEL: &str = "\x1b[38;5;131m"; // #AF5F5F muted red — "deleted" reads as error/removal
pub(super) const FG_UNDERLINE: &str = "\x1b[38;5;153m"; // #AFDFD7 pale teal — distinct from link blue

// Heading colours — warm-to-cool hierarchy (256-colour, dark-terminal-first)
pub(super) const FG_H1: &str = "\x1b[38;5;222m"; // #FFD787 warm gold
pub(super) const FG_H2: &str = "\x1b[38;5;116m"; // #87D7D7 steel cyan
pub(super) const FG_H3: &str = "\x1b[38;5;150m"; // #AFD787 sage green
pub(super) const FG_H4: &str = "\x1b[38;5;183m"; // #D7AFFF muted lavender
pub(super) const FG_H5: &str = "\x1b[38;5;110m"; // #87AFD7 slate blue
pub(super) const FG_H6: &str = "\x1b[38;5;247m"; // #9E9E9E medium grey

// Code blocks (fenced/indented) — light cyan text, bracket border
pub(super) const FG_CODE: &str = "\x1b[38;5;159m"; // #AFFFFF light cyan
pub(super) const FG_LANG: &str = "\x1b[38;5;150m"; // #AFD787 sage — language label in border

// Inline code — more visible bg, warm amber text to distinguish from block code
pub(super) const BG_INLINE_CODE: &str = "\x1b[48;5;237m"; // #3A3A3A clearly-visible dark slab
pub(super) const FG_INLINE_CODE: &str = "\x1b[38;5;215m"; // #FFAF5F warm amber/orange

// Structure / chrome
pub(super) const FG_BORDER: &str = "\x1b[38;5;238m"; // #444444 box/table borders
pub(super) const FG_DIM_TEXT: &str = "\x1b[38;5;244m"; // #808080 blockquote body text
pub(super) const BG_ZEBRA: &str = "\x1b[48;5;234m"; // #1C1C1C barely-perceptible stripe
/// Pre-built replacement: RESET + FG_DIM_TEXT — used when re-colouring blockquote lines.
pub(super) const RESET_DIM: &str = "\x1b[0m\x1b[38;5;244m";
pub(super) const FG_QUOTE_BAR: &str = "\x1b[38;5;74m"; // #5FAFD7 blockquote bar
pub(super) const FG_RULE: &str = "\x1b[38;5;240m"; // #585858 thematic break

// Content accents
pub(super) const FG_BULLET: &str = "\x1b[38;5;179m"; // #D7AF5F warm amber bullets/numbers
pub(super) const FG_LINK: &str = "\x1b[38;5;75m"; // #5FAFFF bright link blue
pub(super) const FG_LINK_URL: &str = "\x1b[38;5;242m"; // #6C6C6C dim URL suffix
pub(super) const FG_IMAGE: &str = "\x1b[38;5;140m"; // #AF87D7 image label
pub(super) const FG_MATH: &str = "\x1b[38;5;213m"; // #FF87FF math spans
pub(super) const FG_CHECKED: &str = "\x1b[38;5;114m"; // #87D787 green checkmark
pub(super) const FG_UNCHECKED: &str = "\x1b[38;5;240m"; // #585858 grey empty box

// Mark/highlight — soft gold bg, near-black fg (avoids eye-burning basic yellow)
pub(super) const BG_MARK: &str = "\x1b[48;5;221m"; // #FFD75F
pub(super) const FG_MARK: &str = "\x1b[38;5;232m"; // #080808
