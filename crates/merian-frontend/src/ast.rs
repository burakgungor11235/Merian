// This ast will be FLAT.
// F don't
// L think
// A cronyms
// T matter

use crate::lex::{SpannedToken, Token, chunkify, lex_all};
use merian_core::span::RawSpan;

// Abstract Syntax Pancake.
pub struct Asp {
    context: String,

    block_place: Vec<RawSpan>,
    block_type: Vec<BlockType>,

    block_run_start: Vec<u32>, // index into contents/content_span, inclusive
    block_run_end: Vec<u32>,   // exclusive

    contents: Vec<Inline>,
    content_span: Vec<RawSpan>,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Inline {
    Text,
    Bold,
    Italic,
    BoldItalic,
    StrikeThru,
    Underline,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlockType {
    Heading(u8),
    Paragraph,
    HRule,
    Comment,
    Unknown,
}

pub struct ChunkView<'a>(pub &'a [Inline], pub &'a [RawSpan]);

impl Asp {
    pub fn from_string(source: &str) -> Self {
        let mut ast = Asp {
            context: source.to_string(),
            block_place: Vec::new(),
            block_type: Vec::new(),
            block_run_start: Vec::new(),
            block_run_end: Vec::new(),
            contents: Vec::new(),
            content_span: Vec::new(),
        };

        for (chunk_place, raw) in chunkify(source) {
            if raw.trim().is_empty() {
                continue;
            }
            ast.push_chunk(chunk_place, raw);
        }

        ast
    }

    pub fn span_text(&self, span: RawSpan) -> &str {
        &self.context[span.start as usize..span.end as usize]
    }

    fn push_chunk(&mut self, place: RawSpan, slice: &str) {
        let toks = lex_all(slice);
        let base = place.start;

        let block_type = classify_block(&toks);

        let run_start = self.contents.len() as u32;
        let mut parser = Parser {
            tokens: &toks,
            pos: 0,
            base,
        };
        parser.parse_inlines(&mut self.contents, &mut self.content_span);
        let run_end = self.contents.len() as u32;

        self.block_place.push(place);
        self.block_type.push(block_type);
        self.block_run_start.push(run_start);
        self.block_run_end.push(run_end);
    }

    pub fn chunk_count(&self) -> usize {
        self.block_place.len()
    }

    pub fn chunkref_nth<'a>(&'a self, chunk: usize) -> ChunkView<'a> {
        let s = self.block_run_start[chunk] as usize;
        let e = self.block_run_end[chunk] as usize;
        ChunkView(&self.contents[s..e], &self.content_span[s..e])
    }

    pub fn block_type_of(&self, chunk: usize) -> BlockType {
        self.block_type[chunk]
    }

    pub fn place_of(&self, chunk: usize) -> RawSpan {
        self.block_place[chunk]
    }

    pub fn iter(&self) -> impl Iterator<Item = ChunkView<'_>> {
        (0..self.chunk_count()).map(|i| self.chunkref_nth(i))
    }
}

fn classify_block(toks: &[SpannedToken]) -> BlockType {
    let mut non_white = toks
        .iter()
        .filter(|t| !matches!(t.token, Token::Whitespace | Token::Tab | Token::Newline));

    match non_white.next().map(|t| &t.token) {
        Some(Token::Hash) => {
            let mut level: u8 = 0;
            for t in non_white {
                match t.token {
                    // fuckass parsing.
                    Token::Number(_) => level += 1,
                    Token::Period => continue,
                    _ => break,
                }
            }
            BlockType::Heading(level.max(1))
        }
        Some(Token::Minus) => {
            if non_white.next().map(|t| &t.token) == Some(&Token::Minus)
                && non_white.next().map(|t| &t.token) == Some(&Token::Minus)
            {
                BlockType::HRule
            } else {
                // no lists yet
                // TODO: Add proper lists
                BlockType::Paragraph
            }
        }
        Some(Token::CommentOpen) => BlockType::Comment,
        Some(_) => BlockType::Paragraph,
        None => BlockType::Unknown,
    }
    
}

pub struct Parser<'a> {
    tokens: &'a [SpannedToken<'a>],
    pos: usize,
    base: u32,
}

impl<'a> Parser<'a> {
    fn advance(&mut self) -> Option<&SpannedToken<'a>> {
        let tok = self.tokens.get(self.pos)?;
        self.pos += 1;
        Some(tok)
    }

    fn parse_inlines(&mut self, contents: &mut Vec<Inline>, spans: &mut Vec<RawSpan>) {
        let mut bold = false;
        let mut italic = false;
        let mut run_start: Option<u32> = None;
        let mut run_end: u32 = self.base;

        while let Some(t) = self.advance() {
            let token = t.token.clone();
            let span = t.span;
            match token {
                Token::StarStar => {
                    Self::flush(contents, spans, &mut run_start, run_end, bold, italic);
                    bold = !bold;
                }
                Token::UnderUnder => {
                    Self::flush(contents, spans, &mut run_start, run_end, bold, italic);
                    italic = !italic;
                }
                Token::Whitespace | Token::Tab | Token::Newline if run_start.is_none() => {}
                _ => {
                    if run_start.is_none() {
                        run_start = Some(self.base + span.start);
                    }
                    run_end = self.base + span.end;
                }
            }
        }
        Self::flush(contents, spans, &mut run_start, run_end, bold, italic);
    }

    /// the finisher.
    fn flush(
        contents: &mut Vec<Inline>,
        spans: &mut Vec<RawSpan>,
        run_start: &mut Option<u32>,
        run_end: u32,
        bold: bool,
        italic: bool,
    ) {
        if let Some(start) = run_start.take()
            && run_end > start
        {
            let style = match (bold, italic) {
                (true, true) => Inline::BoldItalic,
                (true, false) => Inline::Bold,
                (false, true) => Inline::Italic,
                (false, false) => Inline::Text,
            };
            contents.push(style);
            spans.push(RawSpan {
                start,
                end: run_end,
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::ast::ChunkView;

    use super::*;

    #[test]
    fn flat_runs_no_nesting() {
        let src = "plain **bold** and __italic__ and **__both__**";
        let ast = Asp::from_string(src);
        let ChunkView(contents, spans) = ast.chunkref_nth(0);
        assert_eq!(contents.len(), spans.len());
        for (style, span) in contents.iter().zip(spans) {
            println!(
                "{style:?}\t{:?}",
                &src[span.start as usize..span.end as usize]
            );
        }
        assert!(contents.contains(&Inline::Bold));
        assert!(contents.contains(&Inline::Italic));
        assert!(contents.contains(&Inline::BoldItalic));
    }

    #[test]
    fn heading_level_from_dots() {
        let src = "#1.2 a heading";
        let ast = Asp::from_string(src);
        assert_eq!(ast.block_type_of(0), BlockType::Heading(2));
    }

    #[test]
    fn runs_are_contiguous_across_chunks() {
        let src = "first para\n\nsecond para **bold**";
        let ast = Asp::from_string(src);
        assert_eq!(ast.chunk_count(), 2);
        let ChunkView(c0, _) = ast.chunkref_nth(0);
        let ChunkView(c1, s1) = ast.chunkref_nth(1);
        assert_eq!(c0.len() + c1.len(), ast.contents.len());
        let bold_span = s1[c1.iter().position(|i| *i == Inline::Bold).unwrap()];
        assert_eq!(
            &src[bold_span.start as usize..bold_span.end as usize],
            "bold"
        );
    }
}
