//! Command-line entry point for the lexer binary.
//!
//! It reads source code from a file or standard input and prints each token
//! with its position, kind, and original lexeme.

use std::env;
use std::fs;
use std::io::{self, Read};
use std::process::ExitCode;

use easy_lexer::lex;

fn main() -> ExitCode {
    let source = match read_source() {
        Ok(source) => source,
        Err(message) => {
            eprintln!("{message}");
            return ExitCode::from(1);
        }
    };

    let result = lex(&source);

    for token in &result.tokens {
        println!(
            "{:>4}:{:<4} {:<18} {}",
            token.position.line, token.position.column, token.kind, token.lexeme
        );
    }

    if result.errors.is_empty() {
        ExitCode::SUCCESS
    } else {
        for error in &result.errors {
            eprintln!(
                "lex error at {}:{}: {}",
                error.position.line, error.position.column, error.message
            );
        }
        ExitCode::from(1)
    }
}

fn read_source() -> Result<String, String> {
    let args: Vec<String> = env::args().collect();
    match args.len() {
        1 => read_stdin().map_err(|error| format!("failed to read stdin: {error}")),
        2 => fs::read_to_string(&args[1])
            .map_err(|error| format!("failed to read {}: {error}", args[1])),
        _ => Err(format!("usage: {} [source-file]", args[0])),
    }
}

fn read_stdin() -> io::Result<String> {
    let mut buffer = String::new();
    io::stdin().read_to_string(&mut buffer)?;
    Ok(buffer)
}
