//! 第五轮新角度探针（不修源码，仅观察行为）
use easy_analyzer::analyze;
use easy_lexer::lex;
use easy_parser::parse_program_ast;

fn run(src: &str) -> (Vec<String>, Vec<String>) {
    let lex = lex(src);
    if !lex.errors.is_empty() {
        return (
            lex.errors.iter().map(|e| format!("lex: {:?}", e)).collect(),
            vec![],
        );
    }
    let prog = match parse_program_ast(&lex.tokens) {
        Ok(p) => p,
        Err(e) => return (vec![format!("parse: {:?}", e)], vec![]),
    };
    let r = analyze(&prog);
    let errs = r.semantic_errors.iter().map(|e| e.message.clone()).collect();
    let quads = r
        .quadruples
        .iter()
        .map(|q| format!("{} {} {} {}", q.op, q.arg1, q.arg2, q.result))
        .collect();
    (errs, quads)
}

fn show(name: &str, src: &str) {
    let (errs, quads) = run(src);
    eprintln!("\n==== {} ====", name);
    eprintln!("SRC: {}", src);
    eprintln!("ERRORS ({}):", errs.len());
    for e in &errs {
        eprintln!("  - {}", e);
    }
    eprintln!("QUADS ({}):", quads.len());
    for (i, q) in quads.iter().enumerate() {
        eprintln!("  {:>3}: {}", i, q);
    }
}

// ============================================================
// 角度一：跨函数标签/temp 编号唯一性（IR 全局命名）
// ============================================================

#[test]
fn p5_a_label_across_functions_unique() {
    // 多个函数都有 while → 都生成 L1, L2 ... 看是否复用
    show(
        "a_label_unique",
        "fn f(){ while 1==1 { let a:i32=1; } }
         fn g(){ while 1==1 { let a:i32=1; } }
         fn main(){}",
    );
}

// ============================================================
// 角度二：调用未声明函数时 args 中产生的副作用
// ============================================================

#[test]
fn p5_b_call_undeclared_with_side_effects() {
    // undeclared(f(), &mut a) — f() 仍被求值且 PARAM 仍发？
    show(
        "b_undeclared_side_effects",
        "fn f()->i32{1} fn main(){ let mut a:i32=0; undeclared(f(), &mut a); }",
    );
}

// ============================================================
// 角度三：&mut 临时值
// ============================================================

#[test]
fn p5_c_refmut_of_rvalue() {
    // `&mut 1` / `&mut f()` 在当前实现中如何处理？root_identifier 为 None
    show(
        "c_refmut_rvalue",
        "fn f()->i32{1} fn main(){ let p = &mut f(); }",
    );
}

#[test]
fn p5_c2_refmut_of_literal() {
    show("c2_refmut_literal", "fn main(){ let p = &mut 1; }");
}

// ============================================================
// 角度四：函数实参类型与 Type::Ref/&mut/Array 兼容性细节
// ============================================================

#[test]
fn p5_d_refmut_param_with_immut_arg() {
    // 形参 &mut i32, 实参 &i32 应报错
    show(
        "d_refmut_param_immut_arg",
        "fn f(p:&mut i32){} fn main(){ let a:i32=1; f(&a); }",
    );
}

// ============================================================
// 角度五：函数 main 强制要求
// ============================================================

#[test]
fn p5_e_no_main_function() {
    // PDF 是否要求 main 存在？当前似乎不检查
    show("e_no_main", "fn foo(){}");
}

#[test]
fn p5_e2_main_with_return_type() {
    // main()->i32 是否合法（与 PDF 期望）
    show("e2_main_with_return", "fn main()->i32 { return 0; }");
}

// ============================================================
// 角度六：作用域 / shadowing 边界
// ============================================================

#[test]
fn p5_f_shadow_param_in_function_body() {
    // 形参 a，函数体内 let a：合法 shadow
    show(
        "f_shadow_param",
        "fn f(a:i32){ let a:i32 = a + 1; let b:i32 = a; }
         fn main(){}",
    );
}

// ============================================================
// 角度七：for 循环范围中包含函数调用 (continue/break 在内层)
// ============================================================

#[test]
fn p5_g_break_inside_for_in_loop() {
    // 内层 for 中 break (无值) 应当只跳出 for，不影响外层 loop
    show(
        "g_break_for_in_loop",
        "fn main(){ let x = loop { for i in 0..3 { break; } break 1; }; }",
    );
}

// ============================================================
// 角度八：unwrap / panic 风险扫描
// ============================================================

#[test]
fn p5_h_tuple_index_overflow_usize() {
    // 元组下标 .99999... 超 usize::MAX，需 parse::<usize>() 失败处理
    show(
        "h_tuple_idx_overflow",
        "fn main(){ let t:(i32,i32)=(1,2); let x:i32 = t.99999999999999999999999999; }",
    );
}

#[test]
fn p5_h2_array_decl_zero_length_used() {
    // [i32;0] 可以声明 / 下标访问应静态越界
    show(
        "h2_zero_length_indexed",
        "fn main(){ let a:[i32;0] = []; let b:i32 = a[0]; }",
    );
}

#[test]
fn p5_h3_huge_array_length() {
    // 数组长度类型注解 usize::MAX 是否能解析成功并不 panic
    show(
        "h3_huge_array_len",
        "fn main(){ let a:[i32;99999999999999999999] = []; }",
    );
}

// ============================================================
// 角度九：相互递归
// ============================================================

#[test]
fn p5_i_mutual_recursion() {
    show(
        "i_mutual_recursion",
        "fn even(n:i32)->i32{ if n==0 { return 1; } return odd(n-1); }
         fn odd(n:i32)->i32{ if n==0 { return 0; } return even(n-1); }
         fn main()->i32{ return even(3); }",
    );
}

// ============================================================
// 角度十：函数声明顺序敏感性 + 函数名作为表达式的 IR 形态
// ============================================================

#[test]
fn p5_j_func_name_as_rvalue_in_binop() {
    // R-6 / cand_ee 已涵盖；此处看 IR 形态
    show("j_fn_in_binop", "fn f(){} fn main(){ let a:i32 = f + 1; }");
}

// ============================================================
// 角度十一：assign 到未声明变量 + 值表达式有副作用
// ============================================================

#[test]
fn p5_k_assign_undeclared_with_side_effect() {
    // undeclared = f();  — f() 仍发 CALL，但 = 不应发
    show(
        "k_assign_undeclared",
        "fn f()->i32{1} fn main(){ undeclared = f(); }",
    );
}

// ============================================================
// 角度十二：返回 Type::Function 类型时 IR
// ============================================================

#[test]
fn p5_l_return_function_name() {
    // fn f()->i32 { return g; }   g 是函数 — 类型不匹配
    show(
        "l_return_function_name",
        "fn g()->i32{1} fn f()->i32{ return g; } fn main(){}",
    );
}

// ============================================================
// 角度十三：嵌套元组/数组 IR temp 编号一致性 + 大参数函数
// ============================================================

#[test]
fn p5_m_many_args_call() {
    // 25 个实参，验证 PARAM 顺序
    show(
        "m_many_args",
        "fn f(a:i32,b:i32,c:i32,d:i32,e:i32){}
         fn main(){ f(1,2,3,4,5); }",
    );
}

// ============================================================
// 角度十四：condition 是 Call 返回 Unit 的情况
// ============================================================

#[test]
fn p5_n_if_cond_unit_call() {
    // if g() { ... } 其中 g()->Unit
    show("n_if_cond_unit_call", "fn g(){} fn main(){ if g() { } }");
}

// ============================================================
// 角度十五：return 表达式带类型为 Function
// ============================================================

#[test]
fn p5_o_break_value_function_name() {
    // loop { break g; } where g 是函数
    show(
        "o_break_fn_name",
        "fn g(){} fn main(){ let x = loop { break g; }; }",
    );
}

// ============================================================
// 角度十六：deref 链 *(*p) 不报错？
// ============================================================

#[test]
fn p5_p_double_deref() {
    // 二级 deref 当前是否支持？
    show(
        "p_double_deref",
        "fn main(){ let a:i32=1; let p:&i32 = &a; let q:&&i32 = &p; let x:i32 = **q; }",
    );
}

// ============================================================
// 角度十七：array assignment whole-array (let a = b 当 b 是数组)
// ============================================================

#[test]
fn p5_q_array_whole_assignment() {
    // let mut a:[i32;3]=[1,2,3]; let b:[i32;3]=a; b[0]=9;
    show(
        "q_array_copy",
        "fn main(){ let a:[i32;3]=[1,2,3]; let mut b:[i32;3]=a; b[0]=9; }",
    );
}

// ============================================================
// 角度十八：Continue 在 loop {} 表达式中
// ============================================================

#[test]
fn p5_r_continue_in_loop_expr() {
    show(
        "r_continue_in_loop_expr",
        "fn main(){ let x = loop { if 1==1 { continue; } break 1; }; }",
    );
}

// ============================================================
// 角度十九：let x:i32; let y:i32 = x; — 未初始化使用，仍发 = x _ y
// ============================================================

#[test]
fn p5_s_uninit_use_emits_assign() {
    show(
        "s_uninit_use_assign",
        "fn main(){ let x:i32; let y:i32 = x; }",
    );
}

// ============================================================
// 角度二十：Type::Range 渗到表达式上下文（不在 for 中）
// ============================================================

#[test]
fn p5_t_range_as_assign_target_type() {
    // 在 parser 不允许 let x = 0..3，但 if cond { 0..3 } else { 0..3 } 在 expression 位置 — 不会有 Range BinaryOp 被 parsed except inside for
    // Range 只能通过 for 出现；此点已在 D 中讨论。
    show("t_range_outside_for", "fn main(){ for i in 0..3 { let x:i32 = i; } }");
}
