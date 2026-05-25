//! 第五轮聚焦验证
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

#[test]
fn focus_a_refmut_of_literal_is_unchecked() {
    let (errs, _) = run("fn main(){ let p = &mut 1; }");
    eprintln!("ERRS: {:?}", errs);
    // 当前行为：0 errs，p 类型推断为 &mut i32。Rust 拒绝。
}

#[test]
fn focus_b_break_function_name_in_loop_unchecked() {
    let (errs, quads) = run("fn g(){} fn main(){ let x = loop { break g; }; }");
    eprintln!("ERRS: {:?}", errs);
    for q in &quads { eprintln!("  {}", q); }
    // 当前行为：0 errs；break_type=Function；loop 表达式类型推断为 Type::Function；
    // x 是 Type::Function 变量；后续 IR 包含 BREAK g _ L2 — 下游解释器会把 g 当数据。
}

#[test]
fn focus_c_break_function_then_int_in_loop() {
    // 第一个 break 是函数名，第二个 break 是 i32 — 应当报"类型不一致"
    let (errs, _) = run("fn g(){} fn main(){ let x = loop { if 1==1 { break g; } break 1; }; }");
    eprintln!("ERRS: {:?}", errs);
}

#[test]
fn focus_d_call_in_args_undeclared_callee_leaks_call() {
    let (errs, quads) = run("fn f()->i32{1} fn main(){ undef(f()); }");
    eprintln!("ERRS: {:?}", errs);
    for q in &quads { eprintln!("  {}", q); }
    // f() 是否仍产生 CALL（属于错误恢复期内的副作用 IR）
}

#[test]
fn focus_e_huge_array_length_in_typenode() {
    let (errs, _) = run("fn main(){ let a:[i32;99999999999999999999] = []; }");
    eprintln!("ERRS: {:?}", errs);
    // parser 应当 ParseError；不应 panic
}

#[test]
fn focus_f_zero_length_array_indexed() {
    // 用变量下标避免 parser 拒绝 ；parser 期望 number — 但 a[i] 中 i 是 Identifier
    let (errs, _) = run("fn main(){ let a:[i32;0] = []; let i:i32=0; let b:i32 = a[i]; }");
    eprintln!("ERRS: {:?}", errs);
}

#[test]
fn focus_g_main_required() {
    let (errs, _) = run("fn foo(){}");
    eprintln!("ERRS: {:?}", errs);
    // PDF 期望强制 main 存在？当前无任何提示
}

#[test]
fn focus_h_refmut_rvalue_in_let() {
    let (errs, quads) = run("fn f()->i32{1} fn main(){ let p:&mut i32 = &mut f(); }");
    eprintln!("ERRS: {:?}", errs);
    for q in &quads { eprintln!("  {}", q); }
}

#[test]
fn focus_i_param_unused_or_mutated_in_body_borrow() {
    // 形参声明 mut 后取 &mut，离开函数前借用是否清理？
    let (errs, _) = run("fn f(mut a:i32){ let p:&mut i32 = &mut a; }
                         fn main(){ let mut x:i32=1; let p:&mut i32 = &mut x; }");
    eprintln!("ERRS: {:?}", errs);
}

#[test]
fn focus_j_array_assign_via_let_emits_one_assign() {
    // 整数组拷贝是否仅 = a _ b 一条，但 a/b 仍指同一 memory 模型？
    let (_, quads) = run("fn main(){ let a:[i32;3]=[1,2,3]; let mut b:[i32;3]=a; b[0]=9; }");
    for q in &quads { eprintln!("  {}", q); }
}

#[test]
fn focus_k_if_cond_unit_call_emits_if_false() {
    // if g() {} where g()->Unit — 报 "条件类型 () 不可作为条件"，
    // 但仍发 IF_FALSE _ _ L? — 占位符作 cond
    let (errs, quads) = run("fn g(){} fn main(){ if g() { } }");
    eprintln!("ERRS: {:?}", errs);
    for q in &quads { eprintln!("  {}", q); }
}

#[test]
fn focus_l_uninit_used_emits_assign_with_uninit_name() {
    // let x:i32; let y:i32=x; — 当前发 = x _ y 但 x 未初始化，下游解释器读 x 会 panic
    let (errs, quads) = run("fn main(){ let x:i32; let y:i32 = x; }");
    eprintln!("ERRS: {:?}", errs);
    for q in &quads { eprintln!("  {}", q); }
}

#[test]
fn focus_m_function_name_in_array_literal() {
    // let a:[i32;1] = [g]; — g 是函数名 — 数组元素类型推断为 Type::Function
    let (errs, _) = run("fn g(){} fn main(){ let a:[i32;1] = [g]; }");
    eprintln!("ERRS: {:?}", errs);
}

#[test]
fn focus_n_function_name_in_tuple_then_indexed() {
    let (errs, _) = run("fn g(){} fn main(){ let t:(i32,i32) = (g, 1); }");
    eprintln!("ERRS: {:?}", errs);
}

#[test]
fn focus_o_param_mut_marker_in_paramdecl_lost() {
    // 形参声明为 mut，PARAM_DECL 不携带 mut 信息（下游解释器无法知道）
    let (_, quads) = run("fn f(mut a:i32, b:i32){} fn main(){}");
    for q in &quads { eprintln!("  {}", q); }
}
