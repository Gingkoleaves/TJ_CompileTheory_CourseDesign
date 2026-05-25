//! 第六轮第三批：聚焦剩余角落

use easy_lexer::lex;
use easy_parser::parse_program_ast;

fn analyze(src: &str) -> easy_analyzer::AnalysisResult {
    let r = lex(src);
    assert!(r.errors.is_empty(), "lex: {:?}", r.errors);
    let p = parse_program_ast(&r.tokens).expect("parse");
    easy_analyzer::analyze(&p)
}

// ── R-1: 形参与函数同名 ──
#[test]
fn r6_r1_param_named_same_as_self_function() {
    let src = "fn f(f:i32){} fn main(){}";
    let out = analyze(src);
    let msgs: Vec<_> = out.semantic_errors.iter().map(|e| e.message.clone()).collect();
    eprintln!("param=fn name: {:?}", msgs);
}

// ── R-2: 形参遮蔽函数表 ──
#[test]
fn r6_r2_param_shadows_function() {
    let src = "fn f()->i32{1} fn main() { let f:i32 = 2; let x:i32 = f(); }";
    let out = analyze(src);
    let msgs: Vec<_> = out.semantic_errors.iter().map(|e| e.message.clone()).collect();
    eprintln!("param shadows fn: {:?}", msgs);
}

// ── R-3: 形参与 let 同名（R3-5 检查面）──
#[test]
fn r6_r3_param_let_collision() {
    let src = "fn f(a:i32) { let a:i32 = 5; }";
    let out = analyze(src);
    let msgs: Vec<_> = out.semantic_errors.iter().map(|e| e.message.clone()).collect();
    eprintln!("param/let collision: {:?}", msgs);
}

// ── R-4: 字符串字面量根本不存在；但 = 后空 ──
#[test]
fn r6_r4_assign_to_self() {
    let src = "fn main(){ let mut x:i32=0; x = x; }";
    let out = analyze(src);
    let msgs: Vec<_> = out.semantic_errors.iter().map(|e| e.message.clone()).collect();
    eprintln!("x=x: {:?}", msgs);
}

// ── R-5: 在 break 表达式里写赋值（被拒就 OK）──
#[test]
fn r6_r5_assign_inside_expression() {
    let src = "fn main(){ let mut a:i32=0; let b:i32 = (a=1); }";
    let r = lex(src);
    let res = parse_program_ast(&r.tokens);
    eprintln!("assign-in-expr: {:?}", res.err());
}

// ── R-6: 比较表达式连续，无括号 (a < b < c) ──
#[test]
fn r6_r6_chained_comparison() {
    let src = "fn main(){ let a:i32=1; let b:i32=2; let c:i32=3; let x = a<b<c; }";
    let r = lex(src);
    let res = parse_program_ast(&r.tokens);
    eprintln!("a<b<c parse: {:?}", res.is_ok());
    // Rust 拒绝 chained comparison；本课程？parser 顶层一直循环匹配比较运算符
    if let Ok(p) = res {
        let out = easy_analyzer::analyze(&p);
        eprintln!("semantic: {:?}", out.semantic_errors.iter().map(|e| &e.message).collect::<Vec<_>>());
        eprintln!("IR:");
        for q in &out.quadruples {
            eprintln!("  {} {} {} {}", q.op, q.arg1, q.arg2, q.result);
        }
    }
}

// ── R-7: 字段索引上限：t.4294967296（接近 usize overflow）──
#[test]
fn r6_r7_field_overflow() {
    let src = "fn main(){ let t:(i32,i32)=(1,2); let a:i32 = t.4294967296; }";
    let out = analyze(src);
    let msgs: Vec<_> = out.semantic_errors.iter().map(|e| e.message.clone()).collect();
    eprintln!("field overflow: {:?}", msgs);
    // 期望：报错（不会 panic）
}

// ── R-8: 数组初始化元素个数与类型长度不一致 ──
#[test]
fn r6_r8_array_length_mismatch_init() {
    let src = "fn main(){ let a:[i32;3] = [1,2]; }";
    let out = analyze(src);
    let msgs: Vec<_> = out.semantic_errors.iter().map(|e| e.message.clone()).collect();
    eprintln!("array length mismatch: {:?}", msgs);
}

// ── R-9: 引用类型与值类型混淆 ──
#[test]
fn r6_r9_ref_value_mismatch() {
    let src = r#"
        fn main(){
            let x:i32 = 1;
            let p:&i32 = &x;
            let y:i32 = p + 1;
        }
    "#;
    let out = analyze(src);
    let msgs: Vec<_> = out.semantic_errors.iter().map(|e| e.message.clone()).collect();
    eprintln!("&i32 + i32: {:?}", msgs);
}

// ── R-10: deref 然后用 ──
#[test]
fn r6_r10_deref_use() {
    let src = r#"
        fn main(){
            let x:i32 = 5;
            let p:&i32 = &x;
            let y:i32 = *p + 1;
        }
    "#;
    let out = analyze(src);
    let msgs: Vec<_> = out.semantic_errors.iter().map(|e| e.message.clone()).collect();
    eprintln!("*p + 1: {:?}", msgs);
}

// ── R-11: 函数返回 &i32，调用方接收 ──
#[test]
fn r6_r11_function_returns_ref() {
    let src = r#"
        fn f() -> &i32 { let x:i32 = 1; return &x; }
        fn main(){}
    "#;
    let r = lex(src);
    let res = parse_program_ast(&r.tokens);
    match res {
        Ok(p) => {
            let out = easy_analyzer::analyze(&p);
            eprintln!("fn->&i32: {:?}", out.semantic_errors.iter().map(|e| &e.message).collect::<Vec<_>>());
        }
        Err(e) => eprintln!("parse: {}", e),
    }
}

// ── R-12: PARAM 类型与形参类型不一致：调用 g(true) where g(a:i32) ──
#[test]
fn r6_r12_arg_type_mismatch() {
    let src = r#"
        fn g(a:i32){}
        fn main(){ g(1>0); }
    "#;
    let out = analyze(src);
    let msgs: Vec<_> = out.semantic_errors.iter().map(|e| e.message.clone()).collect();
    eprintln!("arg type mismatch bool->i32: {:?}", msgs);
}

// ── R-13: 在 array 字面量里调用函数 ──
#[test]
fn r6_r13_array_with_call_elements() {
    let src = r#"
        fn g() -> i32 { 1 }
        fn main() { let a:[i32;2] = [g(), g()]; }
    "#;
    let out = analyze(src);
    eprintln!("array with calls: {:?}", out.semantic_errors.iter().map(|e| &e.message).collect::<Vec<_>>());
    eprintln!("IR:");
    for q in &out.quadruples {
        eprintln!("  {} {} {} {}", q.op, q.arg1, q.arg2, q.result);
    }
}

// ── R-14: 类型 [i32;0]，空数组 ──
#[test]
fn r6_r14_zero_array_with_index_var() {
    let src = "fn main(){ let i:i32=0; let a:[i32;0] = []; let b:i32 = a[i]; }";
    let out = analyze(src);
    let msgs: Vec<_> = out.semantic_errors.iter().map(|e| e.message.clone()).collect();
    eprintln!("zero array dyn idx: {:?}", msgs);
}

// ── R-15: 在 let x:[i32; 0]; 之后写 a[i] = 5; ──
#[test]
fn r6_r15_zero_array_assign() {
    let src = "fn main(){ let i:i32=0; let mut a:[i32;0]=[]; a[i]=1; }";
    let out = analyze(src);
    let msgs: Vec<_> = out.semantic_errors.iter().map(|e| e.message.clone()).collect();
    eprintln!("zero array dyn write: {:?}", msgs);
}

// ── R-16: 形参 mut + 实参不可变 ──
#[test]
fn r6_r16_mut_param_immut_arg() {
    let src = r#"
        fn g(mut a:i32){}
        fn main(){ let x:i32=1; g(x); }
    "#;
    let out = analyze(src);
    let msgs: Vec<_> = out.semantic_errors.iter().map(|e| e.message.clone()).collect();
    eprintln!("mut param immut arg: {:?}", msgs);
}

// ── R-17: 循环中下标越界（用循环变量）──
#[test]
fn r6_r17_loop_var_oob() {
    let src = r#"
        fn main(){
            let a:[i32;3]=[1,2,3];
            for i in 0..10 {
                let b:i32 = a[i];
            }
        }
    "#;
    let out = analyze(src);
    let msgs: Vec<_> = out.semantic_errors.iter().map(|e| e.message.clone()).collect();
    eprintln!("loop var OOB: {:?}", msgs);
    // 静态分析不识别 i 的范围，期望放行
}

// ── R-18: 函数没有 return 但末尾是 if/else 都 return ──
#[test]
fn r6_r18_function_returns_via_both_branches() {
    let src = r#"
        fn f(c:i32) -> i32 {
            if c > 0 { return 1; } else { return 2; }
        }
        fn main(){}
    "#;
    let out = analyze(src);
    let msgs: Vec<_> = out.semantic_errors.iter().map(|e| e.message.clone()).collect();
    eprintln!("fn returns via both branches: {:?}", msgs);
    // R3-3 仅看最后一条 stmt 是否 Return；这里最后是 if/else，不是 Return
    // 期望是漏报，或正确分析？
}
