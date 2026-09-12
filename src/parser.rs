use logos::Logos;
use std::collections::VecDeque;

use crate::{
    ast::{Block, Document, DocumentMetadata, Inline},
    lexer::Token::{self, BiggerThan, CommentEnd},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Delimiter {
    Bold,
    Italic,
    Underline,
    Strikethru,
}

impl Delimiter {
    fn as_token<'a>(self) -> Token<'a> {
        match self {
            Delimiter::Bold => Token::Bold,
            Delimiter::Italic => Token::Italic,
            Delimiter::Underline => Token::Underline,
            Delimiter::Strikethru => Token::Striketrhu,
        }
    }
}

fn delimiter(token: &Token<'_>) -> Option<Delimiter> {
    match token {
        Token::Bold => Some(Delimiter::Bold),
        Token::Italic => Some(Delimiter::Italic),
        Token::Underline => Some(Delimiter::Underline),
        Token::Striketrhu => Some(Delimiter::Strikethru),
        _ => None,
    }
}

pub struct Parser<'a> {
    lexer: logos::Lexer<'a, Token<'a>>,
    current: Option<Token<'a>>,
    open_delimiters: Vec<Delimiter>,
    token_buffer: VecDeque<Token<'a>>,
}

pub fn parse(input: &str) -> Document<'_> {
    Parser::new(input).parse()
}

impl<'a> Parser<'a> {
    fn new(input: &'a str) -> Self {
        let mut lexer = Token::lexer(input);
        let current = lexer.next().and_then(|result| result.ok());

        Self {
            lexer,
            current,
            open_delimiters: Vec::new(),
            token_buffer: VecDeque::new(),
        }
    }

    fn parse(mut self) -> Document<'a> {
        Document {
            meta: DocumentMetadata {},
            blocks: self.parse_blocks(),
        }
    }

    fn parse_blocks(&mut self) -> Vec<Block<'a>> {
        let mut blocks = Vec::new();

        while self.current.is_some() {
            match self.current {
                Some(Token::HeadingMarker(_)) => {
                    print!("seen heading");
                    blocks.push(self.parse_heading());
                }

                Some(Token::ParagraphBreak) | Some(Token::Newline) => {
                    self.bump();
                }

                Some(Token::BiggerThan) => {
                    blocks.push(self.parse_quote());
                }

                Some(Token::CodeBlockStart) => {
                    blocks.push(self.parse_code_block());
                }

                Some(_) => {
                    blocks.push(self.parse_paragraph());
                }
                None => break,
            }
        }
        blocks
    }

    fn bump(&mut self) {
        if let Some(token) = self.token_buffer.pop_front() {
            self.current = Some(token);
        } else {
            self.current = self.lexer.next().and_then(|result| result.ok());
        }
    }

    fn parse_heading(&mut self) -> Block<'a> {
        let marker = match self.current.take() {
            Some(Token::HeadingMarker(marker)) => marker,
            _ => unreachable!(),
        };

        let level = marker.matches('.').count() + 1;

        self.bump();

        let content = self.parse_until_newline();

        Block::Heading { level, content }
    }

    fn parse_paragraph(&mut self) -> Block<'a> {
        let content = self.parse_until_paragraph_break();

        Block::Paragraph(content)
    }

    fn parse_until_newline(&mut self) -> Vec<Inline<'a>> {
        let mut result = Vec::new();

        loop {
            match self.current {
                None | Some(Token::Newline) | Some(Token::ParagraphBreak) => {
                    if matches!(self.current, Some(Token::Newline)) {
                        self.bump();
                    }

                    break;
                }

                _ => {
                    if let Some(inline) = self.parse_inline() {
                        result.push(inline);
                    } else {
                        self.bump();
                    }
                }
            }
        }

        result
    }
    /// skip comments, set current pos to comment + 1
    fn skip_comments(&mut self) {
        println!("{:?}", self.current);
        while self.current.is_some() && self.current != Some(Token::CommentEnd) {
            println!("{:?}", self.current);
            self.bump();
        }
        if self.current == Some(CommentEnd) {
            self.bump();
        }
    }
    fn parse_until_paragraph_break(&mut self) -> Vec<Inline<'a>> {
        let mut result = Vec::new();

        loop {
            match self.current {
                None | Some(Token::ParagraphBreak) => {
                    if matches!(self.current, Some(Token::ParagraphBreak)) {
                        self.bump();
                    }

                    break;
                }

                Some(Token::Newline) => {
                    self.bump();
                }

                _ => {
                    if let Some(inline) = self.parse_inline() {
                        result.push(inline);
                    } else {
                        self.bump();
                    }
                }
            }
        }

        result
    }
    fn parse_quote(&mut self) -> Block<'a> {
        let mut level = 0;

        while let Some(BiggerThan) = self.current {
            level += 1;
            self.bump();
        }

        if matches!(self.current, Some(Token::Whitespace(_))) {
            self.bump();
        }

        Block::Quote {
            level,
            content: self.parse_quote_content(level),
        }
    }

    fn parse_quote_content(&mut self, level: i32) -> Vec<Block<'a>> {
        let mut content = Vec::new();

        loop {
            match self.current {
                None => break,

                Some(Token::ParagraphBreak) => {
                    break;
                }

                Some(Token::Newline) => {
                    self.bump();
                }

                Some(Token::BiggerThan) => {
                    let mut new_level = 0;

                    while let Some(Token::BiggerThan) = self.current {
                        new_level += 1;
                        self.bump();
                    }

                    if matches!(self.current, Some(Token::Whitespace(_))) {
                        self.bump();
                    }

                    if new_level > level {
                        content.push(Block::Quote {
                            level: new_level,
                            content: self.parse_quote_content(new_level),
                        });
                    } else if new_level == level {
                        content.push(Block::Paragraph(self.parse_until_newline()));
                    } else {
                        for _ in 0..new_level {
                            self.token_buffer.push_front(Token::BiggerThan);
                        }

                        break;
                    }
                }

                _ => {
                    content.push(Block::Paragraph(self.parse_until_newline()));
                }
            }
        }

        content
    }

    fn parse_code_block(&mut self) -> Block<'a> {
        let rem = self.lexer.remainder();

        let newline_pos = rem.find('\n').unwrap_or(rem.len());
        let header_line = &rem[..newline_pos];

        let header = header_line.trim();

        let (before_delim, delim) = match header.rfind('|') {
            Some(pos) => (&header[..pos], header[pos + 1..].trim()),
            None => (header, ""),
        };

        let (lang, title) = match before_delim.find(':') {
            Some(pos) => {
                let lang = before_delim[..pos].trim();
                let title = before_delim[pos + 1..].trim();
                (lang, if title.is_empty() { None } else { Some(title) })
            }
            None => (before_delim.trim(), None),
        };

        let close_tag = format!("!{}>", delim);

        let after_header = if newline_pos < rem.len() {
            &rem[newline_pos + 1..]
        } else {
            ""
        };

        let mut search_offset = 0;
        let mut close_pos = None;

        while let Some(idx) = after_header[search_offset..].find(&close_tag) {
            let absolute_idx = search_offset + idx;
            let at_line_start =
                absolute_idx == 0 || after_header.as_bytes()[absolute_idx - 1] == b'\n';
            if at_line_start {
                close_pos = Some(absolute_idx);
                break;
            } else {
                search_offset = absolute_idx + close_tag.len();
            }
        }

        let (content, total_skip) = if let Some(idx) = close_pos {
            let raw_content = &after_header[..idx];
            let content = if let Some(s) = raw_content.strip_suffix("\r\n") {
                s
            } else if let Some(s) = raw_content.strip_suffix('\n') {
                s
            } else {
                raw_content
            };

            let mut skip = newline_pos + 1 + idx + close_tag.len();

            // Skip trailing newline after `!END>` if present
            // if not, hell will break loose.
            let after_close = &rem[skip..];
            if after_close.starts_with("\r\n") {
                skip += 2;
            } else if after_close.starts_with('\n') {
                skip += 1;
            }

            (content, skip)
        } else {
            // Unclosed code block, consume to EOF
            (after_header, rem.len())
        };

        self.lexer.bump(total_skip);
        self.bump();

        Block::CodeBlock {
            lang,
            title,
            content,
        }
    }

    fn parse_inline(&mut self) -> Option<Inline<'a>> {
        match self.current {
            // TODO: make some kind of a buffer for these.
            Some(Token::Text(text)) => {
                self.bump();
                Some(Inline::Text(text))
            }

            Some(Token::Whitespace(text)) => {
                self.bump();
                Some(Inline::Text(text))
            }

            Some(Token::LBracket) => {
                self.bump();
                Some(Inline::Text("["))
            }

            Some(Token::RBracket) => {
                self.bump();
                Some(Inline::Text("]"))
            }
            Some(Token::CommentStart) => {
                println!("{:?}", self.current);

                self.skip_comments(); // I can deffinelty find a better way to do this
                Some(Inline::Text(""))
            }
            Some(Token::CommentEnd) => {
                self.bump();
                Some(Inline::Text("'/"))
            }
            Some(Token::Backtick) => {
                let rem = self.lexer.remainder();

                if let Some(close_idx) = rem.find('`') {
                    let content = &rem[..close_idx];

                    if !content.contains("\n") {
                        let after_code = &rem[close_idx + 1..];
                        let mut lang = None;
                        let mut total_skip = close_idx + 1;

                        if after_code.starts_with('[')
                            && let Some(end_bracket) = after_code.find(']')
                        {
                            let ident = &after_code[1..end_bracket];
                            if !ident.is_empty() && !ident.contains(char::is_whitespace) {
                                lang = Some(ident);
                                total_skip += end_bracket + 1;
                            }
                        }

                        self.lexer.bump(total_skip);
                        self.bump();

                        Some(Inline::Code { content, lang })
                    } else {
                        self.bump();
                        Some(Inline::Text("`"))
                    }
                } else {
                    self.bump();
                    Some(Inline::Text("`"))
                }
            }

            Some(Token::Bold) => Some(self.parse_delimited(Delimiter::Bold, Inline::Bold)),

            Some(Token::Italic) => Some(self.parse_delimited(Delimiter::Italic, Inline::Italic)),

            Some(Token::Underline) => {
                Some(self.parse_delimited(Delimiter::Underline, Inline::Underline))
            }

            Some(Token::Striketrhu) => {
                Some(self.parse_delimited(Delimiter::Strikethru, Inline::Strikethru))
            }

            _ => None,
        }
    }

    fn parse_delimited<F>(&mut self, delimiter: Delimiter, make_inline: F) -> Inline<'a>
    where
        F: FnOnce(Vec<Inline<'a>>) -> Inline<'a>,
    {
        self.bump();
        self.open_delimiters.push(delimiter);

        let content = self.parse_delimited_content(delimiter);

        self.open_delimiters.pop();
        make_inline(content)
    }

    fn parse_delimited_content(&mut self, closing: Delimiter) -> Vec<Inline<'a>> {
        let mut result = Vec::new();

        loop {
            match self.current {
                None => {
                    break;
                }

                Some(Token::Newline | Token::ParagraphBreak) => {
                    break;
                }

                _ => {
                    if let Some(found) = self.current.as_ref().and_then(delimiter) {
                        if found == closing {
                            self.bump();
                            break;
                        }

                        if self.open_delimiters.contains(&found) {
                            self.token_buffer.push_front(closing.as_token());
                            break;
                        }
                    }

                    if let Some(inline) = self.parse_inline() {
                        result.push(inline);
                    } else {
                        self.bump();
                    }
                }
            }
        }

        result
    }
}
// tested by good enough tm
