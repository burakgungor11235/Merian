use std::collections::BTreeSet;

use logos::{Logos, Span};

// I feel like this is going to go from CS101 to math55 real quick.
// It needs to be light and functional, I'm (hopefully) not trying to reinvent SymPy here.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinOp {
    Add,
    Sub,
    Mul,
    Div,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    Number(String),
    Ident(String),
    Binary {
        op: BinOp,
        lhs: Box<Expr>,
        rhs: Box<Expr>,
    },
    Neg(Box<Expr>),
    Paren(Box<Expr>),
}

impl Expr {
    pub fn free_vars(&self) -> BTreeSet<String> {
        let mut vars = BTreeSet::new();
        self.collect_vars(&mut vars);
        vars
    }

    fn collect_vars(&self, vars: &mut BTreeSet<String>) {
        match self {
            Expr::Number(_) => {}
            Expr::Ident(name) => {
                vars.insert(name.clone());
            }
            Expr::Binary { lhs, rhs, .. } => {
                lhs.collect_vars(vars);
                rhs.collect_vars(vars);
            }
            Expr::Neg(operand) | Expr::Paren(operand) => operand.collect_vars(vars),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ExprError {
    pub code: &'static str,
    pub message: String,
    pub offset: usize,
    pub len: usize,
}

impl ExprError {
    fn new(code: &'static str, message: String, offset: usize, len: usize) -> Self {
        Self {
            code,
            message,
            offset,
            len,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Statement {
    Expr(Expr),
    Binding {
        name: String,
        value: Expr,
    },
    /// a = b
    Relation {
        lhs: Expr,
        rhs: Expr,
    },
}

/// The CEO of lexing presents:
///         LEXING 2
#[derive(Logos, Debug, Clone, Copy, PartialEq)]
#[logos(skip r"\s+")]
enum MathToken<'a> {
    #[regex(r"[0-9.]+")]
    Number(&'a str),
    #[regex(r"[\p{Alphabetic}_][\p{Alphabetic}\p{N}_]*")] // I copy pasted this. I hope it works.
    Ident(&'a str),
    #[token("+")]
    Plus,
    #[token("-")]
    Minus,
    #[token("*")]
    Star,
    #[token("/")]
    Slash,
    #[token("(")]
    LParen,
    #[token(")")]
    RParen,
    #[token(":=")]
    ColonEq,
    #[token("=")]
    Eq,
    #[token(":")]
    Colon,
}

fn tokenize(source: &str) -> Result<Vec<(MathToken<'_>, Span)>, ExprError> {
    let mut lexer = MathToken::lexer(source);
    let mut tokens = Vec::new();

    while let Some(token) = lexer.next() {
        let span = lexer.span();
        let kind = match token {
            Ok(kind) => kind,
            Err(()) => return Err(unexpected_character(source, span)),
        };
        if let MathToken::Number(text) = kind {
            validate_number(text, span.start)?;
        }
        tokens.push((kind, span));
    }

    Ok(tokens)
}

fn unexpected_character(source: &str, span: Span) -> ExprError {
    let char = source[span.start..span.end].chars().next().unwrap_or('\0');
    ExprError::new(
        "unexpected-character",
        format!("unexpected character `{char}` in math expression"),
        span.start,
        char.len_utf8(),
    )
}

fn validate_number(text: &str, offset: usize) -> Result<(), ExprError> {
    let mut seen_dot = false;
    for (idx, char) in text.char_indices() {
        if char != '.' {
            continue;
        }
        if seen_dot {
            return Err(ExprError::new(
                "unexpected-character",
                "a number may contain at most one `.`".to_string(),
                offset + idx,
                1,
            ));
        }
        seen_dot = true;
    }

    if text == "." {
        return Err(ExprError::new(
            "unexpected-character",
            "expected a digit after `.`".to_string(),
            offset,
            1,
        ));
    }

    Ok(())
}

struct Parser<'a> {
    tokens: Vec<(MathToken<'a>, Span)>,
    pos: usize,
    /// Byte length of the source, the offset of end-of-input.
    end: usize,
}

impl<'a> Parser<'a> {
    fn current(&self) -> Option<(MathToken<'a>, Span)> {
        self.tokens.get(self.pos).cloned()
    }

    fn advance(&mut self) {
        self.pos += 1;
    }

    // expr := term (('+' | '-') term)*
    fn expression(&mut self) -> Result<Expr, ExprError> {
        let mut lhs = self.term()?;
        loop {
            let op = match self.current() {
                Some((MathToken::Plus, _)) => BinOp::Add,
                Some((MathToken::Minus, _)) => BinOp::Sub,
                _ => break,
            };
            self.advance();
            let rhs = self.term()?;
            lhs = Expr::Binary {
                op,
                lhs: Box::new(lhs),
                rhs: Box::new(rhs),
            };
        }
        Ok(lhs)
    }

    // term := unary (('*' | '/') unary)*
    fn term(&mut self) -> Result<Expr, ExprError> {
        let mut lhs = self.unary()?;
        loop {
            let op = match self.current() {
                Some((MathToken::Star, _)) => BinOp::Mul,
                Some((MathToken::Slash, _)) => BinOp::Div,
                _ => break,
            };
            self.advance();
            let rhs = self.unary()?;
            lhs = Expr::Binary {
                op,
                lhs: Box::new(lhs),
                rhs: Box::new(rhs),
            };
        }
        Ok(lhs)
    }

    // unary := '-' unary | primary
    fn unary(&mut self) -> Result<Expr, ExprError> {
        if matches!(self.current(), Some((MathToken::Minus, _))) {
            self.advance();
            return Ok(Expr::Neg(Box::new(self.unary()?)));
        }
        self.primary()
    }

    // primary := number | ident | '(' expression ')'
    fn primary(&mut self) -> Result<Expr, ExprError> {
        match self.current() {
            Some((MathToken::Number(value), _)) => {
                self.advance();
                Ok(Expr::Number(value.to_string()))
            }
            Some((MathToken::Ident(name), _)) => {
                self.advance();
                Ok(Expr::Ident(name.to_string()))
            }
            Some((MathToken::LParen, _)) => {
                self.advance();
                let inner = self.expression()?;
                match self.current() {
                    Some((MathToken::RParen, _)) => {
                        self.advance();
                        Ok(Expr::Paren(Box::new(inner)))
                    }
                    None => Err(ExprError::new(
                        "unexpected-eof",
                        "missing closing `)`".to_string(),
                        self.end,
                        0,
                    )),
                    Some((MathToken::ColonEq | MathToken::Eq, span)) => Err(ExprError::new(
                        "unsupported-statement",
                        "a `:=` binding or `=` relation must be at the top level of the envelope"
                            .to_string(),
                        span.start,
                        span.len(),
                    )),
                    Some((_, span)) => Err(ExprError::new(
                        "unexpected-token",
                        "expected `)`".to_string(),
                        span.start,
                        span.len(),
                    )),
                }
            }
            None => Err(ExprError::new(
                "unexpected-eof",
                "math expression ended unexpectedly".to_string(),
                self.end,
                0,
            )),
            Some((_, span)) => Err(ExprError::new(
                "unexpected-token",
                "expected a number, identifier, or `(`".to_string(),
                span.start,
                span.len(),
            )),
        }
    }
}

pub fn parse(source: &str) -> Result<Expr, ExprError> {
    let (parser, expr) = parse_expression(source)?;
    match parser.current() {
        None => Ok(expr),
        Some((MathToken::Eq | MathToken::Colon | MathToken::ColonEq, span)) => Err(ExprError::new(
            "unsupported-statement",
            "relations and bindings are not supported yet".to_string(),
            span.start,
            span.len(),
        )),
        Some((_, span)) => Err(ExprError::new(
            "unexpected-token",
            "unexpected token after the expression".to_string(),
            span.start,
            span.len(),
        )),
    }
}

pub fn parse_statement(source: &str) -> Result<Statement, ExprError> {
    let (mut parser, expr) = parse_expression(source)?;

    match parser.current() {
        None => Ok(Statement::Expr(expr)),
        Some((MathToken::ColonEq, span)) => {
            parser.advance();
            let Expr::Ident(name) = expr else {
                return Err(ExprError::new(
                    "invalid-binding",
                    "`:=` binds a name, so the left side must be an identifier".to_string(),
                    span.start,
                    span.len(),
                ));
            };
            let value = parser.expression()?;
            match parser.current() {
                None => Ok(Statement::Binding { name, value }),
                Some((_, span)) => Err(ExprError::new(
                    "unexpected-token",
                    "unexpected token after the binding".to_string(),
                    span.start,
                    span.len(),
                )),
            }
        }
        Some((MathToken::Eq, _)) => {
            parser.advance();
            let rhs = parser.expression()?;
            match parser.current() {
                None => Ok(Statement::Relation { lhs: expr, rhs }),
                Some((MathToken::Eq, span)) => Err(ExprError::new(
                    "unsupported-statement",
                    "chained relations are not supported yet".to_string(),
                    span.start,
                    span.len(),
                )),
                Some((_, span)) => Err(ExprError::new(
                    "unexpected-token",
                    "unexpected token after the relation".to_string(),
                    span.start,
                    span.len(),
                )),
            }
        }
        Some((MathToken::Colon, span)) => Err(ExprError::new(
            "unsupported-statement",
            "typed bindings are not supported yet".to_string(),
            span.start,
            span.len(),
        )),
        Some((_, span)) => Err(ExprError::new(
            "unexpected-token",
            "unexpected token after the expression".to_string(),
            span.start,
            span.len(),
        )),
    }
}

fn parse_expression<'a>(source: &'a str) -> Result<(Parser<'a>, Expr), ExprError> {
    let tokens = tokenize(source)?;
    let mut parser = Parser {
        tokens,
        pos: 0,
        end: source.len(),
    };
    let expr = parser.expression()?;
    Ok((parser, expr))
}
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn simple() {
        let expr = parse("1 + 2 * 3").unwrap();
        assert_eq!(
            expr,
            Expr::Binary {
                op: BinOp::Add,
                lhs: Box::new(Expr::Number("1".to_string())),
                rhs: Box::new(Expr::Binary {
                    op: BinOp::Mul,
                    lhs: Box::new(Expr::Number("2".to_string())),
                    rhs: Box::new(Expr::Number("3".to_string())),
                }),
            }
        );
    }

    #[test]
    fn operators_associate_to_the_left() {
        let expr = parse("10 - 3 - 2").unwrap();
        let Expr::Binary { op, lhs, rhs } = expr else {
            panic!("expected binary expression");
        };
        assert_eq!(op, BinOp::Sub);
        assert!(matches!(lhs.as_ref(), Expr::Binary { op: BinOp::Sub, .. }));
        assert_eq!(rhs.as_ref(), &Expr::Number("2".to_string()));
    }

    #[test]
    fn pemdas() {
        assert_eq!(
            parse("(1 + 2) * 3").unwrap(),
            Expr::Binary {
                op: BinOp::Mul,
                lhs: Box::new(Expr::Paren(Box::new(Expr::Binary {
                    op: BinOp::Add,
                    lhs: Box::new(Expr::Number("1".to_string())),
                    rhs: Box::new(Expr::Number("2".to_string())),
                }))),
                rhs: Box::new(Expr::Number("3".to_string())),
            }
        );
        assert_eq!(
            parse("-2 * 3").unwrap(),
            Expr::Binary {
                op: BinOp::Mul,
                lhs: Box::new(Expr::Neg(Box::new(Expr::Number("2".to_string())))),
                rhs: Box::new(Expr::Number("3".to_string())),
            }
        );
    }

    #[test]
    fn dangling_operator_hits_eof() {
        assert_eq!(parse("1 +").unwrap_err().code, "unexpected-eof");
        assert_eq!(parse("").unwrap_err().code, "unexpected-eof");
        assert_eq!(parse("(1 + 2").unwrap_err().message, "missing closing `)`");
    }

    #[test]
    fn decimals_and_idents_tokenize() {
        assert_eq!(
            parse("2.5 * pi").unwrap(),
            Expr::Binary {
                op: BinOp::Mul,
                lhs: Box::new(Expr::Number("2.5".to_string())),
                rhs: Box::new(Expr::Ident("pi".to_string())),
            }
        );
        let error = parse("1..2").unwrap_err();
        assert_eq!(error.code, "unexpected-character");
    }

    #[test]
    fn a_second_dot_is_reported_where_it_appears() {
        let error = parse("1.2.3").unwrap_err();
        // println!("{error:?}");
        assert_eq!(error.code, "unexpected-character");
        assert_eq!(error.offset, 3);
        assert_eq!(error.len, 1);
        assert_eq!(error.message, "a number may contain at most one `.`");
    }

    #[test]
    fn unexpected_characters_are_rejected() {
        let error = parse("2 + @").unwrap_err();
        assert_eq!(error.code, "unexpected-character");
        assert_eq!(error.offset, 4);
        assert_eq!(error.len, 1);
        assert_eq!(error.message, "unexpected character `@` in math expression");
    }

    #[test]
    fn unicode_idents_tokenize() {
        // halflife.
        assert_eq!(parse("λ + 2").unwrap(), {
            Expr::Binary {
                op: BinOp::Add,
                lhs: Box::new(Expr::Ident("λ".to_string())),
                rhs: Box::new(Expr::Number("2".to_string())),
            }
        });
        assert_eq!(parse("_x1").unwrap(), Expr::Ident("_x1".to_string()));
    }

    #[test]
    fn free_vars_walk_the_whole_tree() {
        let expr = parse("(a + b) * a - -c").unwrap();
        let vars: Vec<String> = expr.free_vars().into_iter().collect();
        assert_eq!(vars, ["a", "b", "c"]);
        assert!(parse("1 / 2").unwrap().free_vars().is_empty());
    }

    #[test]
    fn colon_eq_binds_a_name() {
        assert_eq!(
            parse_statement("x := 5").unwrap(),
            Statement::Binding {
                name: "x".to_string(),
                value: Expr::Number("5".to_string()),
            }
        );
        assert_eq!(
            parse_statement("total := (1 + 2) * 3").unwrap(),
            Statement::Binding {
                name: "total".to_string(),
                value: Expr::Binary {
                    op: BinOp::Mul,
                    lhs: Box::new(Expr::Paren(Box::new(Expr::Binary {
                        op: BinOp::Add,
                        lhs: Box::new(Expr::Number("1".to_string())),
                        rhs: Box::new(Expr::Number("2".to_string())),
                    }))),
                    rhs: Box::new(Expr::Number("3".to_string())),
                },
            }
        );
        assert_eq!(
            parse_statement("1 + 2").unwrap(),
            Statement::Expr(Expr::Binary {
                op: BinOp::Add,
                lhs: Box::new(Expr::Number("1".to_string())),
                rhs: Box::new(Expr::Number("2".to_string())),
            })
        );
    }

    #[test]
    fn bindings_are_top_level_only() {
        assert_eq!(
            parse_statement("(x := 5)").unwrap_err().code,
            "unsupported-statement"
        );
        assert_eq!(
            parse_statement("1 := 2").unwrap_err().code,
            "invalid-binding"
        );
        assert_eq!(
            parse_statement("(a + b) := 2").unwrap_err().code,
            "invalid-binding" // should be valid in the future.
        );
    }

    #[test]
    fn bullshitto_rejeto() {
        assert_eq!(
            parse_statement("x := 5 +").unwrap_err().code,
            "unexpected-eof"
        );
        assert_eq!(
            parse_statement("x := 5 6").unwrap_err().code,
            "unexpected-token"
        );
    }

    #[test]
    fn colon_eq_wins_over_a_bare_colon() {
        let tokens = tokenize("x := 5").unwrap();
        assert!(matches!(tokens[1].0, MathToken::ColonEq));
        let tokens = tokenize("x: 5").unwrap();
        assert!(matches!(tokens[1].0, MathToken::Colon));
    }
}
