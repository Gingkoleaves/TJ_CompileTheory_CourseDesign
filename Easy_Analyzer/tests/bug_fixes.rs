//! 中等 BUG 修复后的最小复现测试。
//! - BUG #4：空数组字面量 `let a:[i32;0]=[];` 不应报"类型不匹配"
//! - BUG #5：Server `TokenView.type_enum` serde 字段名（由 Easy_Server 的测试覆盖；
//!   此处仅作回归占位以集中跟踪修复点）
//! - BUG #6：函数名作实参时应报"实参类型不一致"而非"未声明"

use easy_analyzer::analyze;
use easy_lexer::lex;
use easy_parser::parse_program_ast;

fn run(src: &str) -> easy_analyzer::AnalysisResult {
    let lex = lex(src);
    assert!(lex.errors.is_empty(), "lexer errors: {:?}", lex.errors);
    let program = parse_program_ast(&lex.tokens).expect("parse failed");
    analyze(&program)
}

#[test]
fn bug4_empty_array_literal_zero_length_ok() {
    let r = run("fn main(){ let a:[i32;0]=[]; }");
    assert!(
        r.semantic_errors.is_empty(),
        "[i32;0] = [] 应被接受，实际报错：{:?}",
        r.semantic_errors
    );
}

#[test]
fn bug4_empty_array_still_rejects_mismatched_length() {
    // 空数组对长度非零的声明仍应报错（长度不匹配，不是元素类型）
    let r = run("fn main(){ let a:[i32;2]=[]; }");
    assert!(
        !r.semantic_errors.is_empty(),
        "[i32;2] = [] 应报长度不匹配，实际无错"
    );
}

#[test]
fn bug6_function_name_as_argument_reports_type_mismatch() {
    // PDF program_3_3__4: 用函数名作实参传给期望 i32 的形参 → 期望"实参类型不一致"
    let r = run(
        r#"
        fn callee(a:i32) {}
        fn helper() {}
        fn main() {
            callee(helper);
        }
        "#,
    );
    let msgs: Vec<String> = r.semantic_errors.iter().map(|e| e.message.clone()).collect();
    assert!(
        msgs.iter().any(|m| m.contains("实参") && m.contains("类型")),
        "应报实参类型不一致，实际错误：{:?}",
        msgs
    );
    assert!(
        !msgs.iter().any(|m| m.contains("`helper`") && m.contains("未声明")),
        "不应把 `helper`（函数名）报为未声明变量，实际：{:?}",
        msgs
    );
}

#[test]
fn bug6_truly_undeclared_still_reports_undeclared() {
    // 回归：真正未声明的标识符仍应报"未声明"，不能被回退误吞
    let r = run("fn main(){ let x:i32 = nope; }");
    assert!(
        r.semantic_errors
            .iter()
            .any(|e| e.message.contains("未声明")),
        "纯未声明标识符应保留'未声明'报错：{:?}",
        r.semantic_errors
    );
}

#[test]
fn bug7_duplicate_function_emits_single_ir() {
    // 重复函数定义应报错且 IR 中只保留首次函数体（一份 FUNC/END_FUNC）
    let r = run("fn f(){} fn f(){} fn main(){}");
    assert!(
        r.semantic_errors
            .iter()
            .any(|e| e.message.contains("重复定义")),
        "应报'重复定义'：{:?}",
        r.semantic_errors
    );
    let func_f_count = r
        .quadruples
        .iter()
        .filter(|q| q.op == "FUNC" && q.arg1 == "f")
        .count();
    assert_eq!(
        func_f_count, 1,
        "重复函数 f 在 IR 中应只出现一次，实际 {} 次",
        func_f_count
    );
}

#[test]
fn bug8_non_unit_function_without_return_still_has_terminator() {
    // `fn f()->i32 { }` 之前不发任何 RETURN；修复后函数末尾必有 RETURN 终结子
    let r = run("fn f()->i32 { } fn main(){}");
    // 取 fn f 的 IR 片段
    let start = r
        .quadruples
        .iter()
        .position(|q| q.op == "FUNC" && q.arg1 == "f")
        .expect("FUNC f missing");
    let end = r.quadruples[start..]
        .iter()
        .position(|q| q.op == "END_FUNC" && q.arg1 == "f")
        .map(|i| i + start)
        .expect("END_FUNC f missing");
    let has_return = r.quadruples[start..end].iter().any(|q| q.op == "RETURN");
    assert!(
        has_return,
        "fn f()->i32{{}} 的 IR 缺少 RETURN 终结子：{:?}",
        &r.quadruples[start..=end]
    );
}

#[test]
fn bug9_unit_if_expr_does_not_emit_assign_to_temp() {
    // `if 1>0 {} else {}` 两分支都是 Unit，不应分配/赋值 if 结果 temp
    let r = run("fn main() { if 1>0 { } else { } }");
    let assigns_to_temp = r
        .quadruples
        .iter()
        .filter(|q| q.op == "=" && q.result.starts_with('t'))
        .count();
    assert_eq!(
        assigns_to_temp, 0,
        "Unit/Unit if 不应产生针对 temp 的赋值 IR：{:?}",
        r.quadruples
    );
}

#[test]
fn bug11_block_expr_reports_uninferred_inner_var() {
    // 块表达式内部的无法推断变量也应被报出
    let r = run("fn main() { let x = { let y; 1 }; }");
    assert!(
        r.semantic_errors
            .iter()
            .any(|e| e.message.contains("y") && e.message.contains("无法推断")),
        "块表达式内未推断变量应被报出：{:?}",
        r.semantic_errors
    );
}

#[test]
fn bug12_break_type_mismatch_skips_result_assign() {
    // 同一 loop 中第二个 break 类型与第一个不一致：报类型不一致，且不发射 = 污染结果 temp
    let r = run(
        r#"
        fn main() {
            let mut x:i32 = loop {
                break 1;
                break 1==1;
            };
        }
        "#,
    );
    assert!(
        r.semantic_errors
            .iter()
            .any(|e| e.message.contains("类型不一致")),
        "应报多个 break 类型不一致：{:?}",
        r.semantic_errors
    );
    // 第一个 break 发 = ，第二个 break 应跳过 = ，因此 loop 内只发 1 次 (=, 1, _, t_result)
    let result_assigns = r
        .quadruples
        .iter()
        .filter(|q| q.op == "=" && q.arg1 == "1" && q.result.starts_with('t'))
        .count();
    assert_eq!(
        result_assigns, 1,
        "类型不一致的 break 不应再发 = ，实际 result 赋值次数 {} ：{:?}",
        result_assigns, r.quadruples
    );
}

#[test]
fn bug13_array_oob_error_carries_name() {
    // 数组越界错误必须带数组名 `a`
    let r = run(
        r#"
        fn main() {
            let a:[i32;3] = [1,2,3];
            let x:i32 = a[5];
        }
        "#,
    );
    assert!(
        r.semantic_errors
            .iter()
            .any(|e| e.message.contains("数组 `a`") && e.message.contains("越界")),
        "越界错误应带数组名 `a`：{:?}",
        r.semantic_errors
    );
}

#[test]
fn bug14_undeclared_function_carries_rule_number() {
    let r = run("fn main() { undef(); }");
    assert!(
        r.semantic_errors
            .iter()
            .any(|e| e.message.contains("未声明的函数") && e.message.contains("规则 3.5")),
        "未声明函数错误应带（规则 3.5）：{:?}",
        r.semantic_errors
    );
}

// ============================================================
// 第二轮 BUG 修复回归（R-1 ~ R-6）
// ============================================================

#[test]
fn bug_r1_negative_literal_index_reports_oob() {
    // a[-1] 应被报为静态越界（规则 8.3）
    let r = run("fn main(){ let a:[i32;3]=[1,2,3]; let b:i32=a[-1]; }");
    assert!(
        r.semantic_errors
            .iter()
            .any(|e| e.message.contains("越界") && e.message.contains("-1")),
        "a[-1] 应报负字面量越界：{:?}",
        r.semantic_errors
    );
}

#[test]
fn bug_r2_oversized_literal_index_reports_oob() {
    // 大于 u128 的数字字面量下标，解析失败也应视为越界
    let r = run("fn main(){ let a:[i32;3]=[1,2,3]; let b:i32=a[99999999999999999999]; }");
    assert!(
        r.semantic_errors
            .iter()
            .any(|e| e.message.contains("越界")),
        "极大字面量下标应报越界：{:?}",
        r.semantic_errors
    );
}

#[test]
fn bug_r3_duplicate_param_name_reported_and_single_decl() {
    let r = run("fn f(a:i32, a:i32){} fn main(){}");
    // 1) 应报"重名"错误
    assert!(
        r.semantic_errors
            .iter()
            .any(|e| e.message.contains("形参") && e.message.contains("重名")),
        "fn f(a, a) 应报形参重名：{:?}",
        r.semantic_errors
    );
    // 2) IR 中 PARAM_DECL a 只应出现一次
    let decls = r
        .quadruples
        .iter()
        .filter(|q| q.op == "PARAM_DECL" && q.arg1 == "a")
        .count();
    assert_eq!(
        decls, 1,
        "fn f(a, a) 的 IR 中 PARAM_DECL a 应只出现 1 次，实际 {}",
        decls
    );
}

#[test]
fn bug_r4_for_binding_annotation_mismatch_reported() {
    // for 循环变量上写了与 range 不兼容的类型注解，应报"不一致"
    let r = run("fn main(){ for i:[i32;3] in 0..3 { } }");
    assert!(
        r.semantic_errors
            .iter()
            .any(|e| e.message.contains("for 循环变量") && e.message.contains("不一致")),
        "for 循环变量类型注解与迭代元素类型不一致应被报出：{:?}",
        r.semantic_errors
    );
}

#[test]
fn bug_r4_for_binding_annotation_matching_silent() {
    // 写了 i32（与 range 一致）应无 R-4 类型错
    let r = run("fn main(){ for i:i32 in 0..3 { } }");
    assert!(
        !r.semantic_errors
            .iter()
            .any(|e| e.message.contains("for 循环变量")),
        "for i:i32 in 0..3 应无 for-binding 错误：{:?}",
        r.semantic_errors
    );
}

#[test]
fn bug_r5_empty_array_length_mismatch_msg_is_clean() {
    // 错误信息不应出现内部占位 <类型错误>
    let r = run("fn main(){ let a:[i32;2]=[]; }");
    let msgs: Vec<String> = r.semantic_errors.iter().map(|e| e.message.clone()).collect();
    assert!(!msgs.is_empty(), "[i32;2] = [] 应报错");
    assert!(
        msgs.iter().all(|m| !m.contains("<类型错误>")),
        "错误信息不应暴露 <类型错误>：{:?}",
        msgs
    );
    assert!(
        msgs.iter().any(|m| m.contains("数组长度不匹配")),
        "应报数组长度不匹配：{:?}",
        msgs
    );
}

#[test]
fn bug_r5_function_as_rvalue_msg_is_clean() {
    // 函数名作 RValue 时，错误信息不应出现 <函数>
    let r = run("fn g(){} fn main(){ let a:i32 = g; }");
    let msgs: Vec<String> = r.semantic_errors.iter().map(|e| e.message.clone()).collect();
    assert!(!msgs.is_empty(), "let a:i32 = g 应报错");
    assert!(
        msgs.iter().all(|m| !m.contains("<函数>")),
        "错误信息不应暴露 <函数> 占位：{:?}",
        msgs
    );
    assert!(
        msgs.iter()
            .any(|m| m.contains("函数 `g`") && m.contains("不能直接用作值")),
        "应给出针对函数名作值的专门提示：{:?}",
        msgs
    );
}

// ============================================================
// 第三轮 BUG 修复回归（B-1 ~ B-5）
// ============================================================

#[test]
fn bug_b1_for_continue_does_not_skip_increment() {
    // for + continue 的语义：continue 跳到 label_cont，从该 label 前进必须先经过
    // 自增 `i = i + 1`，再到 GOTO 回 label_start（条件检查）。
    // 原 bug：continue 跳到 label_start 本身，越过自增 → 死循环。
    let r = run(
        r#"
        fn main() {
            for i in 0..5 {
                if i == 2 { continue; }
            }
        }
        "#,
    );
    let q = &r.quadruples;
    let cont_target = q
        .iter()
        .find(|x| x.op == "CONTINUE")
        .map(|x| x.result.clone())
        .expect("应有 CONTINUE 四元式");
    let target_idx = q
        .iter()
        .position(|x| x.op == "LABEL" && x.arg1 == cont_target)
        .expect("continue 目标 label 应存在");
    let suffix = &q[target_idx..];
    let inc_pos = suffix
        .iter()
        .position(|x| x.op == "+" && x.arg1 == "i" && x.arg2 == "1");
    let if_false_pos = suffix.iter().position(|x| x.op == "IF_FALSE");
    let inc_pos = inc_pos.expect("从 continue 目标向后必须能到达自增 i = i + 1");
    if let Some(cmp) = if_false_pos {
        assert!(
            inc_pos < cmp,
            "B-1 回归：从 continue 目标 (`{}`) 走，自增必须先于下一次条件检查；\
             实际自增相对位置 {}，IF_FALSE 相对位置 {}",
            cont_target,
            inc_pos,
            cmp
        );
    }
}

#[test]
fn bug_b2_array_index_assign_writes_back_to_array() {
    // a[0] = 5; 必须产出 `[]= 5 0 a` 类写回 IR，不能只写到 temp。
    let r = run(
        r#"
        fn main() {
            let mut a:[i32;3] = [1,2,3];
            a[0] = 5;
        }
        "#,
    );
    assert!(
        r.semantic_errors.is_empty(),
        "合法 a[0] = 5 不应报错：{:?}",
        r.semantic_errors
    );
    let has_array_store = r
        .quadruples
        .iter()
        .any(|q| q.op == "[]=" && q.arg1 == "5" && q.arg2 == "0" && q.result == "a");
    assert!(
        has_array_store,
        "B-2 回归：a[0] = 5 应产出 `[]= 5 0 a`，实际 IR：{:?}",
        r.quadruples
    );
}

#[test]
fn bug_b2_array_index_assign_preserves_oob_check() {
    // 写入越界下标仍应报错（不能因为换 op 丢失检查）
    let r = run(
        r#"
        fn main() {
            let mut a:[i32;3] = [1,2,3];
            a[5] = 4;
        }
        "#,
    );
    assert!(
        r.semantic_errors
            .iter()
            .any(|e| e.message.contains("越界") && e.message.contains("数组 `a`")),
        "B-2 回归：写入越界仍应报错并带数组名：{:?}",
        r.semantic_errors
    );
}

#[test]
fn bug_b2_immutable_array_element_assignment_rejected() {
    // 不可变数组写元素仍应报错（规则 8.3）
    let r = run(
        r#"
        fn main() {
            let a:[i32;3] = [1,2,3];
            a[0] = 4;
        }
        "#,
    );
    assert!(
        r.semantic_errors
            .iter()
            .any(|e| e.message.contains("不可变") && e.message.contains("`a`")),
        "B-2 回归：不可变数组元素赋值应报错：{:?}",
        r.semantic_errors
    );
}

#[test]
fn bug_b3_tuple_field_assign_writes_back_to_tuple() {
    let r = run(
        r#"
        fn main() {
            let mut a:(i32,i32) = (1,2);
            a.0 = 5;
        }
        "#,
    );
    assert!(
        r.semantic_errors.is_empty(),
        "合法 a.0 = 5 不应报错：{:?}",
        r.semantic_errors
    );
    let has_tuple_store = r
        .quadruples
        .iter()
        .any(|q| q.op == ".=" && q.arg1 == "5" && q.arg2 == "0" && q.result == "a");
    assert!(
        has_tuple_store,
        "B-3 回归：a.0 = 5 应产出 `.= 5 0 a`，实际 IR：{:?}",
        r.quadruples
    );
}

#[test]
fn bug_b3_tuple_oob_field_assign_rejected() {
    let r = run(
        r#"
        fn main() {
            let mut a:(i32,i32) = (1,2);
            a.5 = 3;
        }
        "#,
    );
    assert!(
        r.semantic_errors
            .iter()
            .any(|e| e.message.contains("元组下标") && e.message.contains("越界")),
        "B-3 回归：元组字段越界写入应报错：{:?}",
        r.semantic_errors
    );
}

#[test]
fn bug_b3_immutable_tuple_field_assignment_rejected() {
    let r = run(
        r#"
        fn main() {
            let a:(i32,i32) = (1,2);
            a.0 = 5;
        }
        "#,
    );
    assert!(
        r.semantic_errors
            .iter()
            .any(|e| e.message.contains("不可变") && e.message.contains("`a`")),
        "B-3 回归：不可变元组字段赋值应报错：{:?}",
        r.semantic_errors
    );
}

#[test]
fn bug_b4_function_operand_in_binary_op_has_friendly_message() {
    // 错误信息既不应包含 <函数>，也应给出"加 `()` 调用"的提示
    let r = run("fn g(){} fn main(){ let a:i32 = g + 1; }");
    let msgs: Vec<String> = r
        .semantic_errors
        .iter()
        .map(|e| e.message.clone())
        .collect();
    assert!(
        msgs.iter().all(|m| !m.contains("<函数>")),
        "B-4 回归：错误信息不应含 <函数> 占位：{:?}",
        msgs
    );
    assert!(
        msgs.iter().any(|m| m.contains("函数名 `g`") && m.contains("`()` 调用")),
        "B-4 回归：应给出'函数名 `g`（请加 `()` 调用）'提示：{:?}",
        msgs
    );
}

#[test]
fn bug_b5_for_non_range_iterable_emits_no_loop_scaffold() {
    // 错误恢复路径下不应再发射含 `_` 操作数的 for 骨架 IR
    let r = run(
        r#"
        fn main() {
            for i in 0+5 { }
        }
        "#,
    );
    assert!(
        r.semantic_errors
            .iter()
            .any(|e| e.message.contains("for 迭代结构必须是范围")),
        "B-5 回归：非 range 应报错：{:?}",
        r.semantic_errors
    );
    // 不应出现 `= _ _ i` 或 `< i _ tN` 的占位骨架
    let has_placeholder_assign = r
        .quadruples
        .iter()
        .any(|q| q.op == "=" && q.arg1 == "_" && q.result == "i");
    assert!(
        !has_placeholder_assign,
        "B-5 回归：错误恢复仍发射 `= _ _ i` 骨架：{:?}",
        r.quadruples
    );
    let has_placeholder_cmp = r
        .quadruples
        .iter()
        .any(|q| q.op == "<" && q.arg1 == "i" && q.arg2 == "_");
    assert!(
        !has_placeholder_cmp,
        "B-5 回归：错误恢复仍发射 `< i _ t` 比较：{:?}",
        r.quadruples
    );
}

// ============================================================
// 第四轮 BUG 修复回归（Round 4：嵌套写回 / Deref 类型 / for-array / then-only if）
// ============================================================

#[test]
fn bug_r4n_nested_array_index_assign_writes_back() {
    // 复现：a[0][1] = 9 必须把改后的内层数组回写到 a[0]，否则 a 不变。
    // IR 应为：INDEX a 0 t1; []= 9 1 t1; []= t1 0 a
    let r = run(
        r#"
        fn main() {
            let mut a:[[i32;2];2] = [[1,2],[3,4]];
            a[0][1] = 9;
        }
        "#,
    );
    assert!(
        r.semantic_errors.is_empty(),
        "合法 a[0][1] = 9 不应报错：{:?}",
        r.semantic_errors
    );
    let inner_store = r
        .quadruples
        .iter()
        .position(|q| q.op == "[]=" && q.arg1 == "9" && q.arg2 == "1");
    let inner_store = inner_store.expect("内层 []= 9 1 <temp> 应存在");
    let temp = r.quadruples[inner_store].result.clone();
    let writeback = r
        .quadruples
        .iter()
        .any(|q| q.op == "[]=" && q.arg1 == temp && q.arg2 == "0" && q.result == "a");
    assert!(
        writeback,
        "嵌套数组写回缺失：应有 `[]= {} 0 a`，实际 IR：{:?}",
        temp, r.quadruples
    );
}

#[test]
fn bug_r4n_nested_tuple_field_assign_writes_back() {
    // 复现：a.0.1 = 9 必须把改后的子元组回写到 a.0
    let r = run(
        r#"
        fn main() {
            let mut a:((i32,i32),i32) = ((1,2),3);
            a.0.1 = 9;
        }
        "#,
    );
    assert!(
        r.semantic_errors.is_empty(),
        "合法 a.0.1 = 9 不应报错：{:?}",
        r.semantic_errors
    );
    let inner_store = r
        .quadruples
        .iter()
        .position(|q| q.op == ".=" && q.arg1 == "9" && q.arg2 == "1");
    let inner_store = inner_store.expect("内层 .= 9 1 <temp> 应存在");
    let temp = r.quadruples[inner_store].result.clone();
    let writeback = r
        .quadruples
        .iter()
        .any(|q| q.op == ".=" && q.arg1 == temp && q.arg2 == "0" && q.result == "a");
    assert!(
        writeback,
        "嵌套元组字段写回缺失：应有 `.= {} 0 a`，实际 IR：{:?}",
        temp, r.quadruples
    );
}

#[test]
fn bug_r4n_mixed_array_in_tuple_writes_back() {
    // a.0[1] = 9 ：array-in-tuple，写回链应是 .= t 0 a
    let r = run(
        r#"
        fn main() {
            let mut a:([i32;3],i32) = ([1,2,3],4);
            a.0[1] = 9;
        }
        "#,
    );
    assert!(
        r.semantic_errors.is_empty(),
        "合法 a.0[1] = 9 不应报错：{:?}",
        r.semantic_errors
    );
    let inner_store = r
        .quadruples
        .iter()
        .position(|q| q.op == "[]=" && q.arg1 == "9" && q.arg2 == "1");
    let inner_store = inner_store.expect("内层 []= 9 1 <temp> 应存在");
    let temp = r.quadruples[inner_store].result.clone();
    let writeback = r
        .quadruples
        .iter()
        .any(|q| q.op == ".=" && q.arg1 == temp && q.arg2 == "0" && q.result == "a");
    assert!(
        writeback,
        "混合写回缺失：应有 `.= {} 0 a`，实际 IR：{:?}",
        temp, r.quadruples
    );
}

#[test]
fn bug_r4n_deref_assign_checks_pointee_type() {
    // *b = (1,2) 中 b:&mut i32，右值 (i32,i32) 与 i32 不兼容，必须报错。
    let r = run(
        r#"
        fn main() {
            let mut a:i32 = 1;
            let b:&mut i32 = &mut a;
            *b = (1, 2);
        }
        "#,
    );
    assert!(
        r.semantic_errors
            .iter()
            .any(|e| e.message.contains("不匹配")),
        "解引用赋值类型不匹配应被报：{:?}",
        r.semantic_errors
    );
}

#[test]
fn bug_r4n_deref_assign_same_type_ok() {
    // 同类型的 *b = v 不应报错（避免新检查误报）
    let r = run(
        r#"
        fn main() {
            let mut a:i32 = 1;
            let b:&mut i32 = &mut a;
            *b = 9;
        }
        "#,
    );
    assert!(
        r.semantic_errors.is_empty(),
        "合法 *b = 9（i32 / &mut i32）不应报错：{:?}",
        r.semantic_errors
    );
}

#[test]
fn bug_r4n_for_in_array_generates_loop_ir() {
    // PDF 8.2：数组可迭代。for x in a 应正常生成循环骨架，且 body 中可读 x。
    let r = run(
        r#"
        fn main() {
            let a:[i32;3] = [10,20,30];
            let mut sum:i32 = 0;
            for x in a {
                sum = sum + x;
            }
        }
        "#,
    );
    assert!(
        r.semantic_errors.is_empty(),
        "for x in a 不应报错（数组可迭代）：{:?}",
        r.semantic_errors
    );
    // 必须出现 `INDEX a <idx-temp> x` 把当前元素写入循环变量
    let has_elem_load = r
        .quadruples
        .iter()
        .any(|q| q.op == "INDEX" && q.arg1 == "a" && q.result == "x");
    assert!(
        has_elem_load,
        "for-array 应在 body 前发出 `INDEX a <i> x`，实际 IR：{:?}",
        r.quadruples
    );
    // 循环长度应以字面量 3 出现在比较中
    let has_len_cmp = r
        .quadruples
        .iter()
        .any(|q| q.op == "<" && q.arg2 == "3");
    assert!(
        has_len_cmp,
        "for-array 应有 `< <i> 3 <t>` 边界比较：{:?}",
        r.quadruples
    );
}

#[test]
fn bug_r4n_for_in_non_iterable_still_rejected() {
    // 非数组、非范围的迭代源仍应报错（错误信息更新为含"或数组"）
    let r = run(
        r#"
        fn main() {
            let k:i32 = 1;
            for i in k { let _x:i32 = i; }
        }
        "#,
    );
    assert!(
        r.semantic_errors
            .iter()
            .any(|e| e.message.contains("for 迭代结构必须是范围")),
        "非可迭代源应报错：{:?}",
        r.semantic_errors
    );
}

#[test]
fn bug_r4n_then_only_if_with_nonunit_tail_rejected() {
    // `let x:i32 = if cond { 1 };` 必须报错——无 else 时整体类型必须是 ()
    let r = run("fn main(){ let x:i32 = if 1 == 1 { 1 }; }");
    assert!(
        r.semantic_errors
            .iter()
            .any(|e| e.message.contains("无 `else`") || e.message.contains("尾值必须为")),
        "无 else 的 if 表达式尾值非 () 应报错：{:?}",
        r.semantic_errors
    );
}

#[test]
fn bug_r4n_then_only_if_unit_tail_ok() {
    // 无 else 但 then 是 Unit 应继续合法（不是 BUG 5 范围内的回归点）
    let r = run("fn main(){ if 1 == 1 { let _x:i32 = 1; } }");
    assert!(
        r.semantic_errors.is_empty(),
        "无 else 但 then 是 Unit 仍应合法：{:?}",
        r.semantic_errors
    );
}

#[test]
fn bug_r8_neg_i32_min_literal_accepted() {
    // R8-1：-2147483648 (i32::MIN) 合法；不应被 R4-1 误判溢出
    let r = run("fn main(){ let x:i32 = -2147483648; }");
    assert!(
        r.semantic_errors.is_empty(),
        "i32::MIN 字面量应合法：{:?}",
        r.semantic_errors
    );
}

#[test]
fn bug_r8_neg_i32_min_minus_one_rejected() {
    // R8-1 边界：-2147483649 仍应报溢出
    let r = run("fn main(){ let x:i32 = -2147483649; }");
    assert!(
        r.semantic_errors.iter().any(|e| e.message.contains("超出 i32 范围")),
        "-2147483649 应报溢出：{:?}",
        r.semantic_errors
    );
}

#[test]
fn bug_r6_call_variable_says_not_a_function() {
    // 把变量当函数调用，应说"不是函数"，而不是"未声明"
    let r = run("fn main(){ let a:i32 = 1; a(); }");
    let msgs: Vec<String> = r.semantic_errors.iter().map(|e| e.message.clone()).collect();
    assert!(
        msgs.iter().any(|m| m.contains("`a`") && m.contains("不是函数")),
        "调用变量应报'不是函数'：{:?}",
        msgs
    );
    assert!(
        !msgs.iter().any(|m| m.contains("未声明的函数 `a`")),
        "已声明的变量 a 不应再被报为'未声明的函数'：{:?}",
        msgs
    );
}
