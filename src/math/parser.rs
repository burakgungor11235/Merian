use crate::ast::MathSource;

use super::MathMode;
use logos::Span;

pub(crate) enum InlineMathParse<'a> {
    Math {
        source: MathSource<'a>,
        consumed: usize,
    },
    Literal {
        end: usize,
        consumed: usize,
    },
    NotMath,
}

pub(crate) fn parse_inline_math<'a>(
    source: &'a str,
    remainder: &'a str,
    open_start: usize,
    remainder_start: usize,
) -> InlineMathParse<'a> {
    let Some((mode, sigil_len)) = math_inline_mode(remainder) else {
        return InlineMathParse::NotMath;
    };

    let Some(close_index) = find_math_close(remainder, sigil_len) else {
        let line_len = remainder.find('\n').unwrap_or(remainder.len());
        return InlineMathParse::Literal {
            end: open_start + 1 + line_len,
            consumed: line_len,
        };
    };

    let payload_span = Span {
        start: remainder_start + sigil_len,
        end: remainder_start + close_index,
    };
    let span = Span {
        start: open_start,
        end: remainder_start + close_index + 1,
    };
    let line = source[..open_start]
        .bytes()
        .filter(|&byte| byte == b'\n')
        .count() as u32
        + 1;

    InlineMathParse::Math {
        source: MathSource {
            mode,
            raw: &source[payload_span.clone()],
            span,
            payload_span,
            line,
        },
        consumed: close_index + 1,
    }
}

fn math_inline_mode(remainder: &str) -> Option<(MathMode, usize)> {
    let candidates = [
        ("?=", MathMode::Both, 2),
        ("m", MathMode::Display, 1),
        ("=", MathMode::Compute, 1),
    ];

    for (prefix, mode, sigil_len) in candidates {
        if remainder.starts_with(prefix)
            && remainder
                .get(sigil_len..)
                .and_then(|rest| rest.chars().next())
                .is_some_and(|c| matches!(c, ' ' | '\t'))
        {
            return Some((mode, sigil_len));
        }
    }

    None
}

fn find_math_close(remainder: &str, start: usize) -> Option<usize> {
    let mut search = start;

    while search < remainder.len() {
        let relative = remainder[search..].find('$')?;
        let index = search + relative;

        if remainder[..index].contains('\n') {
            return None;
        }

        let escaped = remainder[..index]
            .chars()
            .rev()
            .take_while(|&c| c == '\\')
            .count()
            % 2
            == 1;
        if escaped {
            search = index + 1;
            continue;
        }

        return Some(index);
    }

    None
}
