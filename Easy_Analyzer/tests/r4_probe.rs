//! 第四轮额外探针
use easy_analyzer::analyze;
use easy_lexer::lex;
use easy_parser::parse_program_ast;

fn analyze_src(src: &str) -> (Vec<String>, Vec<String>) {
    let lex = lex(src);
    if !lex.errors.is_empty() {
        return (lex.errors.iter().map(|e| format!("{:?}", e)).collect(), vec![]);
    }
    let prog = match parse_program_ast(&lex.tokens) {
        Ok(p) => p,
        Err(e) => return (vec![format!("parse: {:?}", e)], vec![]),
    };
    let r = analyze(&prog);
    let errs = r.semantic_errors.iter().map(|e| e.message.clone()).collect();
    let quads = r.quadruples.iter().map(|q| format!("{} {} {} {}", q.op, q.arg1, q.arg2, q.result)).collect();
    (errs, quads)
}

fn show(name: &str, src: &str) {
    let (errs, quads) = analyze_src(src);
    eprintln!("\n==== {} ====", name);
    eprintln!("SRC: {}", src);
    eprintln!("ERRORS ({}):", errs.len());
    for e in &errs { eprintln!("  - {}", e); }
    eprintln!("QUADS ({}):", quads.len());
    for (i, q) in quads.iter().enumerate() { eprintln!("  {:>3}: {}", i, q); }
}

#[test] fn p1_triple_nested_index_side_effect() {
    // a[f()][g()][0] = 9 — base 链多次重求值；f() 应被调用 ?? 次
    show("p1", "fn f()->i32{0} fn g()->i32{0}
fn main(){ let mut a:[[[i32;1];1];1] = [[[0]]]; a[f()][g()][0] = 9; }");
}

#[test] fn p2_for_array_writes_modify_binding_not_array() {
    // 上轮 r4_c 已观察到：循环变量赋值 IR 写 i 本地 temp，原数组 a 无回写
    // 这是符合 Rust 语义的（for i in a，i 是值拷贝），不应报错。本测试只观察。
    show("p2", "fn main(){ let mut a:[i32;3] = [1,2,3]; for mut i in a { i = i + 10; } }");
}

#[test] fn p3_assign_via_immut_index_chain() {
    // p:&[i32;3]（不可变引用）；(*p)[0] = 9 应报错
    show("p3", "fn main(){ let a:[i32;3]=[1,2,3]; let p:&[i32;3] = &a; (*p)[0] = 9; }");
}

#[test] fn p4_uninit_var_used_in_index_expr() {
    // let x; let a = arr[x]; — x 未初始化使用
    show("p4", "fn main(){ let arr:[i32;3]=[1,2,3]; let x:i32; let a:i32 = arr[x]; }");
}

#[test] fn p5_unknown_type_used_as_index_root() {
    // let a; a[0] = 1; — a 类型 Unknown，是否合理处理
    show("p5", "fn main(){ let mut a; a[0] = 1; }");
}

#[test] fn p6_break_in_loop_in_func_no_loop_outside() {
    // break/continue 在函数最外层（无循环）应报错
    show("p6", "fn main(){ break; }");
}

#[test] fn p7_borrow_pop_in_loop_body() {
    // for 循环体内的借用是否在每轮迭代后正确弹栈
    show("p7", "fn main(){ let mut a:i32=0; let mut b:i32=0; for i in 0..3 { let p:&mut i32 = &mut a; *p = i; } let q:&mut i32 = &mut a; }");
}

#[test] fn p8_main_with_return_type_only() {
    // fn main() -> i32 — PDF 一般要求 main 无返回值，但当前是否接受？
    show("p8", "fn main()->i32 { return 0; }");
}

#[test] fn p9_call_in_array_init() {
    // let a:[i32;1] = [f()]; f 副作用次数？
    show("p9", "fn f()->i32{0} fn main(){ let a:[i32;1] = [f()]; }");
}

#[test] fn p10_field_idx_too_big_panic() {
    // 元组下标超大字面量 — usize::parse 不会 panic？
    show("p10", "fn main(){ let t:(i32,i32) = (1,2); let x:i32 = t.99999999999999999999; }");
}

#[test] fn p11_assign_index_on_non_array() {
    // 对非数组类型做下标赋值
    show("p11", "fn main(){ let mut a:i32=1; a[0]=9; }");
}

#[test] fn p12_for_array_with_type_annotation_mismatch() {
    // for i:i32 in [(1,2)]
    show("p12", "fn main(){ let a:[(i32,i32);1]=[(1,2)]; for i:i32 in a { } }");
}

#[test] fn p13_double_decl_var_in_same_scope() {
    // shadowing 是否允许；下方 b 没初始化但被 shadow 后用，看类型推断
    show("p13", "fn main(){ let a:i32 = 1; let a:i32 = 2; let a:i32 = a + 1; }");
}

#[test] fn p14_param_skip_still_uses_index() {
    // f(a:i32, a:i32) - 跳过第二个 a。但 sig.params 仍存两份；arg index 是否错位？
    show("p14", "fn f(a:i32, a:i32){} fn main(){ f(1,2); }");
}

#[test] fn p15_uninit_in_assignment_value() {
    // 第一次赋值的 RHS 用了未初始化变量
    show("p15", "fn main(){ let x:i32; let y:i32 = x + 1; }");
}

#[test] fn p16_break_value_type_error_silent() {
    // loop {} 内 break <expr> 类型不一致，仍发射后续 BREAK，但 result 是否污染
    show("p16", "fn main(){ let x = loop { if 1==1 { break 1; } break (); }; }");
}

#[test] fn p17_param_name_shadows_function() {
    // 函数 g 存在；另一函数 f 形参也叫 g — R3-5 只在 gen_let 检查，形参不检查
    show("p17", "fn g()->i32{1} fn f(g:i32){ let y:i32 = g; } fn main(){ f(5); }");
}

#[test] fn p18_index_eval_in_lhs_chain_first_arg_index_double() {
    // a[0][f()] = 9：base = a[0]（无副作用），index = f()。
    // gen_assign 走 Index 分支：求 base 一次 + idx 一次。再 write_back 重求 base 一次 + idx 一次（write_back_to_parent 中 index）。
    // 因 base 是 a[0]，index 0 没副作用，但模式上 f() 会被求值两次？
    show("p18", "fn f()->i32{0} fn main(){ let mut a:[[i32;2];1]=[[0,0]]; a[0][f()] = 9; }");
}
