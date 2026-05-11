use std::env;
use std::fs;
use std::io::{self, Read};
use std::path::Path;
use std::process::ExitCode;

use easy_lexer::lex;
use easy_parser::parse_program_ast;
use serde_json;

fn main() -> ExitCode {
    let (source, input_path) = match read_source() {
        Ok(result) => result,
        Err(message) => {
            eprintln!("{message}");
            return ExitCode::from(1);
        }
    };

    let lex_result = lex(&source);

    if !lex_result.errors.is_empty() {
        for error in &lex_result.errors {
            eprintln!(
                "lex error at {}:{}: {}",
                error.position.line, error.position.column, error.message
            );
        }
        return ExitCode::from(1);
    }

    match parse_program_ast(&lex_result.tokens) {
        Ok(ast) => match serde_json::to_string_pretty(&ast) {
            Ok(json) => {
                // If input from file, save AST to file; otherwise print to stdout
                if let Some(path) = input_path {
                    let ast_path = format!(
                        "{}_ast.json",
                        Path::new(&path)
                            .file_stem()
                            .and_then(|s| s.to_str())
                            .unwrap_or("output")
                    );
                    match fs::write(&ast_path, &json) {
                        Ok(_) => {
                            println!("✓ AST saved to: {}", ast_path);
                            ExitCode::SUCCESS
                        }
                        Err(error) => {
                            eprintln!("failed to write AST file: {error}");
                            ExitCode::from(1)
                        }
                    }
                } else {
                    println!("{}", json);
                    ExitCode::SUCCESS
                }
            }
            Err(error) => {
                eprintln!("failed to serialize AST: {error}");
                ExitCode::from(1)
            }
        },
        Err(error) => {
            eprintln!("{error}");
            ExitCode::from(1)
        }
    }
}

fn read_source() -> Result<(String, Option<String>), String> {
    let args: Vec<String> = env::args().collect();
    match args.len() {
        1 => read_stdin()
            .map(|s| (s, None))
            .map_err(|error| format!("failed to read stdin: {error}")),
        2 => {
            let path = &args[1];
            fs::read_to_string(path)
                .map(|s| (s, Some(path.clone())))
                .map_err(|error| format!("failed to read {}: {error}", path))
        }
        _ => Err(format!("usage: {} [source-file]", args[0])),
    }
}

fn read_stdin() -> io::Result<String> {
    let mut buffer = String::new();
    io::stdin().read_to_string(&mut buffer)?;
    Ok(buffer)
}
