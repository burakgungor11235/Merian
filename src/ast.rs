use std::fmt::{self, Display};

#[derive(Debug)]
pub struct Document<'a> {
    pub meta: DocumentMetadata,
    pub blocks: Vec<Block<'a>>,
}

#[derive(Debug)]
pub struct DocumentMetadata {}

#[derive(Debug)]
pub enum Block<'a> {
    Heading {
        level: usize,
        content: Vec<Inline<'a>>,
    },
    Paragraph(Vec<Inline<'a>>),
    Quote {
        level: i32,
        content: Vec<Block<'a>>,
    },

    #[allow(clippy::enum_variant_names)]
    CodeBlock {
        lang: &'a str,
        title: Option<&'a str>,
        content: &'a str,
    },
    List {
        items: Vec<ListItem<'a>>,
    },
    ThematicBreak,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AutoSymbol {
    Numeric,    // '+'
    LowerAlpha, // '-'
    UpperAlpha, // '^'
    Roman,      // '='
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ListMarker {
    Bullet,                        // '*'
    Ordered(usize),                // '1.', '2.', etc.
    Auto(Vec<AutoSymbol>, String), // auto symbols + formatted label
}

impl AutoSymbol {
    pub fn from_char(c: char) -> Option<Self> {
        match c {
            '+' => Some(Self::Numeric),
            '-' => Some(Self::LowerAlpha),
            '^' => Some(Self::UpperAlpha),
            '=' => Some(Self::Roman),
            _ => None,
        }
    }
}
impl ListMarker {
    pub fn is_unordered(&self) -> bool {
        matches!(self, Self::Bullet)
    }

    #[allow(dead_code)] // for future use, probably...
    pub fn is_ordered(&self) -> bool {
        !self.is_unordered()
    }
}

impl Display for ListMarker {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ListMarker::Bullet => write!(f, "*"),
            ListMarker::Ordered(n) => write!(f, "{n}."),
            ListMarker::Auto(_, label) => write!(f, "{label}"),
        }
    }
}

#[derive(Debug)]
pub struct ListItem<'a> {
    pub depth: usize,
    pub marker: ListMarker,
    pub blocks: Vec<Block<'a>>,
}

#[derive(Debug)]
pub enum Inline<'a> {
    Text(&'a str),

    Bold(Vec<Inline<'a>>),
    Italic(Vec<Inline<'a>>),
    Strikethru(Vec<Inline<'a>>),
    Underline(Vec<Inline<'a>>),
    Code {
        content: &'a str,
        lang: Option<&'a str>,
    },
}
