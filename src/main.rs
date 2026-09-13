use std::fs;
use std::io::{self, Write};
use std::path::PathBuf;

use crate::lexer::Token;
use crate::{assembler::Assembler, parser::parse};
use clap::Parser;
use logos::Logos;

mod assembler;
mod ast;
mod lexer;
mod parser;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Cli {
    input: PathBuf,

    /// Place the output into <file> (defaults to standard output)
    #[arg(short = 'o', long = "output", value_name = "FILE")]
    output: Option<PathBuf>,

    /// Dump the Abstract Syntax Tree (AST) for debugging purposes
    #[arg(long = "dump-ast")]
    dump_ast: bool,

    /// Dump tokens
    #[arg(long = "dump-tokens")]
    dump_tokens: bool,
}

fn main() {
    let cli = Cli::parse();

    let source = fs::read_to_string(&cli.input).unwrap_or_else(|err| {
        eprintln!(
            "markupc: fatal error: cannot read input file '{}': {}",
            cli.input.display(),
            err
        );
        std::process::exit(1);
    });

    let doc = parse(&source);
    if cli.dump_tokens {
        println!(
            "{:#?}",
            Token::lexer(&source)
                .map(|f| format!("{:?}", (f)))
                .collect::<Vec<String>>()
        );
    }
    if cli.dump_ast {
        println!("{:#?}", doc);
        return;
    }

    let asm = Assembler::default();
    let html_output = asm.assemble(&doc);

    match cli.output {
        Some(out_path) => {
            // User provided an `-o` flag, write to file
            fs::write(&out_path, html_output).unwrap_or_else(|err| {
                eprintln!(
                    "markupc: fatal error: cannot write to output file '{}': {}",
                    out_path.display(),
                    err
                );
                std::process::exit(1);
            });
        }
        None => {
            // shit the contents to stdout
            let mut stdout = io::stdout();
            if let Err(err) = stdout.write_all(html_output.as_bytes()) {
                eprintln!("markupc: fatal error: failed to write to stdout: {}", err);
                std::process::exit(1);
            }
        }
    }
}
