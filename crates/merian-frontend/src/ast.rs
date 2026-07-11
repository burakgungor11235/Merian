// This ast will be FLAT.
// F don't
// L think
// A cronyms
// T matter

use logos::Span;

struct AST {
    context: String,
    blocks: Vec<Chunk>,
}

struct Chunk {
    place: Span,
    contents: Vec<Inline>,
    content_span: Vec<Span>,
}

#[repr(C)]
enum Inline {
    Text,
    Bold,
    Italic,
    BoldItalic,
    StrikeThru,
    Underline,
}
