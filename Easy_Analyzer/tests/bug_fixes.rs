//! 中等 BUG 修复后的最小复现测试。
//! - BUG #4：空数组字面量 `let a:[i32;0]=[];` 不应报"类型不匹配"
//! - BUG #5：Server `TokenView.type_enum` serde 字段名（由 Easy_Server 的测试覆盖；
//!   此处仅作回归占位以集中跟踪修复点）
//! - BUG #6：函数名作实参时应报"实参类型不一致"而非"未声明"

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
fn bug4_empty_array_literal_zero_length_ok() {
    let r = run("fn main(){ let a:[i32;0]=[]; }");
    assert!(
        r.semantic_errors.is_empty(),
        "[i32;0] = [] 应被接受，实际报错：{:?}",
        r.semantic_errors
    );
}

#[test]
fn bug4_empty_array_still_rejects_mismatched_length() {
    // 空数组对长度非零的声明仍应报错（长度不匹配，不是元素类型）
    let r = run("fn main(){ let a:[i32;2]=[]; }");
    assert!(
        !r.semantic_errors.is_empty(),
        "[i32;2] = [] 应报长度不匹配，实际无错"
    );
}

#[test]
fn bug6_function_name_as_argument_reports_type_mismatch() {
    // PDF program_3_3__4: 用函数名作实参传给期望 i32 的形参 → 期望"实参类型不一致"
    let r = run(
        r#"
        fn callee(a:i32) {}
        fn helper() {}
        fn main() {
            callee(helper);
        }
        "#,
    );
    let msgs: Vec<String> = r.semantic_errors.iter().map(|e| e.message.clone()).collect();
    assert!(
        msgs.iter().any(|m| m.contains("实参") && m.contains("类型")),
        "应报实参类型不一致，实际错误：{:?}",
        msgs
    );
    assert!(
        !msgs.iter().any(|m| m.contains("`helper`") && m.contains("未声明")),
        "不应把 `helper`（函数名）报为未声明变量，实际：{:?}",
        msgs
    );
}

#[test]
fn bug6_truly_undeclared_still_reports_undeclared() {
    // 回归：真正未声明的标识符仍应报"未声明"，不能被回退误吞
    let r = run("fn main(){ let x:i32 = nope; }");
    assert!(
        r.semantic_errors
            .iter()
            .any(|e| e.message.contains("未声明")),
        "纯未声明标识符应保留'未声明'报错：{:?}",
        r.semantic_errors
    );
}
