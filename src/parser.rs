use logos::Logos;
use std::borrow::Cow;
use std::collections::VecDeque;

use crate::{
    ast::{AutoSymbol, Block, Document, DocumentMetadata, Inline, ListItem, ListMarker},
    lexer::Token::{self},
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

fn to_lower_alpha(mut n: usize) -> String {
    let mut s = String::new();
    while n > 0 {
        n -= 1;
        s.push((b'a' + (n % 26) as u8) as char);
        n /= 26;
    }
    s.chars().rev().collect()
}

fn to_upper_alpha(n: usize) -> String {
    to_lower_alpha(n).to_uppercase()
}

fn to_roman(mut n: usize) -> String {
    let table = [
        (1000, "m"),
        (900, "cm"),
        (500, "d"),
        (400, "cd"),
        (100, "c"),
        (90, "xc"),
        (50, "l"),
        (40, "xl"),
        (10, "x"),
        (9, "ix"),
        (5, "v"),
        (4, "iv"),
        (1, "i"),
    ];
    let mut s = String::new();
    for &(val, sym) in &table {
        while n >= val {
            s.push_str(sym);
            n -= val;
        }
    }
    s
}

struct ListCounter {
    stack: Vec<(AutoSymbol, usize)>,
}

impl ListCounter {
    fn new() -> Self {
        Self { stack: Vec::new() }
    }

    fn reset(&mut self) {
        self.stack.clear();
    }

    fn explicit(&mut self, num: usize) -> (usize, ListMarker) {
        if self.stack.is_empty() {
            self.stack.push((AutoSymbol::Numeric, num));
        } else {
            self.stack.truncate(1);
            self.stack[0] = (AutoSymbol::Numeric, num);
        }
        (1, ListMarker::Ordered(num))
    }

    fn auto(&mut self, symbols: &[AutoSymbol]) -> (usize, ListMarker) {
        let depth = symbols.len().max(1);

        if depth < self.stack.len() {
            self.stack.truncate(depth);
        }

        for i in 0..depth.saturating_sub(1) {
            let sym = symbols.get(i).copied().unwrap_or(AutoSymbol::Numeric);
            if i < self.stack.len() {
                if self.stack[i].0 != sym {
                    self.stack[i] = (sym, 1);
                }
            } else {
                self.stack.push((sym, 1));
            }
        }

        let last_idx = depth - 1;
        let target_sym = symbols
            .get(last_idx)
            .copied()
            .unwrap_or(AutoSymbol::Numeric);

        if last_idx < self.stack.len() {
            if self.stack[last_idx].0 == target_sym {
                self.stack[last_idx].1 += 1;
            } else {
                // Symbol changed at same depth
                self.stack[last_idx] = (target_sym, 1);
            }
        } else {
            self.stack.push((target_sym, 1));
        }

        let mut formatted = String::new();
        for (i, (symbol, count)) in self.stack.iter().enumerate() {
            if i > 0 {
                formatted.push('.');
            }

            formatted.push_str(&match symbol {
                AutoSymbol::Numeric => count.to_string(),
                AutoSymbol::LowerAlpha => to_lower_alpha(*count),
                AutoSymbol::UpperAlpha => to_upper_alpha(*count),
                AutoSymbol::Roman => to_roman(*count),
            });
        }

        formatted.push('.');

        (depth, ListMarker::Auto(symbols.to_vec(), formatted))
    }
}
fn is_text_token(token: &Token<'_>) -> bool {
    !matches!(
        token,
        Token::Bold
            | Token::Italic
            | Token::Underline
            | Token::Striketrhu
            | Token::Backtick
            | Token::CommentStart
            | Token::Newline
            | Token::ParagraphBreak // add tokens that shouldn't be treated as text.
            | Token::LBracket
            | Token::ImageOpen
            | Token::Backslash
    )
}

fn is_escape_punct(c: char) -> bool {
    matches!(
        c,
        '!' | '"'
            | '#'
            | '$'
            | '%'
            | '&'
            | '\''
            | '('
            | ')'
            | '*'
            | '+'
            | ','
            | '-'
            | '.'
            | '/'
            | ':'
            | ';'
            | '<'
            | '='
            | '>'
            | '?'
            | '@'
            | '['
            | '\\'
            | ']'
            | '^'
            | '_'
            | '`'
            | '{'
            | '|'
            | '}'
            | '~'
    )
}

fn unescape<'a>(s: &'a str) -> Cow<'a, str> {
    if !s.contains('\\') {
        return Cow::Borrowed(s);
    }
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();

    while let Some(c) = chars.next() {
        if c == '\\' {
            match chars.peek() {
                Some(next) if is_escape_punct(*next) => {
                    out.push(*next);
                    chars.next();
                }
                _ => out.push('\\'),
            }
        } else {
            out.push(c);
        }
    }
    // mooo
    Cow::Owned(out)
}
pub struct Parser<'a> {
    lexer: logos::Lexer<'a, Token<'a>>,
    current: Option<Token<'a>>,
    open_delimiters: Vec<Delimiter>,
    token_buffer: VecDeque<Token<'a>>,
    list_counter: ListCounter,
    pending_list: Option<(i32, usize, ListMarker)>,
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
            list_counter: ListCounter::new(),
            pending_list: None,
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
            if let Some(block) = self.parse_block() {
                blocks.push(block);
            }
        }

        blocks
    }
    fn parse_block(&mut self) -> Option<Block<'a>> {
        if let Some((quote, depth, label)) = self.next_list_marker() {
            let inner = self.parse_list_with(quote, depth, label);
            if quote > 0 {
                return Some(Block::Quote {
                    level: quote,
                    content: vec![inner],
                });
            }
            return Some(inner);
        }

        match self.current {
            Some(Token::HeadingMarker(_)) => {
                self.list_counter.reset(); // New section resets list counters
                Some(self.parse_heading())
            }
            Some(Token::BiggerThan) => Some(self.parse_quote()),
            Some(Token::CodeBlockStart) => Some(self.parse_code_block()),
            Some(Token::Newline | Token::ParagraphBreak) => {
                self.bump();
                None
            }
            Some(Token::ThematicBreak) => {
                self.bump();
                Some(Block::ThematicBreak)
            }
            Some(_) => {
                let p = self.parse_paragraph();
                if let Block::Paragraph(ref inlines) = p
                    && inlines.is_empty()
                {
                    return None;
                }

                Some(p)
            }
            None => None,
        }
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
        self.skip_whitespace();

        let content = self.parse_until_newline();

        Block::Heading { level, content }
    }

    fn parse_paragraph(&mut self) -> Block<'a> {
        let content = self.parse_until_paragraph_break();

        Block::Paragraph(content)
    }
    fn skip_whitespace(&mut self) {
        if matches!(self.current, Some(Token::Whitespace(_))) {
            self.bump();
        }
    }
    fn next_list_marker(&mut self) -> Option<(i32, usize, ListMarker)> {
        if let Some(pending) = self.pending_list.take() {
            return Some(pending);
        }
        self.try_parse_list_marker()
    }
    fn try_parse_list_marker(&mut self) -> Option<(i32, usize, ListMarker)> {
        let mut skipped_toks = Vec::new();

        while matches!(self.current, Some(Token::Whitespace(_))) {
            skipped_toks.push(self.current.take().unwrap());
            self.bump();
        }

        while self.current == Some(Token::BiggerThan) {
            skipped_toks.push(self.current.take().unwrap());
            self.bump();

            while matches!(self.current, Some(Token::Whitespace(_))) {
                skipped_toks.push(self.current.take().unwrap());
                self.bump();
            }
        }

        let indent_width: usize = skipped_toks
            .iter()
            .filter_map(|tok| match tok {
                Token::Whitespace(s) => {
                    Some(
                        s.chars()
                            .map(|c| if c == '\t' { 2 } else { 1 })
                            .sum::<usize>(), //           ^ Cry about it. >:3
                    )
                }
                _ => None,
            })
            .sum();
        let indent_depth = indent_width / 2 + 1;

        let quote_depth = skipped_toks
            .iter()
            .filter(|tok| **tok == Token::BiggerThan)
            .count() as i32;

        let res = self
            .try_parse_explicit_list()
            .or_else(|| self.try_parse_unordered_list(indent_depth))
            .or_else(|| self.try_parse_auto_list());

        if res.is_none() {
            // Backtrack the quote and whitespace markers if it wasn't a list item
            if !skipped_toks.is_empty() {
                if let Some(tok) = self.current.take() {
                    self.token_buffer.push_front(tok);
                }
                while let Some(tok) = skipped_toks.pop() {
                    self.token_buffer.push_front(tok);
                }
                self.current = self.token_buffer.pop_front();
            }
            return None;
        }

        res.map(|(depth, marker)| (quote_depth, depth, marker))
    }
    fn try_parse_explicit_list(&mut self) -> Option<(usize, ListMarker)> {
        let Some(Token::Text(digits)) = self.current else {
            return None;
        };

        let Ok(num) = digits.parse::<usize>() else {
            return None;
        };

        let rem = self.lexer.remainder();

        if !rem.starts_with('.') || !rem[1..].starts_with([' ', '\t']) {
            return None;
        }

        self.bump();
        self.bump();
        self.skip_whitespace();

        Some(self.list_counter.explicit(num))
    }
    fn try_parse_unordered_list(&mut self, indent_depth: usize) -> Option<(usize, ListMarker)> {
        match self.current {
            Some(Token::Star) => {}
            _ => return None,
        };

        let rem = self.lexer.remainder();

        if !rem.starts_with([' ', '\t']) {
            return None;
        }

        self.bump();
        self.skip_whitespace();

        Some((indent_depth, ListMarker::Bullet))
    }
    fn try_parse_auto_list(&mut self) -> Option<(usize, ListMarker)> {
        let symbol = match self.current {
            Some(Token::Plus) => AutoSymbol::Numeric,
            Some(Token::Minus) => AutoSymbol::LowerAlpha,
            Some(Token::Caret) => AutoSymbol::UpperAlpha,
            Some(Token::Equals) => AutoSymbol::Roman,
            _ => return None,
        };

        let rem = self.lexer.remainder();

        if rem.starts_with([' ', '\t']) {
            self.bump();
            self.skip_whitespace();

            return Some(self.list_counter.auto(&[symbol]));
        }

        self.try_parse_nested_auto_list(symbol)
    }
    fn try_parse_nested_auto_list(&mut self, first: AutoSymbol) -> Option<(usize, ListMarker)> {
        let rem = self.lexer.remainder();

        if !rem.starts_with('.') {
            return None;
        }

        let ws_idx = rem.find([' ', '\t', '\n'])?;
        let pattern = &rem[..ws_idx];

        if !pattern
            .chars()
            .all(|c| matches!(c, '.' | '+' | '-' | '^' | '='))
        {
            return None;
        }

        let mut symbols = vec![first];

        for c in pattern.chars() {
            if let Some(sym) = AutoSymbol::from_char(c) {
                symbols.push(sym);
            }
        }

        self.lexer.bump(ws_idx);
        self.bump();
        self.skip_whitespace();

        Some(self.list_counter.auto(&symbols))
    }

    /// Parse a list while keeping what's at the left in mind
    fn parse_list_with(
        &mut self,
        base_quote: i32,
        first_depth: usize,
        first_marker: ListMarker,
    ) -> Block<'a> {
        let mut items = Vec::new();

        let first_para = Block::Paragraph(self.parse_until_newline());
        let mut item_blocks = vec![first_para];
        self.parse_gutter_blocks(&mut item_blocks);

        items.push(ListItem {
            depth: first_depth,
            marker: first_marker,
            blocks: item_blocks,
        });

        while self.current.is_some() {
            while matches!(self.current, Some(Token::Newline)) {
                self.bump();
            }

            if self.current == Some(Token::ParagraphBreak) {
                self.bump();
                while matches!(self.current, Some(Token::Newline)) {
                    self.bump();
                }
                if self.current.is_none() {
                    break;
                }
            }

            if let Some((quote, depth, marker)) = self.next_list_marker() {
                // This code is organic as the shit a cow produces.
                // it works tho.. don't ask further.
                if quote == base_quote {
                    let first_para = Block::Paragraph(self.parse_until_newline());
                    let mut item_blocks = vec![first_para];
                    self.parse_gutter_blocks(&mut item_blocks);

                    items.push(ListItem {
                        depth,
                        marker,
                        blocks: item_blocks,
                    });
                } else if quote > base_quote && base_quote > 0 {
                    let nested = self.parse_list_with(quote, depth, marker.clone());
                    let quoted = Block::Quote {
                        level: quote,
                        content: vec![nested],
                    };
                    if let Some(last) = items.last_mut() {
                        last.blocks.push(quoted);
                    } else {
                        self.pending_list = Some((quote, depth, marker));
                        break;
                    }
                } else {
                    self.pending_list = Some((quote, depth, marker));
                    break;
                }
            } else {
                break;
            }
        }

        Block::List { items }
    }

    fn parse_gutter_blocks(&mut self, item_blocks: &mut Vec<Block<'a>>) {
        while self.try_consume_gutter_prefix() {
            self.bump(); // consume `|`
            self.skip_whitespace();

            match self.current {
                Some(Token::CodeBlockStart) => {
                    item_blocks.push(self.parse_code_block());
                }
                Some(Token::BiggerThan) => {
                    item_blocks.push(self.parse_quote());
                }
                Some(Token::Newline | Token::ParagraphBreak) => {
                    self.bump();
                }
                Some(_) => {
                    let inlines = self.parse_until_newline();
                    if !inlines.is_empty() {
                        item_blocks.push(Block::Paragraph(inlines));
                    }
                }
                None => break,
            }
        }
    }

    fn try_consume_gutter_prefix(&mut self) -> bool {
        let mut skipped = Vec::new();

        if self.current == Some(Token::Newline) {
            skipped.push(self.current.take().unwrap());
            self.bump();
        }

        if self.current == Some(Token::ParagraphBreak) {
            skipped.push(self.current.take().unwrap());
            self.bump();
        }

        while self.current == Some(Token::BiggerThan) {
            skipped.push(self.current.take().unwrap());
            self.bump();
        }

        if matches!(self.current, Some(Token::Whitespace(_))) {
            skipped.push(self.current.take().unwrap());
            self.bump();
        }

        if self.current == Some(Token::Pipe) {
            return true;
        }

        if !skipped.is_empty() {
            if let Some(tok) = self.current.take() {
                self.token_buffer.push_front(tok);
            }
            while let Some(tok) = skipped.pop() {
                self.token_buffer.push_front(tok);
            }
            self.current = self.token_buffer.pop_front();
        }

        false
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
        loop {
            match &self.current {
                None => break,
                Some(Token::CommentEnd) => {
                    self.bump();
                    break;
                }
                Some(Token::Backslash) => {
                    self.bump();
                    match &self.current {
                        None => break,
                        Some(_) => self.bump(),
                    }
                }
                Some(_) => self.bump(),
            }
        }
    }

    fn is_at_block_boundary(&self) -> bool {
        match self.current {
            Some(Token::HeadingMarker(_))
            | Some(Token::CodeBlockStart)
            | Some(Token::BiggerThan)
            | Some(Token::Pipe) => true,

            Some(Token::Plus | Token::Caret | Token::Equals) => {
                let rem = self.lexer.remainder();
                rem.starts_with([' ', '\t', '.'])
            }

            Some(Token::Minus) => {
                let rem = self.lexer.remainder();
                rem.starts_with([' ', '\t', '.'])
            }

            Some(Token::Star) => {
                let rem = self.lexer.remainder();
                rem.starts_with([' ', '\t'])
            }

            Some(Token::Text(digits)) if digits.chars().all(|c| c.is_ascii_digit()) => {
                let rem = self.lexer.remainder();
                rem.starts_with('.') && rem[1..].starts_with([' ', '\t'])
            }

            Some(Token::Whitespace(_)) => {
                let rem = self.lexer.remainder().trim_start_matches([' ', '\t', '>']);
                rem.starts_with('|') // I could be smart here, but I want to be verbose while
                                     // developing it 
                    || rem.starts_with("+ ")
                    || rem.starts_with("+\t")
                    || rem.starts_with("+.")
                    || rem.starts_with("- ")
                    || rem.starts_with("-\t")
                    || rem.starts_with("-.")
                    || rem.starts_with("* ")
                    || rem.starts_with("*\t")
                    || rem.starts_with("^ ")
                    || rem.starts_with("^\t")
                    || rem.starts_with("^.")
                    || rem.starts_with("= ")
                    || rem.starts_with("=\t")
                    || rem.starts_with("=.")
                    || (rem
                        .as_bytes()
                        .first()
                        .map_or_else(|| false, |b| b.is_ascii_digit())
                        && rem.contains('.'))
            }

            _ => false,
        }
    }
    fn parse_quote(&mut self) -> Block<'a> {
        let level = self.consume_quote_prefix();
        let mut content = Vec::new();

        if self.current == Some(Token::CodeBlockStart) {
            content.push(self.parse_code_block());
        } else {
            let inlines = self.parse_until_newline();
            if !inlines.is_empty() {
                content.push(Block::Paragraph(inlines));
            }
        }

        loop {
            if self.current == Some(Token::BiggerThan) {
                let mut skipped = Vec::new();
                let mut ws_skipped = Vec::new();

                while self.current == Some(Token::BiggerThan) {
                    skipped.push(self.current.take().unwrap());
                    self.bump();
                }

                while matches!(self.current, Some(Token::Whitespace(_))) {
                    ws_skipped.push(self.current.take().unwrap());
                    self.bump();
                }

                let next_level = skipped.len() as i32;

                let is_interrupting_boundary = self.is_at_block_boundary()
                    && self.current != Some(Token::BiggerThan)
                    && self.current != Some(Token::CodeBlockStart);

                if next_level >= level && is_interrupting_boundary {
                    if let Some(tok) = self.current.take() {
                        self.token_buffer.push_front(tok);
                    }
                    while let Some(tok) = ws_skipped.pop() {
                        self.token_buffer.push_front(tok);
                    }
                    while let Some(tok) = skipped.pop() {
                        self.token_buffer.push_front(tok);
                    }
                    self.current = self.token_buffer.pop_front();
                    break;
                }

                if next_level == level {
                    if self.current == Some(Token::CodeBlockStart) {
                        content.push(self.parse_code_block());
                    } else {
                        let inlines = self.parse_until_newline();
                        if !inlines.is_empty() {
                            content.push(Block::Paragraph(inlines));
                        }
                    }
                } else if next_level > level {
                    let inlines = self.parse_until_newline();
                    if !inlines.is_empty() {
                        let para = Block::Paragraph(inlines);
                        Self::push_deep_quote(&mut content, next_level, para);
                    }
                } else {
                    if let Some(tok) = self.current.take() {
                        self.token_buffer.push_front(tok);
                    }
                    while let Some(tok) = ws_skipped.pop() {
                        self.token_buffer.push_front(tok);
                    }
                    while let Some(tok) = skipped.pop() {
                        self.token_buffer.push_front(tok);
                    }
                    self.current = self.token_buffer.pop_front();
                    break;
                }
            } else if self.current == Some(Token::Pipe) {
                let rem = self.lexer.remainder().trim_start_matches([' ', '\t']);
                if rem.starts_with('>') {
                    self.bump(); // consume `|`
                    self.skip_whitespace();
                    continue;
                } else {
                    break;
                }
            } else {
                break;
            }
        }

        Block::Quote { level, content }
    }

    /**
    NOTE:
    Documented here because I'm lazy to document it internally.
    Smart attach for deeper quote lines.
    `> / >> / >>>` nests stepwise: jumps like `>> \n >>>>>` attach one
    structural level deeper while preserving the requested level for
    styling. Siblings (e.g. `>>>>> & >>>`) stay siblings.
    */
    fn push_deep_quote(content: &mut Vec<Block<'a>>, next_level: i32, para: Block<'a>) {
        if let Some(Block::Quote {
            level: last_level,
            content: last_content,
        }) = content.last_mut()
            && *last_level < next_level
        {
            Self::push_deep_quote(last_content, next_level, para);
            return;
        }
        content.push(Block::Quote {
            level: next_level,
            content: vec![para],
        });
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

                    if self.is_at_block_boundary() {
                        break;
                    }
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
    fn consume_quote_prefix(&mut self) -> i32 {
        let mut level = 0;

        while self.current == Some(Token::BiggerThan) {
            level += 1;
            self.bump();
        }

        self.skip_whitespace();
        level
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
            let before = &after_header[..absolute_idx];

            let line_start = before.rfind('\n').map(|p| p + 1).unwrap_or(0);

            let at_line_start = before[line_start..].chars().all(|c| c == ' ' || c == '\t');

            if at_line_start {
                close_pos = Some((line_start, absolute_idx));
                break;
            } else {
                search_offset = absolute_idx + close_tag.len();
            }
        }

        let (content, total_skip) = if let Some((line_start, absolute_idx)) = close_pos {
            let raw_content = &after_header[..line_start];
            let content = if let Some(s) = raw_content.strip_suffix("\r\n") {
                s
            } else if let Some(s) = raw_content.strip_suffix('\n') {
                s
            } else {
                raw_content
            };

            let mut skip = newline_pos + 1 + absolute_idx + close_tag.len();

            let after_close = &rem[skip..];
            if after_close.starts_with("\r\n") {
                skip += 2;
            } else if after_close.starts_with('\n') {
                skip += 1;
            }

            (content, skip)
        } else {
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

    fn parse_inline_code(&mut self) -> Option<Inline<'a>> {
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
    fn parse_escape(&mut self) -> Option<Inline<'a>> {
        debug_assert_eq!(self.current, Some(Token::Backslash));
        self.bump(); // \
        if let Some(Token::Text(digits)) = &self.current
            && digits.chars().all(|c| c.is_ascii_digit())
        {
            let rem = self.lexer.remainder();
            if rem.starts_with('.') && rem[1..].starts_with([' ', '\t']) {
                let start = self.lexer.span().start;
                self.bump();

                if self.current == Some(Token::Dot) {
                    let end = self.lexer.span().end;
                    self.bump(); // consume `.`
                    return Some(Inline::Text(&self.lexer.source()[start..end]));
                }
                // Dot went missing ): fall through to lone `\`.
            }
        }

        let lit: Option<&'a str> = match &self.current {
            Some(Token::Bold) => Some("**"),
            Some(Token::Italic) => Some("__"),
            Some(Token::Underline) => Some("=="),
            Some(Token::Striketrhu) => Some("~~"),
            Some(Token::Backtick) => Some("`"),
            Some(Token::LBracket) => Some("["),
            Some(Token::RBracket) => Some("]"),
            Some(Token::ImageOpen) => Some("!["),
            Some(Token::CommentStart) => Some("/'"),
            Some(Token::CommentEnd) => Some("'/"),
            Some(Token::BiggerThan) => Some(">"),
            Some(Token::SmallerThan) => Some("<"),
            Some(Token::Pipe) => Some("|"),
            Some(Token::Plus) => Some("+"),
            Some(Token::Minus) => Some("-"),
            Some(Token::Caret) => Some("^"),
            Some(Token::Equals) => Some("="),
            Some(Token::Dot) => Some("."),
            Some(Token::Star) => Some("*"),
            Some(Token::CodeBlockStart) => Some("<!"),
            Some(Token::ThematicBreak) => Some("---"),
            Some(Token::HeadingMarker(s)) => Some(*s),
            Some(Token::Punctuation(s)) => Some(*s),
            Some(Token::Backslash) => Some("\\"),
            _ => None,
        };

        match lit {
            Some(text) => {
                self.bump();
                Some(Inline::Text(text))
            }
            None => Some(Inline::Text("\\")),
        }
    }

    fn parse_inline(&mut self) -> Option<Inline<'a>> {
        if let Some(tok) = &self.current
            && is_text_token(tok)
        {
            let start = self.lexer.span().start;
            let mut end = self.lexer.span().end;

            self.bump();
            while let Some(next_tok) = &self.current {
                if is_text_token(next_tok) {
                    end = self.lexer.span().end;
                    self.bump();
                } else {
                    break;
                }
            }

            let full_text = &self.lexer.source()[start..end];
            return Some(Inline::Text(full_text));
        }

        match self.current {
            Some(Token::Backtick) => self.parse_inline_code(),

            Some(Token::Backslash) => self.parse_escape(),

            Some(Token::CommentStart) => {
                self.skip_comments();
                self.parse_inline()
            }

            Some(Token::Bold) => Some(self.parse_delimited(Delimiter::Bold, Inline::Bold)),
            Some(Token::Italic) => Some(self.parse_delimited(Delimiter::Italic, Inline::Italic)),
            Some(Token::Underline) => {
                Some(self.parse_delimited(Delimiter::Underline, Inline::Underline))
            }
            Some(Token::Striketrhu) => {
                Some(self.parse_delimited(Delimiter::Strikethru, Inline::Strikethru))
            }

            Some(Token::LBracket) => Some(self.parse_link()),
            Some(Token::ImageOpen) => Some(self.parse_image()),
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

    fn parse_link(&mut self) -> Inline<'a> {
        let open_span = self.lexer.span();
        self.bump(); // [

        self.skip_whitespace();
        let url_start = match &self.current {
            Some(Token::RBracket | Token::Newline | Token::ParagraphBreak) | None => {
                // [] or [
                return Inline::Text(&self.lexer.source()[open_span.start..open_span.end]);
            }
            Some(_) => self.lexer.span().start,
        };

        let mut url_end = url_start;
        let mut has_pipe = false;
        let mut closed = false;

        while self.current.is_some() {
            if self.current == Some(Token::Backslash) {
                url_end = self.lexer.span().end;
                self.bump(); // consume `\`
                match &self.current {
                    None => break,
                    Some(Token::Newline | Token::ParagraphBreak) => break,
                    Some(_) => {
                        url_end = self.lexer.span().end;
                        self.bump();
                    }
                }
                continue;
            }
            match &self.current {
                Some(Token::Pipe) => {
                    has_pipe = true;
                    url_end = self.lexer.span().start;

                    self.bump(); // consume '|'
                    break;
                }
                Some(Token::RBracket) => {
                    url_end = self.lexer.span().start;
                    self.bump(); // consume ']'
                    closed = true;

                    break;
                }
                Some(Token::Newline | Token::ParagraphBreak) => {
                    // Links cannot cross lines/paragraphs

                    break;
                }
                Some(_) => {
                    url_end = self.lexer.span().end;

                    self.bump();
                }
                None => break,
            }
        }

        let raw_url_trimmed = self.lexer.source()[url_start..url_end].trim();
        let url: Cow<'a, str> = unescape(raw_url_trimmed);

        // Autolink
        if closed && !url.is_empty() {
            return Inline::Link {
                text: url.clone(),
                url,
            };
        }

        if has_pipe {
            // TvT
            self.skip_whitespace();
            let label_start = self.lexer.span().start;
            let mut label_end = label_start;

            while self.current.is_some() {
                if self.current == Some(Token::Backslash) {
                    label_end = self.lexer.span().end;
                    self.bump();
                    match &self.current {
                        None => break,
                        Some(Token::Newline | Token::ParagraphBreak) => break,
                        Some(_) => {
                            label_end = self.lexer.span().end;
                            self.bump();
                        }
                    }
                    continue;
                }
                match &self.current {
                    Some(Token::RBracket) => {
                        label_end = self.lexer.span().start;
                        self.bump(); // consume ']'
                        closed = true;
                        break;
                    }
                    Some(Token::Newline | Token::ParagraphBreak) => {
                        break;
                    }
                    Some(_) => {
                        label_end = self.lexer.span().end;
                        self.bump();
                    }
                    None => break,
                }
            }

            if closed {
                let raw_label_trimmed = self.lexer.source()[label_start..label_end].trim();
                let unescaped_label: Cow<'a, str> = unescape(raw_label_trimmed);
                let text = if unescaped_label.is_empty() {
                    url.clone()
                } else {
                    unescaped_label
                };

                return Inline::Link { url, text };
            }
        }

        // malformed, output as literal text
        let fallback_end = self.lexer.span().start;
        let text = if fallback_end > open_span.start {
            &self.lexer.source()[open_span.start..fallback_end]
        } else {
            "["
        };
        Inline::Text(text)
    }

    fn parse_image(&mut self) -> Inline<'a> {
        let open_span = self.lexer.span();
        self.bump();

        self.skip_whitespace();
        let url_start = match &self.current {
            Some(Token::RBracket | Token::Newline | Token::ParagraphBreak) | None => {
                return Inline::Text(&self.lexer.source()[open_span.start..open_span.end]);
            }
            Some(_) => self.lexer.span().start,
        };

        let mut url_end = url_start;
        let mut has_pipe = false;
        let mut closed = false;

        while self.current.is_some() {
            if self.current == Some(Token::Backslash) {
                url_end = self.lexer.span().end;
                self.bump();
                match &self.current {
                    None => break,
                    Some(Token::Newline | Token::ParagraphBreak) => break,
                    Some(_) => {
                        url_end = self.lexer.span().end;
                        self.bump();
                    }
                }
                continue;
            }
            match &self.current {
                Some(Token::Pipe) => {
                    has_pipe = true;
                    url_end = self.lexer.span().start;
                    self.bump();
                    break;
                }
                Some(Token::RBracket) => {
                    url_end = self.lexer.span().start;
                    self.bump();
                    closed = true;
                    break;
                }
                Some(Token::Newline | Token::ParagraphBreak) => break,
                Some(_) => {
                    url_end = self.lexer.span().end;
                    self.bump();
                }
                None => break,
            }
        }

        let img_source: Cow<'a, str> = unescape(self.lexer.source()[url_start..url_end].trim());
        let mut alt: Cow<'a, str> = Cow::Borrowed("");

        if has_pipe {
            self.skip_whitespace();
            let alt_start = self.lexer.span().start;
            let mut alt_end = alt_start;

            while self.current.is_some() {
                if self.current == Some(Token::Backslash) {
                    alt_end = self.lexer.span().end;
                    self.bump();
                    match &self.current {
                        None => break,
                        Some(Token::Newline | Token::ParagraphBreak) => break,
                        Some(_) => {
                            alt_end = self.lexer.span().end;
                            self.bump();
                        }
                    }
                    continue;
                }
                match &self.current {
                    Some(Token::RBracket) => {
                        alt_end = self.lexer.span().start;
                        self.bump();
                        closed = true;
                        break;
                    }
                    Some(Token::Newline | Token::ParagraphBreak) => break,
                    Some(_) => {
                        alt_end = self.lexer.span().end;
                        self.bump();
                    }
                    None => break,
                }
            }
            alt = unescape(self.lexer.source()[alt_start..alt_end].trim());
        }

        if !closed || img_source.is_empty() {
            let fallback_end = self.lexer.span().start;
            let text = if fallback_end > open_span.start {
                &self.lexer.source()[open_span.start..fallback_end]
            } else {
                &self.lexer.source()[open_span.start..open_span.end]
            };
            return Inline::Text(text);
        }

        Inline::Image { img_source, alt }
    }
}
// tested by good enough tm
