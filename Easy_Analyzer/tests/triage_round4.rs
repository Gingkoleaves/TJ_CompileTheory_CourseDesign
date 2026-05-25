//! 第四轮 BUG 探测。仅做实证 inspect，不 assert，便于看到实际行为。

use easy_analyzer::analyze;
use easy_lexer::lex;
use easy_parser::parse_program_ast;

fn analyze_src(src: &str) -> (Vec<String>, Vec<String>) {
    let lex = lex(src);
    if !lex.errors.is_empty() {
        return (
            lex.errors.iter().map(|e| format!("{:?}", e)).collect(),
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
    let (errs, quads) = analyze_src(src);
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

#[test]
fn r4_cand_a_index_side_effect_double_eval() {
    // 写 a[f()] = 1，f() 应只被求值一次；但 write_back_to_parent 是否在 idx 链上重复求值？
    // 此处 base=a 是 Identifier，不进入 write_back，单层下标无副作用。
    // 但 a[f()][0] = 1 时，base=a[f()] 走 Index 分支，write_back 再次求值 index=f()。
    show(
        "r4_a_nested_index_assign_double_call",
        "fn f()->i32 { 0 } fn main(){ let mut a:[[i32;1];1] = [[0]]; a[f()][0] = 9; }",
    );
}

#[test]
fn r4_cand_b_nested_field_assign_doubled() {
    show(
        "r4_b_nested_field_assign_a0_0",
        "fn main(){ let mut a:((i32,),) = ((1,),); a.0.0 = 9; }",
    );
}

#[test]
fn r4_cand_c_for_array_loop_var_assign_no_writeback() {
    // 数组 for：循环体内对 binding 赋值，IR 是否回写到原数组？
    show(
        "r4_c_for_array_mutate_binding",
        "fn main(){ let mut a:[i32;3] = [1,2,3]; for mut i in a { i = i + 10; } }",
    );
}

#[test]
fn r4_cand_d_loop_no_break_unit_default() {
    // loop 无 break，break_type=None，默认 Unit。`let x:i32 = loop {};` 应报错。
    show("r4_d_loop_no_break_assign_i32", "fn main(){ let x:i32 = loop {}; }");
}

#[test]
fn r4_cand_e_array_length_mismatch_double_error() {
    show(
        "r4_e_array_len_mismatch",
        "fn main(){ let a:[i32;3] = [1,2,3,4,5]; }",
    );
}

#[test]
fn r4_cand_f_for_array_zero_length() {
    show(
        "r4_f_for_empty_array",
        "fn main(){ let a:[i32;0] = []; for i in a { let x:i32 = i; } }",
    );
}

#[test]
fn r4_cand_g_borrow_function_param_alias() {
    // 形参在函数体内被 &mut 借用两次：注意 register_borrow 在跨作用域时按 sum 计
    show(
        "r4_g_param_double_mut_borrow",
        "fn f(mut a:i32){ let p:&mut i32 = &mut a; let q:&mut i32 = &mut a; } fn main(){}",
    );
}

#[test]
fn r4_cand_h_recursion_self_call() {
    show(
        "r4_h_fact",
        "fn fact(n:i32)->i32{ if n<2 { return 1; } return n*fact(n-1); } fn main()->i32{ return fact(5); }",
    );
}

#[test]
fn r4_cand_i_main_with_params() {
    // PDF 没强制 main 签名。但若用户写 fn main(x:i32){} 当前接受吗？
    show("r4_i_main_with_param", "fn main(x:i32){}");
}

#[test]
fn r4_cand_j_negative_array_length_via_typenode() {
    // TypeNode::Array.length 是 usize，负长度在 parser 层就拒了。仅做长度=0 与极大数测试。
    show(
        "r4_j_large_array_length",
        "fn main(){ let a:[i32;1000000] = [0; 1000000]; }",
    );
}

#[test]
fn r4_cand_k_param_shadow_same_name_as_func() {
    // 形参与已有函数同名 — gen_let 在 R3-5 加了警告，但形参不走 gen_let
    show(
        "r4_k_param_shadow_fn",
        "fn g()->i32 { 1 } fn f(g:i32){} fn main(){}",
    );
}

#[test]
fn r4_cand_l_continue_in_loop_expr() {
    // continue 跳到 loop_labels.start。loop {} 的 start=label_start（loop body 起点），OK。
    show(
        "r4_l_continue_in_loop",
        "fn main(){ let x = loop { if 1==1 { continue; } break 1; }; }",
    );
}

#[test]
fn r4_cand_m_array_init_repeat_syntax() {
    show(
        "r4_m_array_repeat",
        "fn main(){ let a:[i32;3] = [0;3]; }",
    );
}

#[test]
fn r4_cand_n_deref_assign_to_immutable_ref_inner() {
    // *p = v 其中 p 是 &i32（不可变引用）— gen_assign Unary{Deref} 分支已检测
    show(
        "r4_n_deref_assign_immut",
        "fn main(){ let mut a:i32 = 1; let p:&i32 = &a; *p = 2; }",
    );
}

#[test]
fn r4_cand_o_if_branch_function_call_unit() {
    show(
        "r4_o_if_branch_call_unit",
        "fn g(){} fn main(){ let x:i32 = if 1==1 { g(); 1 } else { 2 }; }",
    );
}

#[test]
fn r4_cand_p_array_in_function_return() {
    show(
        "r4_p_fn_return_array",
        "fn f()->[i32;3] { return [1,2,3]; } fn main()->i32 { let a:[i32;3] = f(); return a[0]; }",
    );
}

#[test]
fn r4_cand_q_borrow_count_when_function_call_returns_ref() {
    // 当前 register_borrow 只在 root_identifier 可识别时累计；&f() 不计
    show(
        "r4_q_borrow_call_return",
        "fn main(){ let a:i32 = 1; let p = &a; let q = &a; }",
    );
}

#[test]
fn r4_cand_r_if_no_else_then_branch_type_only_checked_when_tail_present() {
    // 无 else 的 if，若 then 有 tail i32 — 报错
    show(
        "r4_r_if_no_else_tail_i32",
        "fn main(){ let x:i32 = if 1==1 { 1 }; }",
    );
}

#[test]
fn r4_cand_s_static_oob_bounds_eq() {
    show(
        "r4_s_index_eq_len",
        "fn main(){ let a:[i32;3] = [1,2,3]; let b:i32 = a[3]; }",
    );
}

#[test]
fn r4_cand_t_nested_break_value_to_outer_loop_via_for() {
    // 第三轮 R3-1 已修。再验证 loop -> for -> if -> break-value 的链
    show(
        "r4_t_break_value_in_inner_loop_via_for",
        "fn main(){ let x = loop { for i in 0..1 { if 1==1 { break; } } break 5; }; }",
    );
}

#[test]
fn r4_cand_u_function_param_immutable_reassign() {
    show(
        "r4_u_param_immut_reassign",
        "fn f(a:i32){ a = a + 1; } fn main(){}",
    );
}

#[test]
fn r4_cand_v_unit_func_tail_value() {
    // fn main(){ 1 } — 函数声明返回 Unit（默认），但末尾表达式是 i32
    show("r4_v_unit_func_tail_i32", "fn main(){ 1 }");
}

#[test]
fn r4_cand_w_function_return_tuple_field_access() {
    show(
        "r4_w_return_tuple_field",
        "fn f()->(i32,i32){ return (1,2); } fn main()->i32 { return f().0; }",
    );
}
