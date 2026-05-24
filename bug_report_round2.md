# Easy_Analyzer BUG 清单（第二轮复审）

调查方式：穷尽式人工 + 36 个基线测试印证 + 新增 `Easy_Analyzer/tests/triage.rs`（20 个候选用例）实证验证。
调查范围：`Easy_Analyzer/src/{semantic.rs, types.rs, symbol.rs, ir.rs}` + `Easy_Server/src/main.rs` + `Easy_Parser/src/lib.rs` + `index.html`。
基线：bug_report.md 中 #1-#16 已全部修复，本轮均不重复。

实证文件：`Easy_Analyzer/tests/triage.rs`（保留 3 个 FAILED + 17 个 OK 作为后续修复回归基线）。

---

## 🟠 中等（误判合法代码 / 漏报非法代码） ✅ 已修复

### BUG R-1 数组下标 `-1` 等负字面量漏报静态越界  ✅
- **文件**：`Easy_Analyzer/src/semantic.rs:897-910`
- **现象**：`gen_index` 的静态越界检查仅匹配 `Expr::Number { value }`；但解析器把 `-1` 解析为 `Expr::Unary { op: Neg, expr: Expr::Number{"1"} }`，因此整条 if 不成立，未触发越界判断。
- **复现**（triage::cand_a_negative_literal_index_should_be_oob 已 FAIL 印证）：
  ```rust
  fn main(){ let a:[i32;3]=[1,2,3]; let b:i32=a[-1]; }
  ```
  当前：无任何语义错误。
- **PDF 期望**（8.3）：下标须在 `[0, len)` 内；`-1` 越界，应报错。
- **根因**：模式匹配只覆盖正字面量分支，未识别 `Unary{Neg, Number}` 形态。
- **修法建议**：在静态越界分支增加对 `Expr::Unary { op: UnaryOp::Neg, expr }` 的处理：
  ```rust
  let lit = match index {
      Expr::Number { value } => value.parse::<isize>().ok(),
      Expr::Unary { op: UnaryOp::Neg, expr } => {
          if let Expr::Number { value } = &**expr {
              value.parse::<isize>().ok().map(|n| -n)
          } else { None }
      }
      _ => None,
  };
  ```

### BUG R-2 超大正字面量下标因 `isize` 溢出被静默跳过  ✅
- **文件**：`Easy_Analyzer/src/semantic.rs:898-909`
- **现象**：`value.parse::<isize>()` 失败时 `if let Ok(n) = ...` 短路；明显越界的极大字面量（>isize::MAX）逃过越界检查。
- **复现**（triage::cand_s_overflow_index_skipped 已 FAIL 印证）：
  ```rust
  fn main(){ let a:[i32;3]=[1,2,3]; let b:i32=a[99999999999999999999]; }
  ```
  当前：无任何语义错误。
- **PDF 期望**（8.3）：合法范围 `[0, 3)`；`99999999999999999999` 显然越界。
- **根因**：用 `isize` 做范围比较，没有 fall-through 到"既然解析失败就一定越界（无论符号都越界）"的判断。
- **修法建议**：
  ```rust
  let parsed = value.parse::<u128>().ok(); // 容纳更大正值
  let n = parsed.map(|n| n as i128).or_else(|| value.parse::<i128>().ok());
  if n.map(|n| n < 0 || n as u128 >= len as u128).unwrap_or(true /* 解析失败=过大=越界 */) { 报错 }
  ```
  或更简单：只要 `parse::<usize>()` 失败且 `value` 没有 `-` 前缀，就视为越界。

### BUG R-3 重名形参 `fn f(a:i32, a:i32)` 静默接受  ✅
- **文件**：`Easy_Analyzer/src/semantic.rs:171-190`（`build_function_sig`）+ `:216-223`（`declare`）
- **现象**：`build_function_sig` 把所有形参原样收集到 `sig.params`；`analyze_function` 调 `self.table.declare(VarSymbol::new(pname, ...))` 顺序写入符号表，后者覆盖前者（同 scope shadowing）。形参重名未报错，且 IR 中发射两条 `PARAM_DECL a, i32, _`。
- **复现**（triage::cand_r_duplicate_param_emits_two_decls_and_no_error 已 FAIL 印证）：
  ```rust
  fn f(a:i32, a:i32){} fn main(){}
  ```
  当前：无错；IR 含两条 `(PARAM_DECL, a, i32, _)`；符号表只有"后一个 a"。
- **PDF 期望**：未直接列出，但属于明显的形参列表合法性问题（与 rule 1.4 一致：形参应能唯一识别），且对调用方/IR 都会引起歧义。
- **根因**：未在 `build_function_sig` 或 `analyze_function` 中做形参名唯一性检查。
- **修法建议**：在 `analyze_function` 头部用 `HashSet` 扫描重复：
  ```rust
  let mut seen = std::collections::HashSet::new();
  for p in &f.params {
      if !seen.insert(&p.name) {
          self.error(format!("函数 `{}` 形参 `{}` 重名", f.name, p.name));
      }
  }
  ```

---

## 🟡 轻度（信息丢失 / 设计不严谨）

### BUG R-4 `for` 循环变量的显式类型注解被静默丢弃
- **文件**：`Easy_Analyzer/src/semantic.rs:530-537`
- **现象**：解析器允许 `for mut i:T in iter {}`（`parse_binding(false)` 接收可选 `:T`），但 `gen_for` 不读 `binding.ty`，直接用 `start_ty`（range → 强制 I32）登记符号。用户给出的不匹配类型注解（如 `for i:[i32;3] in 0..3`）既不被采用也不被报错。
- **复现**（triage::cand_q_for_binding_explicit_type_silently_ignored 中通过下游 `let _x:[i32;3]=i` 触发"i32 与 [i32;3] 不匹配"，间接观察到 i 被当作 i32）：
  ```rust
  fn main(){ for mut i:[i32;3] in 0..3 { let _x:[i32;3]=i; } }
  ```
  实际错误指向 `_x`，而真正错误是用户对 `i` 写了与 range 冲突的类型注解。
- **PDF 期望**：PDF 未直接给出此例，但属于"用户写了的语义信息被静默吞"的瑕疵。
- **修法建议**：在 `gen_for` 形参注解非空且不兼容 `start_ty` 时报错；否则保留原有 `start_ty` 行为：
  ```rust
  if let Some(node) = &binding.ty {
      let declared = from_node(node);
      if declared.is_known() && !declared.compatible(&start_ty) {
          self.error(format!("for 循环变量 `{}` 注解类型 {} 与迭代起点类型 {} 不一致",
              binding.name, declared.display(), start_ty.display()));
      }
  }
  ```

---

## 🔵 措辞 / 信息瑕疵

### BUG R-5 错误信息中暴露内部占位 `<类型错误>` / `<函数>`
- **文件**：`Easy_Analyzer/src/types.rs:103, 106`（`Type::display`）
- **现象**：用户能在错误消息里看到 `<类型错误>` / `<函数>` 这类只面向开发者的占位字符串。
  - `let a:[i32;2]=[];` → "声明类型 [i32; 2] 与初始化表达式类型 **[<类型错误>; 0]** 不匹配"
  - `fn g(){} fn main(){ let a:i32 = g; }` → "声明类型 i32 与初始化表达式类型 **<函数>** 不匹配"
- **复现**：triage::cand_m_empty_array_lit_mismatched_length_msg、cand_n_function_as_plain_rvalue（均通过，但 println 暴露文本）。
- **修法建议**：
  - 空数组场景：在 `gen_let` 数组分支特判 `Type::Array { length: 0, .. } vs Type::Array { length: n>0, .. }` 直接给"数组长度不匹配（0 vs n）"。
  - 函数名作 RValue：`Type::Function` 转义为"函数引用（不可作 RValue）"或在 `gen_let/gen_assign` 上游识别 `expr_ty == Function` 后给专门报错。

### BUG R-6 调用变量名报"调用未声明的函数 `a`"措辞失真
- **文件**：`Easy_Analyzer/src/semantic.rs:815-817`（`gen_call`）
- **现象**：`a` 是已声明的变量，被当函数调用时报"调用未声明的函数 `a`（规则 3.5）"，给读者错觉 `a` 不存在。
- **复现**（triage::cand_i_call_a_variable，已通过但暴露措辞问题）：
  ```rust
  fn main(){ let a:i32 = 1; a(); }
  ```
- **修法建议**：在 `gen_call` 中先 `lookup` 一次 — 如果作为变量存在，给出"变量 `a` 不是函数，不能被调用（规则 3.5）"；否则保留现"未声明"措辞。

---

## ✅ 复审通过的范围（用证据，不只口号）

下列方向各跑过实证测试且均符合预期（具体见 `Easy_Analyzer/tests/triage.rs` 与既有 36 个测试）：

### A. PDF 强制语义检查的边界
| 检查 | 测试 | 结果 |
|---|---|---|
| 嵌套块表达式内部 `let mut z;` 类型推断失败 | triage::cand_t_nested_block_expr_uninferred_inner | ✅ 报 "z 无法推断类型" |
| if 分支类型不一致（i32 vs 单分支 unit） | triage::cand_l_if_branches_mixed_unit_value | ✅ 报 "分支类型不一致" |
| 类型 RValue 推断: 函数名作 RHS（`let a:i32=g`） | triage::cand_n_function_as_plain_rvalue | ✅ 报"类型不匹配"（措辞另见 R-5） |
| 数组字面量 `[i32;2]=[]` 长度不匹配仍报错 | bug_fixes::bug4_empty_array_still_rejects_mismatched_length + triage::cand_m | ✅ |
| 借用作用域：内部块 `&mut a` 弹栈后，外部 `&a` 合法 | triage::cand_k_borrow_scope_drop_on_block_exit | ✅ 朴素借用语义生效 |
| 未初始化 immutable + 二次赋值 | triage::cand_p_uninit_then_two_assigns_immutable_rejected | ✅ 报 "不可变变量 不能再次赋值" |

### B. 四元式 IR 的语义正确性（解释器实证）
| 程序 | 测试 | 结果 |
|---|---|---|
| `for i in 0..5 { s = s+i; }` 求和 | triage::cand_b_for_loop_sums_correctly | ✅ 返回 10 |
| 嵌套 while + continue + break，外层 3 次 × 每次内层 1+2+4 | triage::cand_c_nested_while_continue_and_break | ✅ 返回 21 |
| 嵌套 while + continue 仅影响最内层 | triage::cand_o_nested_loop_continue_only_inner | ✅ 返回 6 |
| `loop { break 42; }` 写入 loop 结果 temp 再赋给变量 | triage::cand_f_loop_break_with_value_writes_result_temp | ✅ 两条 = 链路完整 |
| `f(g(), h())` 嵌套 CALL → 顺序为 CALL g, CALL h, PARAM tg, PARAM th, CALL f | triage::cand_d_nested_call_param_order | ✅ |
| 含 `1+2+3+4`、`(1+2)*(3+4)`、if、while 的程序：临时变量与 LABEL 编号唯一 | triage::cand_e_temp_and_label_uniqueness | ✅ |
| BREAK/CONTINUE 四元式始终携带目标 label | ir_interpreter::break_and_continue_carry_target_label | ✅（BUG #1/#2 修复回归） |

### C. 错误恢复与多错并发
| 程序 | 测试 | 结果 |
|---|---|---|
| 一段含 3 类错误（类型不匹配/算术非 i32/未声明），不出现连锁污染 | triage::cand_j_multiple_errors_no_cascade | ✅ 3 条错误齐全、无重复 |
| `let mut a:i32 = 1==1; let b:i32 = a + (1==1);` —— `a` 后续仍可作 i32 使用，不被前错污染 | 同上 | ✅ |
| `Type::Error` 在 compatible 中通配，避免连锁 | types.rs:60 + 多个 triage 用例 | ✅ |

### D. panic / unwrap 风险
全文 `unwrap / expect / 下标` 复审结果：
- `semantic.rs:204` `expect("函数签名应已登记")`：由 `analyze_program` 先全部 `declare_function`、再 `analyze_function` 顺序保护，且跳过重复函数，路径已封闭，**不可触达**。
- `semantic.rs:158, 164` 下标 `skip[i]`：与 `program.functions` 同长 `Vec`，索引来自同一 `enumerate()`，**安全**。
- `semantic.rs:939` `elements[idx].clone()`：上面 `if idx >= elements.len()` 已 guard，**安全**。
- 各处 `unwrap_or / unwrap_or_else / unwrap_or_default`：均为带默认值版本，**不会 panic**。
- 未发现可由输入触发的 panic 路径。

### E. Server JSON 字段契约
| Server 输出（snake/camel） | 前端读取 | 一致性 |
|---|---|---|
| `lexerErrors`（已 rename） | `data.lexerErrors` | ✅ |
| `parseError`（已 rename） | `data.parseError` | ✅ |
| `semanticErrors`（已 rename） | `data.semanticErrors` | ✅ |
| `quadruples`（默认 snake） | `data.quadruples` | ✅ |
| `tokens` | `data.tokens` | ✅ |
| `ast` | `data.ast` | ✅ |
| Token: `type` (rename), `typeEnum` (rename) | `t.type`, `t.typeEnum` | ✅（BUG #5 已修） |
| Token: `line`, `col`, `value` | `t.line`, `t.col`, `t.value` | ✅ |
| Quadruple: `index/op/arg1/arg2/result` | `q.index/op/arg1/arg2/result` | ✅ |
| 三种状态（词法错 / 解析错 / 语义错）下前端各字段均有自洽兜底（默认 `[]`/`null`） | index.html:323-332 已 `|| []` 防御 | ✅ |
| CORS：`allow_origin(Any)` | file:// 与跨源可用 | ✅（BUG #15 已修） |

### F. 跨 crate 数据流（AST variant 覆盖）
对照 `easy_parser::lib.rs`：

`Statement` 9 个 variant、`Expr` 12 个 variant、`ElseBranch` 3 个 variant、`TypeNode` 5 个 variant、`UnaryOp` 4 个、`BinaryOp` 11 个 —— 全部在 `semantic.rs` 中显式 match 覆盖，无 `_ =>` 通配吞错：
- `gen_stmt` 显式 match 全 9 个 `Statement` variant（semantic.rs:269-281）。
- `gen_expr` 显式 match 全 12 个 `Expr` variant（semantic.rs:676-691）。
- `gen_unary` 显式 match 全 4 个 `UnaryOp`（semantic.rs:723-778）。
- `gen_binary` 通过 `bin_op_info` 显式 match 全 11 个 `BinaryOp`（semantic.rs:1135-1149）。
- `gen_if_expr` 显式 match 全 3 个 `ElseBranch`（semantic.rs:1057-1061）。
- `from_node` 显式 match 全 5 个 `TypeNode`（types.rs:113-131）。

未发现"语义分析器漏处理 AST 形态"的问题。

---

## 修复优先级

1. **应修**（关系 PDF 8.3 静态检查完整性）：R-1, R-2 —— 一条 `match` 扩展即可
2. **应修**（影响 IR/符号表健康性）：R-3 —— 在函数入口加重名扫描
3. **可修**（用户体验）：R-4, R-5, R-6 —— 错误信息打磨与少量语义补全

修复时建议同时将 `Easy_Analyzer/tests/triage.rs` 中三个 FAILED 用例升级为正式回归（`bug_fixes.rs` 续编号），并把 `cand_q/m/n/i` 的 println 改为强 assert。

---

## 第二轮结论

- **未发现新的 🔴 严重缺陷**（无可触达 panic、无强制规则系统性漏检、无 IR 不可执行问题）。
- 新发现 **3 个 🟠 中等 + 1 个 🟡 轻度 + 2 个 🔵 措辞** 共 6 条新缺陷，全部已用 `tests/triage.rs` 提供最小复现，可直接作为下一轮修复输入。
- 项目主干（PDF 强制规则 + IR 生成 + Server 契约 + 跨 crate 流转）经实证测试稳定。
