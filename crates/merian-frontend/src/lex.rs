// basically, a lexer is dum. It should be dum.

use logos::{Lexer, Logos, Span};

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

#[derive(Debug, Clone, PartialEq)]
pub struct SpannedToken<'s> {
    pub token: Token<'s>,
    pub span: RawSpan,
}

#[derive(Logos, Debug, PartialEq, Clone)]
pub enum Token<'s> {
    #[token("**")]
    StarStar,

    #[token("__", priority = 4)]
    UnderUnder,

    #[token("/'")]
    CommentOpen,

    #[token("'/")]
    CommentClose,

    #[token("@?")]
    AtQuestion,

    #[token("@")]
    At,

    #[token("!")]
    Bang,

    #[token("&")]
    Amp,

    #[token("#")]
    Hash,

    #[token("-")]
    Minus,

    #[token("+")]
    Plus,

    #[token("(")]
    LParen,

    #[token(")")]
    RParen,

    #[token(".")]
    Period,

    #[token(",")]
    Comma,

    #[token("?")]
    Question,

    #[token("'")]
    Apostrophe,

    #[token("`")]
    Backtick,

    #[regex(r"[0-9]+")]
    Number(&'s str),

    #[regex(r"[A-Za-z][A-Za-z0-9]*(_[A-Za-z0-9]+)*")]
    Text(&'s str),
    
    #[regex(r" ")]
    Whitespace,

    #[regex(r"\t")]
    Tab,

    #[regex(r"[\r\n|\n]")]
    Newline,
    Error(String),
}

/// Lexes an entire string into a Vector of tokens and spans.
/// If an error occurs, it returns the first error found.
pub fn lex_all<'s>(source: &'s str) -> Vec<SpannedToken<'s>> {
    let mut lexer = Token::lexer(source);
    let mut tokens = Vec::new();

    while let Some(result) = lexer.next() {
        let span = RawSpan::from(lexer.span());
        match result {
            Ok(token) => tokens.push(SpannedToken { token, span }),
            Err(e) => tokens.push(SpannedToken{token: Token::Error("".into()), span}),
        }
    }
    tokens
}

#[cfg(test)]
mod test {
    use crate::lex::lex_all;

    #[test]
    fn simple() {
        let inp = r"#hello __from__ **the** _*other*_ side. 
            yipee";

        lex_all(inp)
            .iter()
            .for_each(|c| println!("{:?}\t{:?}", c.token, c.span))
    }
}
