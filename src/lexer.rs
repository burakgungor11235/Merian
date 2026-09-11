use logos::Logos;

#[derive(Logos, Debug, PartialEq)]
pub enum Token<'a> {
    #[token("\n\n")]
    ParagraphBreak,

    #[token("\n")]
    Newline,

    #[regex(r"#[0-9]+(?:\.[0-9]+)*")]
    HeadingMarker(&'a str),

    #[token("**")]
    Bold,
    #[token("__")]
    Italic,
    #[token("==")]
    Underline,
    #[token("~~")]
    Striketrhu,

    #[token("`")]
    Backtick,
    #[token("[")]
    LBracket,
    #[token("]")]
    RBracket,

    #[regex(r"[ \t]+")]
    Whitespace(&'a str),

    #[regex(r"[^\s#*_~=`\[\]]+")]
    Text(&'a str),

    #[regex(r"[#*_~=]")]
    Punctuation(&'a str),
}
#[cfg(test)]
mod tests {

    macro_rules! wrap {
        ($t:expr) => {
            Some(Ok($t))
        };
    }

    use crate::lexer::Token::*;

    use super::*;
    #[test]
    fn simple() {
        let mut l = Token::lexer("simple\n\ntext");
        assert_eq!(l.next(), wrap!(Token::Text("simple")));
        assert_eq!(l.next(), wrap!(Token::ParagraphBreak));
        assert_eq!(l.next(), wrap!(Token::Text("text")));
    }

    #[test]
    fn heading() {
        let mut lexer = Token::lexer(
            "#1\n\
     #1.1\n\
     #1.1.1\n\
        ",
        );

        assert_eq!(lexer.next(), Some(Ok(Token::HeadingMarker("#1"))));
        assert_eq!(lexer.next(), Some(Ok(Token::Newline)));
        assert_eq!(lexer.next(), Some(Ok(Token::HeadingMarker("#1.1"))));
        assert_eq!(lexer.next(), Some(Ok(Token::Newline)));
        assert_eq!(lexer.next(), Some(Ok(Token::HeadingMarker("#1.1.1"))));
        assert_eq!(lexer.next(), Some(Ok(Token::Newline)));
    }
    #[test]
    fn bold_italic() {
        let mut lexer = Token::lexer("**start**__end__~~lol~~==important==");
        assert_eq!(lexer.next(), wrap!(Bold));
        assert_eq!(lexer.next(), wrap!(Text("start")));
        assert_eq!(lexer.next(), wrap!(Bold));
        assert_eq!(lexer.next(), wrap!(Italic));
        assert_eq!(lexer.next(), wrap!(Text("end")));
        assert_eq!(lexer.next(), wrap!(Italic));
        assert_eq!(lexer.next(), wrap!(Striketrhu));
        assert_eq!(lexer.next(), wrap!(Text("lol")));
        assert_eq!(lexer.next(), wrap!(Striketrhu));
        assert_eq!(lexer.next(), wrap!(Underline));
        assert_eq!(lexer.next(), wrap!(Text("important")));
        assert_eq!(lexer.next(), wrap!(Underline));
    }
}
