use std::fs;
use std::io::{self, Write};
use std::path::PathBuf;

use clap::Parser;

use merian_lang::assembler;
use merian_lang::backend::ast_to_ir_lower::lower;
use merian_lang::backend::resolver::resolve;
use merian_lang::{lexer, parser};

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Cli {
    /// Input file (required unless -c/--calculator is used)
    #[arg(required_unless_present = "calc")]
    input: Option<PathBuf>,

    /// Place the output into <file> (defaults to standard output)
    #[arg(short = 'o', long = "output", value_name = "FILE")]
    output: Option<PathBuf>,

    /// Dump the AST
    #[arg(long = "dump-ast")]
    dump_ast: bool,

    /// Dump the unresolved IR
    #[arg(long = "dump-ir")]
    dump_ir: bool,

    /// Dump the resolved IR
    #[arg(long = "dump-resolved")]
    dump_resolved: bool,

    /// Dump tokens
    #[arg(long = "dump-tokens")]
    dump_tokens: bool,

    /// Cat
    #[arg(long = "cat")]
    cat: bool,

    /// Dump math envelopes: parse trees, free variables, and eval previews
    #[arg(long = "debug-math")]
    debug_math: bool,

    #[arg(long = "calculator", short = 'c', value_name = "EXPR")]
    /// Evaluate a math expression and print the exact result
    calc: Option<String>,
}

fn main() {
    let cli = Cli::parse();
    if let Some(expression) = &cli.calc {
        run_calc(expression, cli.debug_math);
        return;
    }
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

fn run_calc(expression: &str, debug: bool) {
    let parsed = merian_lang::math::expr::parse(expression).unwrap_or_else(|error| {
        if debug {
            eprintln!("parse error: {error:#?}");
        }
        eprintln!("error: {}: {}", error.code, error.message);
        std::process::exit(1);
    });
    if debug {
        print!("{}", merian_lang::math::debug::dump_expression(&parsed));
    }
    let value =
        merian_lang::math::solve::eval(&parsed, &merian_lang::math::solve::EvalCtx::default())
            .unwrap_or_else(|error| {
                eprintln!("error: {}: {}", error.code, error.message);
                std::process::exit(1);
            });
    println!("{}", value.to_plain_string());
}

fn run(cli: &Cli) {
    let Some(input) = cli.input.as_deref() else {
        eprintln!("error: no input file given");
        std::process::exit(1);
    };
    let source = fs::read_to_string(input).unwrap_or_else(|err| {
        eprintln!(" cannot read input file '{}': {err}", input.display(),);
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

    let ir = lower(&doc);
    if cli.dump_ir {
        println!("{ir:#?}");
    }
    if cli.debug_math {
        print!("{}", merian_lang::math::debug::dump_document_math(&ir));
    }

    let (resolved, diags) = resolve(ir);
    for d in &diags {
        eprintln!("warning: {d}");
    }
    if cli.dump_resolved {
        println!("{resolved:#?}");
    }

    let html_output = assembler::Backend::emit(assembler::Assembler::default(), &resolved);

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
