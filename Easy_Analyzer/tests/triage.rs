//! Round-2 BUG 复审：对潜在新 BUG 用最小程序实证。
//! 不论通过/失败都保留，作为后续修复或回归基线。

use std::collections::HashMap;

use easy_analyzer::analyze;
use easy_analyzer::ir::Quadruple;
use easy_lexer::lex;
use easy_parser::parse_program_ast;

fn run(src: &str) -> easy_analyzer::AnalysisResult {
    let lex = lex(src);
    assert!(lex.errors.is_empty(), "lex errors: {:?}", lex.errors);
    let program = parse_program_ast(&lex.tokens).expect("parse failed");
    analyze(&program)
}

// ===== 通用 IR 解释器（与 ir_interpreter.rs 同一份逻辑，简化复制） =====
fn value_of(env: &HashMap<String, i32>, name: &str) -> i32 {
    if let Ok(n) = name.parse::<i32>() {
        return n;
    }
    *env.get(name).unwrap_or_else(|| panic!("unknown operand `{}`", name))
}
fn run_main(quads: &[Quadruple]) -> HashMap<String, i32> {
    let start = quads
        .iter()
        .position(|q| q.op == "FUNC" && q.arg1 == "main")
        .expect("no main");
    let end = quads[start..]
        .iter()
        .position(|q| q.op == "END_FUNC" && q.arg1 == "main")
        .expect("no END_FUNC main")
        + start;
    let body = &quads[start + 1..end];

    let mut labels: HashMap<String, usize> = HashMap::new();
    for (i, q) in body.iter().enumerate() {
        if q.op == "LABEL" {
            labels.insert(q.arg1.clone(), i);
        }
    }

    let mut env: HashMap<String, i32> = HashMap::new();
    let mut pc: usize = 0;
    let mut steps = 0usize;
    while pc < body.len() {
        steps += 1;
        assert!(steps < 100_000, "interpreter ran away");
        let q = &body[pc];
        match q.op.as_str() {
            "LABEL" | "PARAM_DECL" => pc += 1,
            "=" => {
                let v = value_of(&env, &q.arg1);
                env.insert(q.result.clone(), v);
                pc += 1;
            }
            "+" | "-" | "*" | "/" => {
                let a = value_of(&env, &q.arg1);
                let b = value_of(&env, &q.arg2);
                let v = match q.op.as_str() {
                    "+" => a + b,
                    "-" => a - b,
                    "*" => a * b,
                    "/" => a / b,
                    _ => unreachable!(),
                };
                env.insert(q.result.clone(), v);
                pc += 1;
            }
            "<" | "<=" | ">" | ">=" | "==" | "!=" => {
                let a = value_of(&env, &q.arg1);
                let b = value_of(&env, &q.arg2);
                let v = match q.op.as_str() {
                    "<" => a < b,
                    "<=" => a <= b,
                    ">" => a > b,
                    ">=" => a >= b,
                    "==" => a == b,
                    "!=" => a != b,
                    _ => unreachable!(),
                };
                env.insert(q.result.clone(), v as i32);
                pc += 1;
            }
            "NEG" => {
                let v = -value_of(&env, &q.arg1);
                env.insert(q.result.clone(), v);
                pc += 1;
            }
            "GOTO" => pc = *labels.get(&q.result).expect("GOTO target missing"),
            "IF_FALSE" => {
                let v = value_of(&env, &q.arg1);
                if v == 0 {
                    pc = *labels.get(&q.result).expect("IF_FALSE target missing");
                } else {
                    pc += 1;
                }
            }
            "BREAK" => {
                let target = &q.result;
                assert!(target != "_", "BREAK 未携带目标 label: {:?}", q);
                pc = *labels.get(target).expect("BREAK target not found");
            }
            "CONTINUE" => {
                let target = &q.result;
                assert!(target != "_", "CONTINUE 未携带目标 label: {:?}", q);
                pc = *labels.get(target).expect("CONTINUE target not found");
            }
            "RETURN" => {
                if q.arg1 != "_" {
                    let v = value_of(&env, &q.arg1);
                    env.insert("__return__".to_string(), v);
                }
                break;
            }
            other => panic!("unsupported op in interpreter: {}", other),
        }
    }
    env
}

// ============================================================
// CANDIDATE BUG A: 静态越界检查未识别负字面量下标 a[-1]
// ============================================================
#[test]
fn cand_a_negative_literal_index_should_be_oob() {
    let r = run("fn main(){ let a:[i32;3]=[1,2,3]; let b:i32=a[-1]; }");
    // PDF 8.3: 下标合法范围 [0, len)；-1 显然越界。
    let has_oob = r
        .semantic_errors
        .iter()
        .any(|e| e.message.contains("越界"));
    assert!(
        has_oob,
        "期望对 a[-1] 报静态越界（PDF 8.3），实际错误：{:?}",
        r.semantic_errors
    );
}

// ============================================================
// CANDIDATE BUG B: for 循环计数和（解释器实证 for IR 正确性）
// ============================================================
#[test]
fn cand_b_for_loop_sums_correctly() {
    let quads = {
        let lex = lex("fn main()->i32{ let mut s:i32=0; for i in 0..5 { s = s + i; } return s; }");
        assert!(lex.errors.is_empty());
        let p = parse_program_ast(&lex.tokens).unwrap();
        let r = analyze(&p);
        assert!(r.semantic_errors.is_empty(), "{:?}", r.semantic_errors);
        r.quadruples
    };
    let env = run_main(&quads);
    // 0+1+2+3+4 = 10
    assert_eq!(env.get("__return__").copied(), Some(10));
}

// ============================================================
// CANDIDATE BUG C: 嵌套 while + continue + break，外层多次进入
// ============================================================
#[test]
fn cand_c_nested_while_continue_and_break() {
    // 计算: 对 i in 1..=3, 内层 j in 1..=5 但 j==3 时 continue, j==5 时 break；
    // 即对每个 i 累加 1+2+4 = 7；外层 3 次 → 21
    let quads = {
        let lex = lex(r#"
            fn main()->i32{
                let mut i:i32=0;
                let mut acc:i32=0;
                while i < 3 {
                    i = i + 1;
                    let mut j:i32 = 0;
                    while j < 10 {
                        j = j + 1;
                        if j == 3 { continue; }
                        if j == 5 { break; }
                        acc = acc + j;
                    }
                }
                return acc;
            }
        "#);
        let p = parse_program_ast(&lex.tokens).unwrap();
        let r = analyze(&p);
        assert!(r.semantic_errors.is_empty(), "{:?}", r.semantic_errors);
        r.quadruples
    };
    let env = run_main(&quads);
    assert_eq!(env.get("__return__").copied(), Some(21));
}

// ============================================================
// CANDIDATE BUG D: PARAM 顺序 / 嵌套 CALL 不应错乱
// 解释器不支持 CALL，所以做静态结构断言：
// 对 f(g(), h()) 应该是 CALL g; CALL h; PARAM t_g; PARAM t_h; CALL f.
// ============================================================
#[test]
fn cand_d_nested_call_param_order() {
    let r = run(r#"
        fn g()->i32{ return 10; }
        fn h()->i32{ return 20; }
        fn f(a:i32, b:i32)->i32{ return a + b; }
        fn main(){ let x:i32 = f(g(), h()); }
    "#);
    assert!(r.semantic_errors.is_empty(), "{:?}", r.semantic_errors);
    // 找到 main 函数体
    let s = r
        .quadruples
        .iter()
        .position(|q| q.op == "FUNC" && q.arg1 == "main")
        .unwrap();
    let e = r.quadruples[s..]
        .iter()
        .position(|q| q.op == "END_FUNC" && q.arg1 == "main")
        .unwrap()
        + s;
    let body = &r.quadruples[s + 1..e];

    // 提取所有控制类操作的顺序
    let seq: Vec<(String, String)> = body
        .iter()
        .filter(|q| matches!(q.op.as_str(), "CALL" | "PARAM"))
        .map(|q| (q.op.clone(), q.arg1.clone()))
        .collect();
    // 期望顺序: CALL g, CALL h, PARAM tg, PARAM th, CALL f
    let ops: Vec<&str> = seq.iter().map(|(o, _)| o.as_str()).collect();
    let names: Vec<&str> = seq.iter().map(|(_, n)| n.as_str()).collect();
    assert_eq!(
        ops, &["CALL", "CALL", "PARAM", "PARAM", "CALL"],
        "PARAM/CALL 顺序异常：{:?}",
        seq
    );
    assert_eq!(names[0], "g");
    assert_eq!(names[1], "h");
    assert_eq!(names[4], "f");
}

// ============================================================
// CANDIDATE BUG E: 临时变量 / LABEL 编号唯一性
// ============================================================
#[test]
fn cand_e_temp_and_label_uniqueness() {
    let r = run(r#"
        fn main(){
            let a:i32 = 1 + 2 + 3 + 4;
            let b:i32 = (1+2)*(3+4);
            if 1 { let _x:i32 = 1; } else { let _y:i32 = 2; }
            while 1 { break; }
        }
    "#);
    let mut temps = std::collections::HashSet::new();
    let mut labels = std::collections::HashSet::new();
    for q in &r.quadruples {
        // 凡是被作为 result 的临时变量（tN），不应重复定义
        if q.result.starts_with('t') && q.result.len() > 1 && q.result[1..].chars().all(|c| c.is_ascii_digit()) {
            assert!(
                temps.insert(q.result.clone()),
                "临时变量 `{}` 被定义两次：{:?}",
                q.result,
                q
            );
        }
        if q.op == "LABEL" {
            assert!(
                labels.insert(q.arg1.clone()),
                "LABEL `{}` 出现两次：{:?}",
                q.arg1,
                q
            );
        }
    }
}

// ============================================================
// CANDIDATE BUG F: loop 表达式 break 带值的解释（不依赖 CALL）
// 用 loop 模拟最简形式：让 break 返回的值通过 loop 表达式赋给变量。
// 解释器不直接支持 result_place 的语义，但 break 时已 emit (=, v, _, t_result)。
// ============================================================
#[test]
fn cand_f_loop_break_with_value_writes_result_temp() {
    let r = run(r#"
        fn main(){
            let x:i32 = loop { break 42; };
        }
    "#);
    assert!(r.semantic_errors.is_empty(), "{:?}", r.semantic_errors);
    // 期望：存在一条 (=, "42", _, tN) 把字面量写入 loop 结果 temp，
    // 然后另一条 (=, tN, _, x) 把 temp 赋给 x。
    let assign_42_to_temp = r
        .quadruples
        .iter()
        .find(|q| q.op == "=" && q.arg1 == "42" && q.result.starts_with('t'));
    assert!(
        assign_42_to_temp.is_some(),
        "缺少把 break 42 写入 loop 结果 temp 的赋值：{:?}",
        r.quadruples
    );
    let temp = assign_42_to_temp.unwrap().result.clone();
    let assign_temp_to_x = r
        .quadruples
        .iter()
        .find(|q| q.op == "=" && q.arg1 == temp && q.result == "x");
    assert!(
        assign_temp_to_x.is_some(),
        "缺少把 loop 结果 temp 赋给 x 的赋值：{:?}",
        r.quadruples
    );
}

// ============================================================
// CANDIDATE BUG G: while/for 内 `break <expr>;` 是否被允许?
// PDF 7.4 字面要求 break <expr> 必须在 loop 体内（loop 表达式）。
// 现实现：仅检查 loop_depth > 0，把 while/for 也算作 loop 体，
// 因此 `while 1 { break 1; }` 不报错。判定:可能漏报，但 PDF 7.4
// 仅有 program_7_4__2 一例(outside any loop)，未给出 while+break-expr 用例。
// 此测试观察当前行为，不强制 assert，便于报告中描述。
// ============================================================
#[test]
fn cand_g_break_value_inside_while_is_silently_accepted() {
    let r = run("fn main(){ while 1 { break 5; } }");
    // 现实现：无错。仅观察。
    println!("[OBSERVED] while+break-value errors: {:?}", r.semantic_errors);
    // 不要 panic：把观察结果写到 stdout（cargo test -- --nocapture 可看）。
}

// ============================================================
// CANDIDATE BUG H: 形参重名 fn f(a:i32, a:i32) — 是否处理？
// ============================================================
#[test]
fn cand_h_duplicate_param_names() {
    let r = run("fn f(a:i32, a:i32){} fn main(){}");
    // 观察：现实现是否报"参数名重复"？
    println!("[OBSERVED] dup-param errors: {:?}", r.semantic_errors);
    // 不强 assert。
}

// ============================================================
// CANDIDATE BUG I: 调用变量名（非函数）— 错误信息文不对题？
// ============================================================
#[test]
fn cand_i_call_a_variable() {
    let r = run("fn main(){ let a:i32 = 1; a(); }");
    println!("[OBSERVED] call-variable errors: {:?}", r.semantic_errors);
    // 修复 R-6 后：应报"变量 `a` 不是函数"，而不是"未声明的函数 a"。
    let has_not_a_fn = r
        .semantic_errors
        .iter()
        .any(|e| e.message.contains("不是函数"));
    assert!(
        has_not_a_fn,
        "调用变量应报'不是函数'：{:?}",
        r.semantic_errors
    );
}

// ============================================================
// CANDIDATE BUG J: 多语义错误并发，是否互相污染或重复
// ============================================================
#[test]
fn cand_j_multiple_errors_no_cascade() {
    let r = run(r#"
        fn main(){
            let a:i32 = 1==1;          // 1) 类型不匹配
            let b:i32 = a + (1==1);    // 2) 算术非 i32
            let c:i32 = undef + 1;     // 3) 变量未声明
            let d:i32 = c;             // 不应在 c 之后误报 (c 因 (3) 已被声明且初始化)
        }
    "#);
    let msgs: Vec<&str> = r
        .semantic_errors
        .iter()
        .map(|e| e.message.as_str())
        .collect();
    // 至少要有上述三类
    assert!(msgs.iter().any(|m| m.contains("不匹配")), "缺类型不匹配：{:?}", msgs);
    assert!(msgs.iter().any(|m| m.contains("i32")), "缺 i32 提示：{:?}", msgs);
    assert!(msgs.iter().any(|m| m.contains("未声明")), "缺未声明：{:?}", msgs);
    println!("[OBSERVED] multi-error: {:?}", msgs);
}

// ============================================================
// CANDIDATE BUG K: 借用作用域跨语句块 (借用结束于 push_scope/pop_scope)
// 内部块创建 &mut a 离开块后，再 &a 是否被允许？
// ============================================================
#[test]
fn cand_k_borrow_scope_drop_on_block_exit() {
    let r = run(r#"
        fn main(){
            let mut a:i32 = 1;
            { let _b = &mut a; }   // 借用在此结束（朴素：scope-pop 即释放）
            let _c = &a;           // 不应报错
        }
    "#);
    // 朴素借用：scope-pop 释放借用 → 应通过
    assert!(
        r.semantic_errors.is_empty(),
        "块退出后再借用应被允许（朴素借用语义）：{:?}",
        r.semantic_errors
    );
}

// ============================================================
// CANDIDATE BUG L: 通过 if-else 表达式分支类型不一致
// ============================================================
#[test]
fn cand_l_if_branches_mixed_unit_value() {
    let r = run("fn main(){ let x:i32 = if 1==1 { 1 } else { }; }");
    // 期望：报 if 表达式分支类型不一致（i32 vs ()）
    let mismatch = r
        .semantic_errors
        .iter()
        .any(|e| e.message.contains("分支类型不一致"));
    assert!(
        mismatch,
        "期望 if 分支类型不一致错误：{:?}",
        r.semantic_errors
    );
}

// ============================================================
// CANDIDATE BUG M: 数组字面量空 + 长度非零（已修复回归）
// ============================================================
#[test]
fn cand_m_empty_array_lit_mismatched_length_msg() {
    let r = run("fn main(){ let a:[i32;2]=[]; }");
    let msgs: Vec<&str> = r.semantic_errors.iter().map(|e| e.message.as_str()).collect();
    println!("[OBSERVED] [i32;2]=[] errors: {:?}", msgs);
    assert!(
        !r.semantic_errors.is_empty(),
        "[i32;2]=[] 应报错（数量不匹配）"
    );
}

// ============================================================
// CANDIDATE BUG N: 对函数名作 RHS 表达式（非调用形式）
// ============================================================
#[test]
fn cand_n_function_as_plain_rvalue() {
    let r = run("fn g(){} fn main(){ let a:i32 = g; }");
    println!("[OBSERVED] function-as-rvalue errors: {:?}", r.semantic_errors);
    // 期望：类型不匹配（<函数> vs i32），不应是"未声明"。
    let has_mismatch = r
        .semantic_errors
        .iter()
        .any(|e| e.message.contains("不匹配"));
    assert!(
        has_mismatch || r.semantic_errors.iter().any(|e| e.message.contains("函数")),
        "函数名作 RHS 应被合理报错：{:?}",
        r.semantic_errors
    );
}

// ============================================================
// CANDIDATE BUG O: 嵌套循环 — break/continue 仅影响最内层
// 通过解释器实测 i,j 跨层数值
// ============================================================
#[test]
fn cand_o_nested_loop_continue_only_inner() {
    let quads = {
        let lex = lex(r#"
            fn main()->i32{
                let mut i:i32 = 0;
                let mut total:i32 = 0;
                while i < 3 {
                    i = i + 1;
                    let mut j:i32 = 0;
                    while j < 3 {
                        j = j + 1;
                        if j == 2 { continue; }
                        total = total + 1;
                    }
                }
                return total;
            }
        "#);
        let p = parse_program_ast(&lex.tokens).unwrap();
        let r = analyze(&p);
        assert!(r.semantic_errors.is_empty(), "{:?}", r.semantic_errors);
        r.quadruples
    };
    let env = run_main(&quads);
    // 内层每次走 j=1,3 → +2，外层 3 次 → 6
    assert_eq!(env.get("__return__").copied(), Some(6));
}

// ============================================================
// CANDIDATE BUG P: gen_let 对未初始化变量类型推断的边界
// `let x; x = 1; x = 2;` —— 第二次赋值 x 不是 mut，应该报错。
// ============================================================
#[test]
fn cand_p_uninit_then_two_assigns_immutable_rejected() {
    let r = run(r#"
        fn main(){
            let x;
            x = 1;
            x = 2;
        }
    "#);
    let msgs: Vec<&str> = r.semantic_errors.iter().map(|e| e.message.as_str()).collect();
    println!("[OBSERVED] late-init then reassign errors: {:?}", msgs);
    // 当前实现：第一次 x=1 把 initialized=true，第二次 x=2 时 immutable&&initialized → 报错。
    let has_immutable = msgs.iter().any(|m| m.contains("不可变"));
    assert!(
        has_immutable,
        "未声明 mut 的 let x; 第二次赋值应被拒：{:?}",
        msgs
    );
}

// ============================================================
// CANDIDATE BUG Q: for 循环变量带显式类型注解被静默丢弃
// 例: for mut i:[i32;3] in 0..3 {} — 用户给的类型与 range 推断不一致，
// 当前实现完全忽略 binding.ty，强制 i 为 I32，不报任何错。
// ============================================================
#[test]
fn cand_q_for_binding_explicit_type_silently_ignored() {
    let r = run("fn main(){ for mut i:[i32;3] in 0..3 { let _x:[i32;3]=i; } }");
    // 体内 `let _x:[i32;3]=i;` —— 若 i 被当作 [i32;3]，应无错；若 i 实际是 i32，应报类型不匹配。
    // 当前实现：i 被强制为 i32 → 报"声明类型 [i32;3] 与 i32 不匹配"。这间接揭示 binding.ty 被丢弃。
    let mismatch = r
        .semantic_errors
        .iter()
        .any(|e| e.message.contains("不匹配"));
    println!("[OBSERVED] for-binding-type errors: {:?}", r.semantic_errors);
    assert!(mismatch, "for 循环变量类型注解被丢弃且未与 iterable 校验：{:?}", r.semantic_errors);
}

// ============================================================
// CANDIDATE BUG R: 重名形参 fn f(a:i32, a:i32) 未报错
// IR 里出现两条 PARAM_DECL a, 符号表只保留后者。
// ============================================================
#[test]
fn cand_r_duplicate_param_emits_two_decls_and_no_error() {
    let r = run("fn f(a:i32, a:i32){} fn main(){}");
    let decls = r
        .quadruples
        .iter()
        .filter(|q| q.op == "PARAM_DECL" && q.arg1 == "a")
        .count();
    println!("[OBSERVED] dup-param PARAM_DECL count: {}, errors: {:?}", decls, r.semantic_errors);
    // 同名形参显然是错误：要么报错，要么 IR 不该重复。
    let reported = r.semantic_errors.iter().any(|e| e.message.contains("重复") || e.message.contains("重名"));
    assert!(
        reported || decls < 2,
        "fn f(a,a) 应报重名 或 至少不发两份 PARAM_DECL，实际：errors={:?}, decls={}",
        r.semantic_errors,
        decls
    );
}

// ============================================================
// CANDIDATE BUG S: 静态越界对大正字面量溢出 isize 时被静默跳过
// ============================================================
#[test]
fn cand_s_overflow_index_skipped() {
    let r = run("fn main(){ let a:[i32;3]=[1,2,3]; let b:i32=a[99999999999999999999]; }");
    let has_oob = r.semantic_errors.iter().any(|e| e.message.contains("越界"));
    println!("[OBSERVED] overflow-index errors: {:?}", r.semantic_errors);
    // 巨大字面量明显越界（合法范围 [0,3)），但当前实现因 isize 解析失败而漏报。
    assert!(has_oob, "极大字面量下标应报越界：{:?}", r.semantic_errors);
}

// ============================================================
// CANDIDATE BUG T: gen_let init 是 Expr::Block，块内 let 形如 `let mut y;`
// 类型推断成功（块尾表达式为 1，块的类型为 i32），但块的 let y 仍应被报
// 已由 BUG #11 修复 — 这里做嵌套块（块表达式套块表达式）回归。
// ============================================================
#[test]
fn cand_t_nested_block_expr_uninferred_inner() {
    let r = run("fn main(){ let x:i32 = { { let mut z; 5 } }; }");
    let has_z_err = r.semantic_errors.iter().any(|e| e.message.contains("z") && e.message.contains("无法推断"));
    println!("[OBSERVED] nested block uninferred errors: {:?}", r.semantic_errors);
    assert!(has_z_err, "嵌套块内的 let mut z; 应被报为无法推断：{:?}", r.semantic_errors);
}
