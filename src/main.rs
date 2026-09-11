use std::io::{stdin, stdout};

use crate::parser::parse;

mod ast;
mod lexer;
mod parser;

fn main() {
    loop {
        let mut s = String::new();
        let _ = stdin().read_line(&mut s);
        println!("{:?}", parse(&s));
    }
}
