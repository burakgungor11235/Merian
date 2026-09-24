use logos::Span;

use super::MathMode;

#[derive(Debug, Clone, PartialEq)]
pub struct IrMathSource {
    pub mode: MathMode,
    pub raw: String,
    pub span: Span,
    pub payload_span: Span,
    pub line: u32,
}
