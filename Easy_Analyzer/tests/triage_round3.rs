//! Round-3 BUG 复审：针对代码审查发现的若干可疑点做实证。
//!
//! 重点：
//! - U: `break value;` 内嵌于 `for/while` 而 `for/while` 嵌于 `loop {}` 时，
//!   loop_exprs.last() 与 loop_labels.last() 不一致，可能把 value 写到外层 loop 的 result_place。
//! - V: `break value;` 在 `while` 循环里是否产生孤立的赋值（cand_g 已观察"无错"，
//!   这里进一步看 IR 中是否真有针对该 value 的赋值副作用）。
//! - W: `break;`（无值）在 `loop {}` 里，break_type 默认为 Unit；
//!   但 gen_loop_expr 返回 break_type.unwrap_or(Type::Unit)，因此 loop 表达式的类型在
//!   只有无值 break 时是 Unit。但 result_place 临时变量从未被写入，
//!   用 loop 表达式做 RValue 是否会拿到未定义值？
//! - X: 函数体最后一条语句已 `return;` 后，仍发射兜底 `RETURN _ _ _`，
//!   IR 不可达指令重复。仅作观察。
//! - Y: 数组下标越界报错后，IR 仍发射 INDEX。下游若想直接执行 IR 可能拿到非法访问。仅观察。
//! - Z: 元组的"运行时下标"（即使是字面整数）`a.0` 与负字面量场景的协同。

use easy_analyzer::analyze;
use easy_lexer::lex;
use easy_parser::parse_program_ast;

fn run(src: &str) -> easy_analyzer::AnalysisResult {
    let lex = lex(src);
    assert!(lex.errors.is_empty(), "lex errors: {:?}", lex.errors);
    let program = parse_program_ast(&lex.tokens).expect("parse failed");
    analyze(&program)
}

// ============================================================
// CANDIDATE BUG U: break-value 跨循环类型上下文窜流
//   loop { for i in 0..3 { break 42; } } 中的 `break 42`
//   应当作用于内层 for，但当前实现把 42 写入外层 loop 的 result_place。
// ============================================================
#[test]
fn cand_u_break_value_in_nested_for_writes_outer_loop_result() {
    let src = r#"
        fn main(){
            let x = loop {
                for i in 0..3 { break 42; }
                break 7;
            };
        }
    "#;
    let r = run(src);

    // 找到所有 break 出现的 quad 及前后赋值
    let quads = &r.quadruples;
    println!(
        "[OBSERVED] U semantic_errors = {:?}",
        r.semantic_errors.iter().map(|e| &e.message).collect::<Vec<_>>()
    );
    for (i, q) in quads.iter().enumerate() {
        println!("  {:>3}: {} {} {} {}", i, q.op, q.arg1, q.arg2, q.result);
    }

    // 真实期望：要么 (a) 报错"break 带值仅 loop 支持"，要么 (b) 把 42 写到 for 的位置（即不要写到外层）。
    // 当前若把 42 写到外层 loop 的 result temp，就是窜流 BUG。

    // 找到外层 loop 的 result_place：它是第一个 `=` 把 7 写到的 temp。
    // 简化判断：检查 "= 42 _ t<N>" 这条 quad 的 result，和 "= 7 _ t<M>" 的 result 是否相同。
    let mut write_42: Option<String> = None;
    let mut write_7: Option<String> = None;
    for q in quads {
        if q.op == "=" && q.arg1 == "42" {
            write_42 = Some(q.result.clone());
        }
        if q.op == "=" && q.arg1 == "7" {
            write_7 = Some(q.result.clone());
        }
    }
    println!(
        "[OBSERVED] write_42 -> {:?}, write_7 -> {:?}",
        write_42, write_7
    );
    if let (Some(a), Some(b)) = (&write_42, &write_7) {
        // 若两者写到同一 temp，说明 42 串到了外层 loop 的 result_place。
        if a == b {
            println!(
                "[BUG-U] break 42 (in inner for) 与 break 7 (outer loop) 写入同一 result temp `{}` —— 窜流",
                a
            );
        }
    }
    // 不强 assert，只观察。
}

// ============================================================
// CANDIDATE BUG V: while 内 `break expr;` 完全无报错，且无副作用赋值（合理 vs 漏检）
// ============================================================
#[test]
fn cand_v_break_value_in_while_no_assignment_emitted() {
    let r = run("fn main(){ while 1 { break 5; } }");
    println!("[OBSERVED] V errors: {:?}", r.semantic_errors);
    for (i, q) in r.quadruples.iter().enumerate() {
        println!("  {:>3}: {} {} {} {}", i, q.op, q.arg1, q.arg2, q.result);
    }
    // 观察：BREAK 5 _ <end_label> 是否出现，前面是否有诡异 `= 5 _ X` 写到不该写的位置。
}

// ============================================================
// CANDIDATE BUG W: 只用无值 break 的 loop 表达式作为 RValue —— result_place 是悬空 temp
//   let x:i32 = loop { break; };  在当前实现下：
//     - break_type = None? 还是 Unit?
//     - 类型推断为 Unit，则与 i32 不匹配应报错。
//     - 若没报错则有漏检。
// ============================================================
#[test]
fn cand_w_loop_with_only_value_less_break_as_i32_rhs() {
    let r = run("fn main(){ let x:i32 = loop { break; }; }");
    println!("[OBSERVED] W errors: {:?}", r.semantic_errors);
    for (i, q) in r.quadruples.iter().enumerate() {
        println!("  {:>3}: {} {} {} {}", i, q.op, q.arg1, q.arg2, q.result);
    }
    // 期望：报"声明类型 i32 与初始化表达式类型 () 不匹配"
    let has = r
        .semantic_errors
        .iter()
        .any(|e| e.message.contains("不匹配") || e.message.contains("类型"));
    println!("[OBSERVED] W has type error: {}", has);
}

// ============================================================
// CANDIDATE BUG X: return 后函数体仍多发一条 RETURN _ _ _
// ============================================================
#[test]
fn cand_x_explicit_return_followed_by_terminator() {
    let r = run("fn f()->i32 { return 1; } fn main(){}");
    println!("[OBSERVED] X errors: {:?}", r.semantic_errors);
    let returns: Vec<_> = r
        .quadruples
        .iter()
        .filter(|q| q.op == "RETURN")
        .collect();
    for q in &returns {
        println!("  RETURN {} {} {}", q.arg1, q.arg2, q.result);
    }
    println!("[OBSERVED] X #RETURN = {}", returns.len());
    // 仅观察：是否有多于 1 个 RETURN（其中一个是末尾兜底 `RETURN _ _ _`）。
}

// ============================================================
// CANDIDATE BUG Y: 数组下标越界后仍发射 INDEX
// ============================================================
#[test]
fn cand_y_oob_index_still_emits_index_quad() {
    let r = run("fn main(){ let a:[i32;3]=[1,2,3]; let b:i32=a[5]; }");
    println!("[OBSERVED] Y errors: {:?}", r.semantic_errors);
    let has_index = r.quadruples.iter().any(|q| q.op == "INDEX");
    println!("[OBSERVED] Y has INDEX quad: {}", has_index);
    // 仅观察。
}

// ============================================================
// CANDIDATE BUG Z: tuple `.0` 上的负字面量场景（句法应不允许，但确认）
// ============================================================
#[test]
fn cand_z_tuple_field_overflow_index() {
    // PDF 9.3 越界已在 semantic.rs 处理；此处确认超大 usize 解析失败时不 panic
    let r = run("fn main(){ let a:(i32,i32)=(1,2); let b:i32=a.999999999999999999999999999; }");
    println!("[OBSERVED] Z errors: {:?}", r.semantic_errors);
    // 期望：报"元组字段 ... 不是合法整数下标" 或 "越界"，但不应 panic
}

// ============================================================
// CANDIDATE BUG AA: gen_for 错误恢复路径（非 a..b）push_scope 后是否同步 borrow_scopes
//   验证 `for i in not_a_range { ... }` 后续作用域不被破坏
// ============================================================
#[test]
fn cand_aa_for_with_non_range_iterable_does_not_corrupt_state() {
    let r = run(r#"
        fn main(){
            let mut k:i32 = 1;
            for i in k { let _x:i32 = i; }
            let r = &k;
            let y:i32 = *r;
        }
    "#);
    println!("[OBSERVED] AA errors: {:?}", r.semantic_errors);
    // 应至少报 "for 迭代结构必须是范围 `a..b`"。
    let has = r.semantic_errors.iter().any(|e| e.message.contains("迭代结构必须是范围"));
    println!("[OBSERVED] AA has 范围错误: {}", has);
}

// ============================================================
// CANDIDATE BUG BB: 函数 main 返回 i32 但无 return 时，末尾兜底 RETURN _ _ _，缺乏控制流警告
// ============================================================
#[test]
fn cand_bb_non_unit_function_falls_through_without_warning() {
    let r = run("fn main()->i32 { let a:i32 = 1; }");
    println!("[OBSERVED] BB errors: {:?}", r.semantic_errors);
    for (i, q) in r.quadruples.iter().enumerate() {
        println!("  {:>3}: {} {} {} {}", i, q.op, q.arg1, q.arg2, q.result);
    }
    // 观察：是否有"函数声明返回类型 i32, 但末端无 return" 类似的报错。
}

// ============================================================
// CANDIDATE BUG CC: 递归调用（前向引用）应可正常签名匹配
// ============================================================
#[test]
fn cand_cc_recursive_function_call() {
    let r = run("fn fact(n:i32)->i32 { if n==0 { 1 } else { n * fact(n-1) } } fn main(){ let x:i32 = fact(3); }");
    println!("[OBSERVED] CC errors: {:?}", r.semantic_errors);
    assert!(r.semantic_errors.is_empty(), "递归调用不应报错: {:?}", r.semantic_errors);
}

// ============================================================
// CANDIDATE BUG DD: ElseBranch::ElseIf 链式（else if）类型与控制流
// ============================================================
#[test]
fn cand_dd_else_if_chain_types() {
    let r = run(r#"
        fn main(){
            let x:i32 = 1;
            let y:i32 = if x==1 { 10 } else if x==2 { 20 } else { 30 };
        }
    "#);
    println!("[OBSERVED] DD errors: {:?}", r.semantic_errors);
    assert!(r.semantic_errors.is_empty(), "else-if 链不应报错: {:?}", r.semantic_errors);
}

// ============================================================
// CANDIDATE BUG EE: 函数名在比较 `<` 等位置时是否专门提示
// ============================================================
#[test]
fn cand_ee_function_in_comparison() {
    let r = run("fn g()->i32 { 1 } fn main(){ let x:i32 = if g < 1 { 0 } else { 1 }; }");
    println!("[OBSERVED] EE errors: {:?}", r.semantic_errors);
}

// ============================================================
// CANDIDATE BUG FF: 数组元素类型不匹配后续 INDEX 类型推断
// ============================================================
#[test]
fn cand_ff_array_element_mismatch_then_index_type() {
    // 用 i32 数组的索引位置不再"误污染"声明类型不匹配
    let r = run(r#"
        fn main(){
            let mut a:[i32;3] = [1,2,3];
            let b:i32 = a[(1==1)];  // 索引非 i32
        }
    "#);
    println!("[OBSERVED] FF errors: {:?}", r.semantic_errors);
    let has = r.semantic_errors.iter().any(|e| e.message.contains("数组下标类型"));
    println!("[OBSERVED] FF reports index-type: {}", has);
}

// ============================================================
// CANDIDATE BUG GG: 函数返回 `[i32;3]`，调用结果是否能匹配数组类型形参
// ============================================================
#[test]
fn cand_gg_function_returning_array() {
    let r = run(r#"
        fn make()->[i32;3] { [1,2,3] }
        fn take(a:[i32;3]) {}
        fn main(){ take(make()); }
    "#);
    println!("[OBSERVED] GG errors: {:?}", r.semantic_errors);
    assert!(r.semantic_errors.is_empty(), "数组返回/传参不应报错: {:?}", r.semantic_errors);
}
