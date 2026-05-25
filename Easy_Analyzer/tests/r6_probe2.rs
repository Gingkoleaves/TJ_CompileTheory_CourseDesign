//! 第六轮第二批探针：聚焦 semantic / 跨 crate 边界

use easy_lexer::lex;
use easy_parser::parse_program_ast;

fn analyze(src: &str) -> easy_analyzer::AnalysisResult {
    let r = lex(src);
    assert!(r.errors.is_empty(), "lex errs: {:?}", r.errors);
    let p = parse_program_ast(&r.tokens).expect("parse");
    easy_analyzer::analyze(&p)
}

fn parse_err(src: &str) -> Option<String> {
    let r = lex(src);
    if !r.errors.is_empty() { return Some(format!("lex: {:?}", r.errors)); }
    match parse_program_ast(&r.tokens) {
        Ok(_) => None,
        Err(e) => Some(format!("parse: {}", e)),
    }
}

// ── Q-1: 顶层不允许 fn 关键字以外的内容，应报错；尝试触发 ──
#[test]
fn r6_q1_garbage_after_function() {
    let e = parse_err("fn main(){} let x=1;");
    eprintln!("garbage after fn: {:?}", e);
    // 期望 parse error
}

// ── Q-2: 嵌套字段读取 ──
#[test]
fn r6_q2_nested_field_read() {
    let src = r#"
        fn main(){
            let t:((i32,i32),i32) = ((1,2), 3);
            let a:i32 = t.0.0;
            let b:i32 = t.0.1;
            let c:i32 = t.1;
        }
    "#;
    let out = analyze(src);
    let msgs: Vec<_> = out.semantic_errors.iter().map(|e| e.message.clone()).collect();
    eprintln!("nested field read errs: {:?}", msgs);
    assert!(msgs.is_empty(), "nested field read should be OK");
}

// ── Q-3: 嵌套字段写入：t.0.0 = 5 ──
#[test]
fn r6_q3_nested_field_write() {
    let src = r#"
        fn main(){
            let mut t:((i32,i32),i32) = ((1,2), 3);
            t.0.0 = 5;
        }
    "#;
    let out = analyze(src);
    let msgs: Vec<_> = out.semantic_errors.iter().map(|e| e.message.clone()).collect();
    eprintln!("nested field write errs: {:?}", msgs);
    eprintln!("IR:");
    for (i, q) in out.quadruples.iter().enumerate() {
        eprintln!("  {}: {} {} {} {}", i, q.op, q.arg1, q.arg2, q.result);
    }
    // 关注：是否正确递归 write-back 到 t（外层元组）
}

// ── Q-4: t.0[1] = 5 混合字段+下标 ──
#[test]
fn r6_q4_field_then_index_write() {
    let src = r#"
        fn main(){
            let mut t:([i32;2], i32) = ([1,2], 3);
            t.0[1] = 99;
        }
    "#;
    let out = analyze(src);
    let msgs: Vec<_> = out.semantic_errors.iter().map(|e| e.message.clone()).collect();
    eprintln!("field-then-index errs: {:?}", msgs);
    eprintln!("IR:");
    for (i, q) in out.quadruples.iter().enumerate() {
        eprintln!("  {}: {} {} {} {}", i, q.op, q.arg1, q.arg2, q.result);
    }
}

// ── Q-5: a[0].1 = 5 下标+字段写入 ──
#[test]
fn r6_q5_index_then_field_write() {
    let src = r#"
        fn main(){
            let mut a:[(i32,i32);2] = [(1,2),(3,4)];
            a[0].1 = 99;
        }
    "#;
    let out = analyze(src);
    let msgs: Vec<_> = out.semantic_errors.iter().map(|e| e.message.clone()).collect();
    eprintln!("index-then-field errs: {:?}", msgs);
    eprintln!("IR:");
    for (i, q) in out.quadruples.iter().enumerate() {
        eprintln!("  {}: {} {} {} {}", i, q.op, q.arg1, q.arg2, q.result);
    }
}

// ── Q-6: 元组内含数组类型 ──
#[test]
fn r6_q6_tuple_with_array_field() {
    let src = r#"
        fn main(){
            let t:([i32;3], i32) = ([1,2,3], 4);
            let a:i32 = t.0[2];
        }
    "#;
    let out = analyze(src);
    eprintln!("tuple-with-array errs: {:?}", out.semantic_errors.iter().map(|e| &e.message).collect::<Vec<_>>());
}

// ── Q-7: 同名函数和参数 ──
#[test]
fn r6_q7_function_and_param_same_name() {
    let src = "fn main(main:i32){} fn other(){}";
    let out = analyze(src);
    let msgs: Vec<_> = out.semantic_errors.iter().map(|e| e.message.clone()).collect();
    eprintln!("fn-param same name errs: {:?}", msgs);
}

// ── Q-8: 函数体的尾表达式是个 break；应被拒？──
#[test]
fn r6_q8_break_as_tail() {
    let src = "fn main(){ loop { break 1 } }";
    let r = lex(src);
    let res = parse_program_ast(&r.tokens);
    eprintln!("break as tail: {:?}", res.is_ok());
}

// ── Q-9: range 用 i32 字面量但下界 > 上界 ──
#[test]
fn r6_q9_reverse_range() {
    let src = "fn main(){ for i in 5..3 {} }";
    let out = analyze(src);
    let msgs: Vec<_> = out.semantic_errors.iter().map(|e| e.message.clone()).collect();
    eprintln!("reverse range errs: {:?}", msgs);
    // 期望：解析成功，语义可能不检查，运行时空循环
}

// ── Q-10: 在 if 表达式条件位置用赋值表达式：不允许 ──
#[test]
fn r6_q10_assign_in_condition() {
    let src = "fn main(){ let mut x:i32=0; if x=1 {} }";
    let res = parse_err(src);
    eprintln!("assign in cond: {:?}", res);
    // 期望：parse error（赋值是语句而非表达式）
}

// ── Q-11: PARAM_DECL/PARAM 顺序 ──
#[test]
fn r6_q11_param_order() {
    let src = r#"
        fn f(a:i32, b:i32, c:i32) -> i32 { return a+b+c; }
        fn main() -> i32 { return f(1, 2, 3); }
    "#;
    let out = analyze(src);
    eprintln!("call IR:");
    for q in &out.quadruples {
        eprintln!("  {} {} {} {}", q.op, q.arg1, q.arg2, q.result);
    }
}

// ── Q-12: 重复 fn 但参数不同（重载）──
#[test]
fn r6_q12_overload_attempt() {
    let src = "fn f(a:i32){} fn f(a:i32, b:i32){} fn main(){}";
    let out = analyze(src);
    let msgs: Vec<_> = out.semantic_errors.iter().map(|e| e.message.clone()).collect();
    eprintln!("overload: {:?}", msgs);
}

// ── Q-13: 函数体单纯一个 `;` ──
#[test]
fn r6_q13_function_with_only_semicolon() {
    let src = "fn main() { ; ; ; }";
    let out = analyze(src);
    eprintln!("only ;: {:?}", out.semantic_errors.iter().map(|e| &e.message).collect::<Vec<_>>());
}

// ── Q-14: let x:i32; 之后未赋值就 return x —— 不可变 + 未初始化 ──
#[test]
fn r6_q14_uninit_immutable_returned() {
    let src = "fn main()->i32 { let x:i32; return x; }";
    let out = analyze(src);
    let msgs: Vec<_> = out.semantic_errors.iter().map(|e| e.message.clone()).collect();
    eprintln!("uninit immut return: {:?}", msgs);
}

// ── Q-15: shadowing 后类型变化是否影响 IR ──
#[test]
fn r6_q15_shadow_with_type_change() {
    let src = r#"
        fn main(){
            let x:i32 = 1;
            let x:[i32;2] = [1,2];
            let y:i32 = x[0];
        }
    "#;
    let out = analyze(src);
    eprintln!("shadow type change: {:?}", out.semantic_errors.iter().map(|e| &e.message).collect::<Vec<_>>());
}

// ── Q-16: 元组下标静态越界 ──
#[test]
fn r6_q16_tuple_static_oob() {
    let src = "fn main(){ let t:(i32,i32) = (1,2); let a:i32 = t.5; }";
    let out = analyze(src);
    let msgs: Vec<_> = out.semantic_errors.iter().map(|e| e.message.clone()).collect();
    eprintln!("tuple OOB: {:?}", msgs);
}

// ── Q-17: 数组元素是数组（二维数组）──
#[test]
fn r6_q17_nested_array_type() {
    let src = "fn main(){ let m:[[i32;2];2] = [[1,2],[3,4]]; let a:i32 = m[0][1]; }";
    let out = analyze(src);
    eprintln!("2D array: {:?}", out.semantic_errors.iter().map(|e| &e.message).collect::<Vec<_>>());
}

// ── Q-18: 在 for 的 range 端点使用 `&x` 引用 ──
#[test]
fn r6_q18_for_range_with_ref() {
    let src = r#"
        fn main(){
            let n:i32=5;
            for i in 0..&n {}
        }
    "#;
    let out = analyze(src);
    let msgs: Vec<_> = out.semantic_errors.iter().map(|e| e.message.clone()).collect();
    eprintln!("for range with &: {:?}", msgs);
}

// ── Q-19: 调用未知名字传入复杂表达式 ──
#[test]
fn r6_q19_undeclared_call_complex_args() {
    let src = "fn main(){ undef(1+2*3, [1,2], (4,5)); }";
    let out = analyze(src);
    let msgs: Vec<_> = out.semantic_errors.iter().map(|e| e.message.clone()).collect();
    eprintln!("undef call complex args: {:?}", msgs);
    // 期望：未声明函数 + 不该多报子表达式错
}

// ── Q-20: 一个空函数 + 重复定义 ──
#[test]
fn r6_q20_redefinition_empty() {
    let src = "fn f(){} fn f(){} fn main(){}";
    let out = analyze(src);
    let msgs: Vec<_> = out.semantic_errors.iter().map(|e| e.message.clone()).collect();
    eprintln!("redef: {:?}", msgs);
    // FUNC 数量应只有 1 个 f（已修 BUG #7）
    let f_count = out.quadruples.iter().filter(|q| q.op == "FUNC" && q.arg1 == "f").count();
    eprintln!("f FUNC count: {}", f_count);
}

// ── Q-21: arg in call is tuple constructor; 类型应是 tuple ──
#[test]
fn r6_q21_tuple_as_arg() {
    let src = "fn g(t:(i32,i32)){} fn main(){ g((1,2)); }";
    let out = analyze(src);
    let msgs: Vec<_> = out.semantic_errors.iter().map(|e| e.message.clone()).collect();
    eprintln!("tuple as arg: {:?}", msgs);
}

// ── Q-22: 函数返回 -> () 但有 return 1 ──
#[test]
fn r6_q22_unit_return_with_value() {
    let src = "fn main() -> () { return 1; }";
    let out = analyze(src);
    let msgs: Vec<_> = out.semantic_errors.iter().map(|e| e.message.clone()).collect();
    eprintln!("unit return with value: {:?}", msgs);
}
