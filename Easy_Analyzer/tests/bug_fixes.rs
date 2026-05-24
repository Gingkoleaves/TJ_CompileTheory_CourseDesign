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

#[test]
fn bug7_duplicate_function_emits_single_ir() {
    // 重复函数定义应报错且 IR 中只保留首次函数体（一份 FUNC/END_FUNC）
    let r = run("fn f(){} fn f(){} fn main(){}");
    assert!(
        r.semantic_errors
            .iter()
            .any(|e| e.message.contains("重复定义")),
        "应报'重复定义'：{:?}",
        r.semantic_errors
    );
    let func_f_count = r
        .quadruples
        .iter()
        .filter(|q| q.op == "FUNC" && q.arg1 == "f")
        .count();
    assert_eq!(
        func_f_count, 1,
        "重复函数 f 在 IR 中应只出现一次，实际 {} 次",
        func_f_count
    );
}

#[test]
fn bug8_non_unit_function_without_return_still_has_terminator() {
    // `fn f()->i32 { }` 之前不发任何 RETURN；修复后函数末尾必有 RETURN 终结子
    let r = run("fn f()->i32 { } fn main(){}");
    let start = r
        .quadruples
        .iter()
        .position(|q| q.op == "FUNC" && q.arg1 == "f")
        .expect("FUNC f missing");
    let end = r.quadruples[start..]
        .iter()
        .position(|q| q.op == "END_FUNC" && q.arg1 == "f")
        .map(|i| i + start)
        .expect("END_FUNC f missing");
    let has_return = r.quadruples[start..end].iter().any(|q| q.op == "RETURN");
    assert!(
        has_return,
        "fn f()->i32{{}} 的 IR 缺少 RETURN 终结子：{:?}",
        &r.quadruples[start..=end]
    );
}

#[test]
fn bug9_unit_if_expr_does_not_emit_assign_to_temp() {
    // `if 1>0 {} else {}` 两分支都是 Unit，不应分配/赋值 if 结果 temp
    let r = run("fn main() { if 1>0 { } else { } }");
    let assigns_to_temp = r
        .quadruples
        .iter()
        .filter(|q| q.op == "=" && q.result.starts_with('t'))
        .count();
    assert_eq!(
        assigns_to_temp, 0,
        "Unit/Unit if 不应产生针对 temp 的赋值 IR：{:?}",
        r.quadruples
    );
}

#[test]
fn bug11_block_expr_reports_uninferred_inner_var() {
    // 块表达式内部的无法推断变量也应被报出
    let r = run("fn main() { let x = { let y; 1 }; }");
    assert!(
        r.semantic_errors
            .iter()
            .any(|e| e.message.contains("y") && e.message.contains("无法推断")),
        "块表达式内未推断变量应被报出：{:?}",
        r.semantic_errors
    );
}

#[test]
fn bug12_break_type_mismatch_skips_result_assign() {
    // 同一 loop 中第二个 break 类型与第一个不一致：报类型不一致，且不发射 = 污染结果 temp
    let r = run(
        r#"
        fn main() {
            let mut x:i32 = loop {
                break 1;
                break 1==1;
            };
        }
        "#,
    );
    assert!(
        r.semantic_errors
            .iter()
            .any(|e| e.message.contains("类型不一致")),
        "应报多个 break 类型不一致：{:?}",
        r.semantic_errors
    );
    // 第一个 break 发 =；第二个 break 应跳过 =，loop 内只发 1 次 (=, 1, _, t_result)
    let result_assigns = r
        .quadruples
        .iter()
        .filter(|q| q.op == "=" && q.arg1 == "1" && q.result.starts_with('t'))
        .count();
    assert_eq!(
        result_assigns, 1,
        "类型不一致的 break 不应再发 =，实际 result 赋值次数 {}：{:?}",
        result_assigns, r.quadruples
    );
}
