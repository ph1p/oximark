use compact_str::CompactString;

#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Block {
    Document {
        children: Vec<Block>,
    },
    BlockQuote {
        children: Vec<Block>,
    },
    List {
        kind: ListKind,
        start: u32,
        tight: bool,
        children: Vec<Block>,
    },
    ListItem {
        children: Vec<Block>,
        checked: Option<bool>,
    },
    Paragraph {
        raw: String,
    },
    Heading {
        level: u8,
        raw: String,
    },
    CodeBlock {
        info: CompactString,
        literal: String,
    },
    HtmlBlock {
        literal: String,
    },
    ThematicBreak,
    Table(Box<TableData>),
}

#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct TableData {
    pub alignments: Vec<TableAlignment>,
    pub num_cols: usize,
    pub header: Vec<CompactString>, // len == num_cols
    pub rows: Vec<CompactString>,   // flat row-major, len == num_rows * num_cols
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum ListKind {
    Bullet(u8),  // marker character: b'-', b'*', b'+'
    Ordered(u8), // delimiter: b'.' or b')'
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum TableAlignment {
    None,
    Left,
    Center,
    Right,
}
