use logos::Span;


#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RawSpan {
    pub start: u32,
    pub end: u32,
}

impl From<Span> for RawSpan {
    fn from(s: Span) -> Self {
        Self {
            start: s.start as u32,
            end: s.end as u32,
        }
    }
}
