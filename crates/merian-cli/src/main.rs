use std::{env, fs::read_to_string};

use merian_frontend::ast::Asp;
use merian_lower::{
    backends::{self, merian_html::build},
    lower::{LowerContext, Lowerer},
};

fn main() -> Result<(), std::io::Error> {
    let args: Vec<String> = env::args().collect();

    let source = args[1].clone();
    let dest = args[2].clone();

    let contents = read_to_string(source)?;
    let asp = Asp::from_string(&contents);

    let lowerer = backends::merian_html::HtmlLowerer;
    let mut lowererctx = LowerContext::default();
    let lowered = lowerer.lower(&asp, &mut lowererctx).unwrap();

    build(lowered, dest)
}
