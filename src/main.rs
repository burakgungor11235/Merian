use std::fs;
use std::io::{self, Write};
use std::path::PathBuf;

use clap::Parser;

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

    /// Dump the AST
    #[arg(long = "dump-ast")]
    dump_ast: bool,

    /// Dump tokens
    #[arg(long = "dump-tokens")]
    dump_tokens: bool,

    /// Cat
    #[arg(long = "cat")]
    cat: bool,
}

fn main() {
    let cli = Cli::parse();
    run(&cli);
}

fn handle_cat() {
    #[cfg(target_os = "windows")]
    let _ = std::process::Command::new("cmd")
        .args(["/C", "start", "https://cataas.com/cat/gif"])
        .spawn();

    #[cfg(target_os = "macos")]
    let _ = std::process::Command::new("open")
        .arg("https://cataas.com/cat/gif")
        .spawn();

    #[cfg(target_os = "linux")]
    let _ = std::process::Command::new("xdg-open")
        .arg("https://cataas.com/cat/gif")
        .spawn();
}

fn run(cli: &Cli) {
    let source = fs::read_to_string(&cli.input).unwrap_or_else(|err| {
        eprintln!(" cannot read input file '{}': {err}", cli.input.display(),);
        std::process::exit(1);
    });

    if cli.cat {
        handle_cat();
    }
    let doc = parser::parse(&source);

    if cli.dump_tokens {
        use lexer::Token;
        use logos::Logos;

        let tokens: Vec<_> = Token::lexer(&source).map(|t| format!("{t:?}")).collect();
        println!("{tokens:#?}");
    }

    if cli.dump_ast {
        println!("{doc:#?}");
    }

    let html_output = assembler::Assembler::default().assemble(&doc);

    match &cli.output {
        Some(out_path) => {
            fs::write(out_path, &html_output).unwrap_or_else(|err| {
                eprintln!(
                    "error: cannot write to output file '{}': {err}",
                    out_path.display(),
                );
                std::process::exit(1);
            });
        }
        None => {
            // throw the contents to stdout
            if let Err(err) = io::stdout().write_all(html_output.as_bytes()) {
                eprintln!("error: failed to write to stdout (how tf?): {err}");
                std::process::exit(1);
            }
        }
    }
}
