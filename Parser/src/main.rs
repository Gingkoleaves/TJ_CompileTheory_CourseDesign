use std::env;
use std::fs;
use std::io::{self, Read};
use std::process::ExitCode;

use my_lexer::lex;
use my_parser::parse_tokens;

fn main() -> ExitCode {
    let source = match read_source() {
        Ok(source) => source,
        Err(message) => {
            eprintln!("{message}");
            return ExitCode::from(1);
        }
    };

    let lex_result = lex(&source);

    for token in &lex_result.tokens {
        println!(
            "{:>4}:{:<4} {:<18} {}",
            token.position.line, token.position.column, token.kind, token.lexeme
        );
    }

    if !lex_result.errors.is_empty() {
        for error in &lex_result.errors {
            eprintln!(
                "lex error at {}:{}: {}",
                error.position.line, error.position.column, error.message
            );
        }
        return ExitCode::from(1);
    }

    match parse_tokens(&lex_result.tokens) {
        Ok(()) => {
            println!("syntax analysis succeeded");
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("{error}");
            ExitCode::from(1)
        }
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
