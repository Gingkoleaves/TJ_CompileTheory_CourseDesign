//! CLI：从 stdin 或文件读取源代码，输出语义错误与四元式 JSON。

use std::env;
use std::fs;
use std::io::{self, Read};
use std::process;

fn main() {
    let args: Vec<String> = env::args().collect();
    let source = if args.len() >= 2 {
        match fs::read_to_string(&args[1]) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("无法读取文件 {}: {}", &args[1], e);
                process::exit(1);
            }
        }
    } else {
        let mut buf = String::new();
        if io::stdin().read_to_string(&mut buf).is_err() {
            eprintln!("无法读取 stdin");
            process::exit(1);
        }
        buf
    };

    let lex = easy_lexer::lex(&source);
    if !lex.errors.is_empty() {
        for e in &lex.errors {
            eprintln!("[词法错误] {}:{}: {}", e.position.line, e.position.column, e.message);
        }
        process::exit(1);
    }

    let program = match easy_parser::parse_program_ast(&lex.tokens) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("[语法错误] {}", e);
            process::exit(1);
        }
    };

    let result = easy_analyzer::analyze(&program);
    match serde_json::to_string_pretty(&result) {
        Ok(s) => println!("{}", s),
        Err(e) => {
            eprintln!("序列化失败: {}", e);
            process::exit(1);
        }
    }
}
