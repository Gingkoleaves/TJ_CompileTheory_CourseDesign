//! 最小四元式解释器：验证 BUG #1/#2/#3 修复后 BREAK/CONTINUE
//! 携带正确的目标 label，且循环程序能正常终止并产出预期结果。
//!
//! 解释器仅覆盖 main 函数中、不涉及函数调用/引用/数组/元组的子集。

use std::collections::HashMap;

use easy_analyzer::analyze;
use easy_analyzer::ir::Quadruple;
use easy_lexer::lex;
use easy_parser::parse_program_ast;

fn gen_ir(src: &str) -> Vec<Quadruple> {
    let lex = lex(src);
    assert!(lex.errors.is_empty(), "lex errors: {:?}", lex.errors);
    let program = parse_program_ast(&lex.tokens).expect("parse failed");
    let result = analyze(&program);
    assert!(
        result.semantic_errors.is_empty(),
        "semantic errors: {:?}",
        result.semantic_errors
    );
    result.quadruples
}

fn value_of(env: &HashMap<String, i32>, name: &str) -> i32 {
    if let Ok(n) = name.parse::<i32>() {
        return n;
    }
    *env.get(name).unwrap_or_else(|| panic!("unknown operand `{}`", name))
}

/// 解释 main 函数体（位于 FUNC main … END_FUNC main 之间），返回最终变量环境。
fn run_main(quads: &[Quadruple]) -> HashMap<String, i32> {
    let start = quads
        .iter()
        .position(|q| q.op == "FUNC" && q.arg1 == "main")
        .expect("no main function");
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
        assert!(steps < 100_000, "interpreter ran away (infinite loop?)");
        let q = &body[pc];
        match q.op.as_str() {
            "LABEL" | "PARAM_DECL" | "FUNC" | "END_FUNC" => {
                pc += 1;
            }
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
            "GOTO" => {
                pc = *labels.get(&q.result).expect("GOTO target missing");
            }
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
                assert!(
                    target != "_",
                    "BREAK 未携带目标 label（BUG #1 回归）: {:?}",
                    q
                );
                pc = *labels
                    .get(target)
                    .unwrap_or_else(|| panic!("BREAK target `{}` not found", target));
            }
            "CONTINUE" => {
                let target = &q.result;
                assert!(
                    target != "_",
                    "CONTINUE 未携带目标 label（BUG #2 回归）: {:?}",
                    q
                );
                pc = *labels
                    .get(target)
                    .unwrap_or_else(|| panic!("CONTINUE target `{}` not found", target));
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

#[test]
fn break_terminates_while_loop() {
    // i 从 0 累加到 3 后 break，期望 i == 3
    let quads = gen_ir(
        r#"
        fn main() -> i32 {
            let mut i:i32 = 0;
            while i < 10 {
                if i == 3 { break; }
                i = i + 1;
            }
            return i;
        }
        "#,
    );
    let env = run_main(&quads);
    assert_eq!(env.get("__return__").copied(), Some(3));
}

#[test]
fn continue_skips_iteration_in_while() {
    // 求 0..10 中非 5 之和：0+1+2+3+4+6+7+8+9 = 40
    let quads = gen_ir(
        r#"
        fn main() -> i32 {
            let mut i:i32 = 0;
            let mut sum:i32 = 0;
            while i < 10 {
                i = i + 1;
                if i == 5 { continue; }
                sum = sum + i;
            }
            return sum;
        }
        "#,
    );
    let env = run_main(&quads);
    // i 累加完毕为 10；sum = 1+2+3+4+6+7+8+9+10 = 50
    assert_eq!(env.get("__return__").copied(), Some(50));
}

#[test]
fn nested_loops_break_innermost() {
    // 内层 break 只跳出内层；外层继续到 i==2 时整体退出
    let quads = gen_ir(
        r#"
        fn main() -> i32 {
            let mut i:i32 = 0;
            let mut acc:i32 = 0;
            while i < 5 {
                let mut j:i32 = 0;
                while j < 10 {
                    if j == 2 { break; }
                    acc = acc + 1;
                    j = j + 1;
                }
                i = i + 1;
            }
            return acc;
        }
        "#,
    );
    let env = run_main(&quads);
    // 外层 i: 0..5（5 次），每次内层累加 acc 2 次 → 10
    assert_eq!(env.get("__return__").copied(), Some(10));
}

#[test]
fn break_and_continue_carry_target_label() {
    // 静态断言：所有 BREAK / CONTINUE 四元式 result 字段都不是占位符
    let quads = gen_ir(
        r#"
        fn main() {
            let mut i:i32 = 0;
            while i < 5 {
                i = i + 1;
                if i == 2 { continue; }
                if i == 4 { break; }
            }
        }
        "#,
    );
    for q in &quads {
        if q.op == "BREAK" {
            assert_ne!(q.result, "_", "BREAK 第四元仍为占位（BUG #1 回归）: {:?}", q);
        }
        if q.op == "CONTINUE" {
            assert_ne!(q.result, "_", "CONTINUE 第四元仍为占位（BUG #2 回归）: {:?}", q);
        }
    }
}
