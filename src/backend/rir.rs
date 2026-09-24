use crate::math::rir::RMathResult;

use logos::Span;
/// Resolved IR: same shape as [`IrDoc`],
/// but every detail a renderer needs or the query language needs is baked in.
/// This is the canonical output of Merian, everything that comes after it is up to your preference.
/// The world is your oyster and I like eating oysters so do whatever you want to with this info.
#[derive(Debug, Clone, PartialEq)]
pub struct ResolvedDoc {
    pub title: String,
    pub chunks: Vec<RChunk>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RChunk {
    pub id: usize,
    pub kind: RBlock,
}

#[derive(Debug, Clone, PartialEq)]
pub enum RBlock {
    Heading {
        level: u8,
        permalink: String,
        inlines: Vec<RInline>,
    },
    Paragraph(Vec<RInline>),
    Quote {
        level: u8,
        body: Vec<RBlock>,
    },
    Code {
        lang: String,
        title: Option<String>,
        src: String,
    },
    List {
        items: Vec<RListItem>,
    },
    Error(RError),
    Rule,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RListItem {
    pub depth: usize,
    pub ordered: bool,
    pub marker: String,
    pub body: Vec<RBlock>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum RInline {
    Text(String),
    Bold(Vec<RInline>),
    Italic(Vec<RInline>),
    Underline(Vec<RInline>),
    Strike(Vec<RInline>),
    Code { src: String, lang: Option<String> },
    Link { url: String, text: String },
    Image { src: String, alt: String },
    Math(RMathResult),
    Error(RError),
    Ref { target: usize, exists: bool },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RErrorKind {
    Syntax,
    Unsupported,
    Semantic,
    Backend,
    Document,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorRenderStrategy {
    InlineMessage,
    BlockMessage,
    SourceFallback,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RError {
    pub kind: RErrorKind,
    pub code: String,
    pub message: String,
    pub span: Option<Span>,
    pub source: Option<String>,
    pub render: ErrorRenderStrategy,
}
