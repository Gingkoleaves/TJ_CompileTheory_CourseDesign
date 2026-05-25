//! Round-3 追加探测：进一步针对 return/赋值/range 等边界场景。

use easy_analyzer::analyze;
use easy_lexer::lex;
use easy_parser::parse_program_ast;

fn run(src: &str) -> easy_analyzer::AnalysisResult {
    let lex = lex(src);
    assert!(lex.errors.is_empty(), "lex errors: {:?}", lex.errors);
    let program = parse_program_ast(&lex.tokens).expect("parse failed");
    analyze(&program)
}

// HH: `return ();` 在 unit 函数中应被接受 —— 等同于 `return;`
#[test]
fn cand_hh_return_unit_literal_in_unit_function() {
    let r = run("fn main(){ return (); }");
    println!("[OBSERVED] HH errors: {:?}", r.semantic_errors);
    // 期望：无错。但当前实现见到 `return <expr>`，expected=Unit，会直接报
    // "函数无返回类型，return 不能带表达式"。这是漏判：`()` 字面量类型也是 Unit，应允许。
    let has = r
        .semantic_errors
        .iter()
        .any(|e| e.message.contains("不能带表达式"));
    println!("[OBSERVED] HH has '不能带表达式' error: {}", has);
}

// II: `return tuple_expr;` 在 unit 函数中（更隐蔽场景）
#[test]
fn cand_ii_return_block_unit_in_unit_function() {
    let r = run("fn main(){ return {}; }");  // 块表达式无尾返回 Unit
    println!("[OBSERVED] II errors: {:?}", r.semantic_errors);
}

// JJ: 函数体尾表达式为 `()`：等同于无尾，应不报错
#[test]
fn cand_jj_tail_unit_literal_in_unit_function() {
    let r = run("fn main() { () }");
    println!("[OBSERVED] JJ errors: {:?}", r.semantic_errors);
}

// KK: 把 range 存到变量再做 for 迭代 —— 应允许或报清晰错误，不应 panic
#[test]
fn cand_kk_range_value_stored_and_iterated() {
    let r = run("fn main(){ let r = 0..3; for i in r { let _x:i32 = i; } }");
    println!("[OBSERVED] KK errors: {:?}", r.semantic_errors);
}

// LL: 数组下标用 i32 变量（动态下标），不应报"静态越界"，但要发射 INDEX
#[test]
fn cand_ll_dynamic_index_no_static_oob() {
    let r = run("fn main(){ let a:[i32;3]=[1,2,3]; let mut i:i32 = 1; let b:i32 = a[i]; }");
    println!("[OBSERVED] LL errors: {:?}", r.semantic_errors);
    assert!(r.semantic_errors.is_empty(), "动态下标不应报越界: {:?}", r.semantic_errors);
}

// MM: 数组元素是引用类型：`let a:[&i32; 2] = [&x, &y];`
#[test]
fn cand_mm_array_of_references() {
    let r = run(r#"
        fn main(){
            let x:i32 = 1;
            let y:i32 = 2;
            let a:[&i32; 2] = [&x, &y];
            let b:&i32 = a[0];
        }
    "#);
    println!("[OBSERVED] MM errors: {:?}", r.semantic_errors);
}

// NN: 元组中含函数名 `(g, 1)` —— 当前会创建 Tuple{[Function, I32]}，后续使用是否合理
#[test]
fn cand_nn_function_in_tuple_literal() {
    let r = run(r#"
        fn g()->i32 { 1 }
        fn main(){
            let t = (g, 1);
        }
    "#);
    println!("[OBSERVED] NN errors: {:?}", r.semantic_errors);
}

// OO: 不可变变量首次 init 后再赋值给同名形参（shadowing） —— 不应报"不可变变量再赋值"
#[test]
fn cand_oo_shadowing_with_let_then_assign() {
    let r = run(r#"
        fn main(){
            let a:i32 = 1;
            let mut a:i32 = 2;  // shadow，且 mut
            a = 3;
        }
    "#);
    println!("[OBSERVED] OO errors: {:?}", r.semantic_errors);
    assert!(r.semantic_errors.is_empty(), "shadowing+mut 不应报错: {:?}", r.semantic_errors);
}

// PP: `&` 表达式作 RHS 时，类型应为 Ref —— 与声明类型 i32 不匹配应报错
#[test]
fn cand_pp_assigning_ref_to_i32_var() {
    let r = run(r#"
        fn main(){
            let x:i32 = 1;
            let y:i32 = &x;
        }
    "#);
    println!("[OBSERVED] PP errors: {:?}", r.semantic_errors);
    let has = r.semantic_errors.iter().any(|e| e.message.contains("不匹配"));
    assert!(has, "应报类型不匹配: {:?}", r.semantic_errors);
}

// QQ: gen_for 在错误恢复路径下 `for i in expr {}`，若 expr 是 range 中的右侧不是 i32，
//     比如 `for i in 0..(g())` 其中 g 返回 i32：应可工作
#[test]
fn cand_qq_for_with_call_as_range_end() {
    let r = run(r#"
        fn end()->i32 { 5 }
        fn main(){ for i in 0..end() { let _x:i32 = i; } }
    "#);
    println!("[OBSERVED] QQ errors: {:?}", r.semantic_errors);
    assert!(r.semantic_errors.is_empty(), "for+call 不应报错: {:?}", r.semantic_errors);
}

// RR: `break` 不带值 + `break val;` 混用，类型应推断为带值的类型
#[test]
fn cand_rr_loop_with_mixed_break_kinds() {
    let r = run(r#"
        fn main(){
            let x:i32 = loop {
                if 1 == 1 { break; }
                break 42;
            };
        }
    "#);
    println!("[OBSERVED] RR errors: {:?}", r.semantic_errors);
    // 现行实现：第一个 break（无值）走 else 分支不更新 break_type；
    // 第二个 break 42 设 break_type = I32；
    // 最终 loop 类型推断为 I32 —— 与声明 i32 兼容。
    // 但实际上无值 break 也"break 出了 loop"，类型应当是 Unit，因此应当报"类型不一致"。
    // 该情况是否漏检？
    let has_inconsistent = r
        .semantic_errors
        .iter()
        .any(|e| e.message.contains("不一致"));
    println!("[OBSERVED] RR reports break-type inconsistency: {}", has_inconsistent);
}

// SS: 函数被声明返回 Unit，函数体显式 `return 1;` —— 已知报错路径，确认
#[test]
fn cand_ss_unit_function_return_value() {
    let r = run("fn main(){ return 1; }");
    println!("[OBSERVED] SS errors: {:?}", r.semantic_errors);
    let has = r.semantic_errors.iter().any(|e| e.message.contains("不能带表达式"));
    assert!(has, "应报'不能带表达式': {:?}", r.semantic_errors);
}

// TT: 同名函数与变量 —— `fn x(){} fn main(){ let x:i32 = 1; x(); }` 调用应解析为函数还是变量？
#[test]
fn cand_tt_function_and_var_same_name() {
    let r = run("fn x()->i32 { 1 } fn main(){ let x:i32 = 2; let y:i32 = x(); }");
    println!("[OBSERVED] TT errors: {:?}", r.semantic_errors);
    // gen_call 优先查函数表 -> sig 存在 -> 当作函数调用。
    // 但 gen_identifier(x) 优先查变量表 -> 找到 i32 变量 -> 作 i32 用。
    // 这两种解析在同一作用域内表现不同，可能引起歧义。
    // 期望：要么明确报"标识符 x 既是函数名又是变量名"，要么至少一致。
}
