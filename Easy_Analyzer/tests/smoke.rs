//! 冒烟测试：对若干小程序验证语义错误与四元式生成。

use easy_analyzer::analyze;
use easy_lexer::lex;
use easy_parser::parse_program_ast;

fn run(src: &str) -> easy_analyzer::AnalysisResult {
    let lex = lex(src);
    assert!(lex.errors.is_empty(), "lexer errors: {:?}", lex.errors);
    let program = parse_program_ast(&lex.tokens).expect("parse failed");
    analyze(&program)
}

#[test]
fn arithmetic_ok() {
    let r = run("fn main(){ let mut a:i32=1; let b:i32=a+2*3; }");
    assert!(r.semantic_errors.is_empty(), "{:?}", r.semantic_errors);
    // 四元式应包含 *, +, =
    let ops: Vec<&str> = r.quadruples.iter().map(|q| q.op.as_str()).collect();
    assert!(ops.contains(&"*"));
    assert!(ops.contains(&"+"));
    assert!(ops.contains(&"="));
}

#[test]
fn use_undeclared() {
    let r = run("fn main(){ a = 1; }");
    assert!(
        r.semantic_errors
            .iter()
            .any(|e| e.message.contains("未声明")),
        "expected undeclared error, got {:?}",
        r.semantic_errors
    );
}

#[test]
fn immutable_reassign() {
    let r = run("fn main(){ let c:i32=1; c=2; }");
    assert!(
        r.semantic_errors
            .iter()
            .any(|e| e.message.contains("不可变变量")),
        "{:?}",
        r.semantic_errors
    );
}

#[test]
fn assign_type_mismatch() {
    let r = run("fn main(){ let mut a:i32=0; a = 1==1; }");
    assert!(
        r.semantic_errors
            .iter()
            .any(|e| e.message.contains("类型") && e.message.contains("不匹配")),
        "{:?}",
        r.semantic_errors
    );
}

#[test]
fn use_before_assign() {
    let r = run("fn main(){ let mut a:i32; let mut b:i32 = a; }");
    assert!(
        r.semantic_errors
            .iter()
            .any(|e| e.message.contains("赋值前")),
        "{:?}",
        r.semantic_errors
    );
}

#[test]
fn return_type_mismatch_empty() {
    let r = run("fn f()->i32{ return; }");
    assert!(
        r.semantic_errors
            .iter()
            .any(|e| e.message.contains("return")),
        "{:?}",
        r.semantic_errors
    );
}

#[test]
fn return_type_mismatch_value() {
    let r = run("fn f(){ return 1; }");
    assert!(
        r.semantic_errors
            .iter()
            .any(|e| e.message.contains("return") && e.message.contains("不能带表达式")),
        "{:?}",
        r.semantic_errors
    );
}

#[test]
fn call_arg_count() {
    let r = run("fn f(){} fn main(){ f(1); }");
    assert!(
        r.semantic_errors
            .iter()
            .any(|e| e.message.contains("形参数量")),
        "{:?}",
        r.semantic_errors
    );
}

#[test]
fn call_arg_type() {
    let r = run("fn f(a:i32){} fn main(){ f(1==1); }");
    assert!(
        r.semantic_errors
            .iter()
            .any(|e| e.message.contains("实参类型")),
        "{:?}",
        r.semantic_errors
    );
}

#[test]
fn break_outside_loop() {
    let r = run("fn main(){ break; }");
    assert!(
        r.semantic_errors
            .iter()
            .any(|e| e.message.contains("循环体内")),
        "{:?}",
        r.semantic_errors
    );
}

#[test]
fn while_ir() {
    let r = run("fn main(){ let mut i:i32=0; while i<3 { i = i+1; } }");
    assert!(r.semantic_errors.is_empty(), "{:?}", r.semantic_errors);
    let ops: Vec<&str> = r.quadruples.iter().map(|q| q.op.as_str()).collect();
    assert!(ops.contains(&"LABEL"));
    assert!(ops.contains(&"IF_FALSE"));
    assert!(ops.contains(&"GOTO"));
    assert!(ops.contains(&"<"));
}

#[test]
fn if_ir() {
    let r = run("fn main(){ let a:i32=0; if a<1 { } else { } }");
    assert!(r.semantic_errors.is_empty(), "{:?}", r.semantic_errors);
    let ops: Vec<&str> = r.quadruples.iter().map(|q| q.op.as_str()).collect();
    assert!(ops.contains(&"IF_FALSE"));
    assert!(ops.contains(&"GOTO"));
    assert!(ops.contains(&"LABEL"));
}

#[test]
fn function_call_with_return() {
    let r = run("fn add(a:i32,b:i32)->i32{ return a+b; } fn main(){ let c:i32 = add(1,2); }");
    assert!(r.semantic_errors.is_empty(), "{:?}", r.semantic_errors);
    let ops: Vec<&str> = r.quadruples.iter().map(|q| q.op.as_str()).collect();
    assert!(ops.contains(&"PARAM"));
    assert!(ops.contains(&"CALL"));
    assert!(ops.contains(&"RETURN"));
}
