# Easy_Analyzer BUG 总清单（三轮复审汇总）

本文档汇总了三轮 BUG 复审的全部结论：

- **第一轮**（#1-#16）：5 路并行 agent 静态代码追踪 + 21 个 cargo test 印证。
- **第二轮**（R-1~R-6）：穷尽式人工复审 + 36 个基线测试 + `tests/triage.rs`（20 个候选用例）实证。
- **第三轮**（R3-1~R3-7）：新增 `tests/triage_round3.rs` 13 项 + `tests/triage_round3_extra.rs` 13 项做实证探测。

调查范围（共同）：`Easy_Analyzer/src/{semantic.rs, types.rs, symbol.rs, ir.rs}` + `Easy_Parser/src/lib.rs` + `Easy_Server/src/main.rs` + `index.html`。

实证文件：`Easy_Analyzer/tests/triage.rs`、`tests/triage_round3.rs`、`tests/triage_round3_extra.rs`。

---

# 第一轮：#1-#16（已全部修复）

## 🔴 严重（阻塞 IR 可执行 / 强制规则范围） ✅ 已修复

### BUG #1 `BREAK` 四元式不携带目标 label  ✅
- **文件**：`Easy_Analyzer/src/semantic.rs:577`
- **现象**：`self.ir.emit("BREAK", arg, PLACEHOLDER, PLACEHOLDER)` —— 第四元（result）应填循环 end label，实际填的是 `"_"`
- **复现**：`fn main()->i32{ let mut i:i32=0; while i<10 { if i==3 { break; } i=i+1; } return i; }`
- **影响**：任何含 break 的合法程序生成的 IR 不可被解释/翻译为目标代码

### BUG #2 `CONTINUE` 四元式不携带目标 label  ✅
- **文件**：`Easy_Analyzer/src/semantic.rs:589-590`
- **现象**：`self.ir.emit("CONTINUE", PLACEHOLDER, PLACEHOLDER, PLACEHOLDER)` —— 应填回到循环 cond label
- **影响**：同 BUG #1

### BUG #3 `FuncCtx` 没有 loop_stack，无法填写上述 label  ✅
- **文件**：`Easy_Analyzer/src/semantic.rs:54, 214, 1034`
- **现象**：现有 `loop_exprs` 只服务于 `break <expr>`，不存普通 break/continue 的目标
- **建议**：在 `FuncCtx` 加 `loop_labels: Vec<(start: String, end: String)>`，`gen_while/gen_for/gen_loop_expr` 进入时 push、退出时 pop，`gen_break/gen_continue` 读栈顶填到第四元

## 🟠 中等（误报合法代码 / 影响扩展规则） ✅ 已修复

### BUG #4 零长度数组字面量 `let a:[i32;0]=[]` 误报类型不匹配  ✅
- **文件**：`Easy_Analyzer/src/semantic.rs:922-925` + `Easy_Analyzer/src/types.rs:63-72`
- **现象**：`gen_array` 对空 elements 推断为 `Array{Unknown, 0}`；`compatible` 在数组分支递归到 `I32.compatible(Unknown)` 回落到 `==` → false；`is_known()` 仅检查顶层，所以 mismatch 路径触发误报
- **复现**：`fn main(){ let a:[i32;0]=[]; }` → 报 "[i32; 0] 与 [<未知>; 0] 不匹配"
- **修法**：空数组返回 `Array{Error, 0}`（Error 在 compatible 中通配），或 `compatible` 数组分支对 Unknown 元素短路通过

### BUG #5 Server `TokenView.type_enum` 字段没加 serde rename，前端"枚举"列恒为空  ✅
- **文件**：`Easy_Server/src/main.rs:45, 64` + `index.html:375`
- **现象**：后端序列化为 `"type_enum"`，前端读 `t.typeEnum` 得 `undefined` → `esc(undefined)` 输出空字符串
- **复现**：任意源码点"分析" → Token 表"枚举"列空白
- **修法**：给 `type_enum: String` 加 `#[serde(rename = "typeEnum")]`，或前端改读 `t.type_enum`

### BUG #6 函数名作实参时错误信息文不对题（PDF 例 program_3_3__4）  ✅
- **文件**：`Easy_Analyzer/src/semantic.rs::gen_identifier` (~L649) + `gen_call`
- **现象**：PDF 期望"实参类型与形参类型不一致"；实际报"变量 program_3_3_4_a 未声明"（因 `gen_identifier` 在变量符号表查不到函数名）
- **影响**：语义上仍报错 → 测试 `assert_err` 通过，但与 PDF 描述不符，扣分点
- **修法**：`gen_identifier` 找不到变量时回退 `lookup_function`，返回 `Type::Function` 占位让 `gen_call` 比对

## 🟡 轻度（IR 不优雅 / 错误恢复瑕疵） ✅ 已修复

### BUG #7 重复函数定义后仍生成两份 FUNC IR  ✅
- **文件**：`Easy_Analyzer/src/semantic.rs:144-154`（无去重跳过）
- **现象**：`fn f(){} fn f(){}` 报"重复定义"正确，但 IR 含两份 `FUNC f`…`END_FUNC f`

### BUG #8 带返回类型的函数若无任何 return，IR 缺终结子  ✅
- **文件**：`Easy_Analyzer/src/semantic.rs:222-237`（仅 Unit 返回类型才 emit 隐式 RETURN）
- **现象**：`fn f()->i32{ }` 的 IR 没有 RETURN，下游可能误判 fall-through

### BUG #9 `if` 表达式两分支均为 Unit 时仍分配未写入的 temp  ✅
- **文件**：`Easy_Analyzer/src/semantic.rs:991-1024` (`gen_if_expr`)
- **现象**：无条件 `new_temp()`，但 Unit 分支不 emit `=` → temp 悬空

### BUG #10 `loop { ... }` 的 end label 不可达  ✅（由 BUG #1 修复带动：BREAK 现在携带 end label，end 通过 BREAK 可达）
- **文件**：`Easy_Analyzer/src/semantic.rs:1027-1049` (`gen_loop_expr`)
- **现象**：只发 `LABEL start … GOTO start; LABEL end`，end 仅靠 break 才到（叠加 BUG #1 → 死循环）

### BUG #11 块表达式作用域内"类型无法推断"漏报  ✅
- **文件**：`Easy_Analyzer/src/semantic.rs:959-971` (`gen_block_expr` 未调 `check_current_scope_uninferred`)
- **现象**：`let x = { let y; 1 };` 不报 y 无法推断

### BUG #12 类型不一致的 `break <expr>` 仍发射 `=` 赋值 IR  ✅
- **文件**：`Easy_Analyzer/src/semantic.rs:563-572`
- **现象**：报错后照常 emit `(=, "()", _, tN)`

## 🔵 措辞 / 信息缺失 ✅ 已修复

### BUG #13 数组越界错误未带数组名  ✅
- **文件**：`Easy_Analyzer/src/semantic.rs:851-853`
- **现象**：`数组下标 5 越界，合法范围 [0,3)` 缺变量名

### BUG #14 "调用未声明的函数"错误未带规则号  ✅
- **文件**：`Easy_Analyzer/src/semantic.rs:767`
- **现象**：对比其他错误都带"（规则 X.X）"

### BUG #15 Server 无 CORS，file:// 打开 index.html 不工作  ✅
- **文件**：`Easy_Server/src/main.rs:152-154`
- **现象**：同源访问 OK；跨源/本地双击 html 失败
- **严重度**：取决于使用方式

### BUG #16 词法错误时跳过 parser，前端无原因提示  ✅
- **文件**：`Easy_Server/src/main.rs:79`
- **现象**："AST 不可用"无解释

## ✅ 第一轮确认正确处理

**28 条 PDF 语义检查**：1.5/2.1/2.2/2.3/3.5/5.2/5.4/6.1/6.3/6.4/7.4/8.2/8.3/9.2/9.3 全部在源码层正确实现，`requirements_coverage.rs` + `smoke.rs` 21 个测试印证。

**算术/比较 IR**：操作数顺序（`a-b → (-, a, b, t)`）、IF_FALSE 跳 else/end、while cond/end 标签结构、`=` 方向、CALL 返回值写 result、PARAM FIFO、临时变量单调不复用 —— 全部正确。

**边界**：自递归、互递归、shadowing 链（含类型变化）、深嵌套 if/while、空函数/空语句、多错误恢复、条件中的复杂表达式与函数调用、多个不可变引用、嵌套元组/数组、数组静态越界、未声明函数调用 —— 全部行为符合预期。

**Server**：JSON 字段 `lexerErrors/parseError/semanticErrors/quadruples` 与前端期望一致（仅 `typeEnum` 除外），quadruple 子字段 `index/op/arg1/arg2/result` 命名一致，Vec 空时返回 `[]` 而非 `null`，监听 `127.0.0.1:3000` 限本机。

**未发现可达 panic**：全文 `unwrap/expect` 仅 `semantic.rs:189` 一处，被"先全部 declare 再 analyze"的顺序保护。

---

# 第二轮：R-1~R-6（已全部修复）

## 🟠 中等（误判合法代码 / 漏报非法代码） ✅ 已修复

### BUG R-1 数组下标 `-1` 等负字面量漏报静态越界  ✅
- **文件**：`Easy_Analyzer/src/semantic.rs:897-910`
- **现象**：`gen_index` 的静态越界检查仅匹配 `Expr::Number { value }`；但解析器把 `-1` 解析为 `Expr::Unary { op: Neg, expr: Expr::Number{"1"} }`，因此整条 if 不成立，未触发越界判断。
- **复现**（triage::cand_a_negative_literal_index_should_be_oob 已 FAIL 印证）：
  ```rust
  fn main(){ let a:[i32;3]=[1,2,3]; let b:i32=a[-1]; }
  ```
- **修法**：在静态越界分支增加对 `Expr::Unary { op: UnaryOp::Neg, expr }` 的处理。

### BUG R-2 超大正字面量下标因 `isize` 溢出被静默跳过  ✅
- **文件**：`Easy_Analyzer/src/semantic.rs:898-909`
- **现象**：`value.parse::<isize>()` 失败时 `if let Ok(n) = ...` 短路；明显越界的极大字面量（>isize::MAX）逃过越界检查。
- **复现**（triage::cand_s_overflow_index_skipped 已 FAIL 印证）：
  ```rust
  fn main(){ let a:[i32;3]=[1,2,3]; let b:i32=a[99999999999999999999]; }
  ```
- **修法**：用 `u128` 解析；解析失败一律视为越界。

### BUG R-3 重名形参 `fn f(a:i32, a:i32)` 静默接受  ✅
- **文件**：`Easy_Analyzer/src/semantic.rs:171-190`（`build_function_sig`）+ `:216-223`（`declare`）
- **现象**：`build_function_sig` 把所有形参原样收集；后续 `declare` 覆盖前者；形参重名未报错，且 IR 中发射两条 `PARAM_DECL`。
- **复现**（triage::cand_r_duplicate_param_emits_two_decls_and_no_error 已 FAIL 印证）：
  ```rust
  fn f(a:i32, a:i32){} fn main(){}
  ```
- **修法**：在 `analyze_function` 头部用 `HashSet` 扫描重复。

## 🟡 轻度（信息丢失 / 设计不严谨） ✅ 已修复

### BUG R-4 `for` 循环变量的显式类型注解被静默丢弃  ✅
- **文件**：`Easy_Analyzer/src/semantic.rs:530-537`
- **现象**：`for mut i:T in iter {}` 中 T 不被采用也不被报错。
- **修法**：在 `gen_for` 中校验 `binding.ty` 与 `start_ty` 一致性。

## 🔵 措辞 / 信息瑕疵 ✅ 已修复

### BUG R-5 错误信息中暴露内部占位 `<类型错误>` / `<函数>`  ✅
- **文件**：`Easy_Analyzer/src/types.rs:103, 106`（`Type::display`）
- **现象**：用户能在错误消息里看到 `<类型错误>` / `<函数>` 这类只面向开发者的占位字符串。
- **修法**：在 `gen_let` 数组分支特判长度不匹配，函数名作 RValue 给专门报错。

### BUG R-6 调用变量名报"调用未声明的函数 `a`"措辞失真  ✅
- **文件**：`Easy_Analyzer/src/semantic.rs:815-817`（`gen_call`）
- **现象**：`a` 是已声明的变量被当函数调用时报"未声明"，给读者错觉 `a` 不存在。
- **修法**：在 `gen_call` 中先 `lookup` 一次，按变量 vs 函数区分错误措辞。

## ✅ 第二轮复审通过的范围

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
| `Type::Error` 在 compatible 中通配，避免连锁 | types.rs:60 + 多个 triage 用例 | ✅ |

### D. panic / unwrap 风险
全文 `unwrap / expect / 下标` 复审结果：
- `semantic.rs:204` `expect("函数签名应已登记")`：由 `analyze_program` 先全部 `declare_function`、再 `analyze_function` 顺序保护，**不可触达**。
- `semantic.rs:158, 164` 下标 `skip[i]`：与 `program.functions` 同长 `Vec`，索引来自同一 `enumerate()`，**安全**。
- `semantic.rs:939` `elements[idx].clone()`：上面 `if idx >= elements.len()` 已 guard，**安全**。
- 各处 `unwrap_or / unwrap_or_else / unwrap_or_default`：均为带默认值版本，**不会 panic**。
- 未发现可由输入触发的 panic 路径。

### E. Server JSON 字段契约
| Server 输出 | 前端读取 | 一致性 |
|---|---|---|
| `lexerErrors`（已 rename） | `data.lexerErrors` | ✅ |
| `parseError`（已 rename） | `data.parseError` | ✅ |
| `semanticErrors`（已 rename） | `data.semanticErrors` | ✅ |
| `quadruples`（默认 snake） | `data.quadruples` | ✅ |
| Token: `type` (rename), `typeEnum` (rename) | `t.type`, `t.typeEnum` | ✅（BUG #5 已修） |
| Quadruple: `index/op/arg1/arg2/result` | `q.index/op/arg1/arg2/result` | ✅ |
| CORS：`allow_origin(Any)` | file:// 与跨源可用 | ✅（BUG #15 已修） |

### F. 跨 crate 数据流（AST variant 覆盖）
对照 `easy_parser::lib.rs`：`Statement` 9 个 variant、`Expr` 12 个 variant、`ElseBranch` 3 个 variant、`TypeNode` 5 个 variant、`UnaryOp` 4 个、`BinaryOp` 11 个 —— 全部在 `semantic.rs` 中显式 match 覆盖，无 `_ =>` 通配吞错。

---

# 第三轮：R3-1~R3-7

基线：bug_report 中 #1-#16 + R-1~R-6 已全部修复，本轮均不重复。

## 🔴 严重（数据流错乱） — 1 条

### BUG R3-1 嵌套循环中 `break <expr>;` 把值写入错误的 result_place

- **位置**：`Easy_Analyzer/src/semantic.rs:752-779`（`gen_break`）
- **根因**：`gen_break` 写入 result_place 取自 `func_ctx.loop_exprs.last_mut()`，跳转目标 end_label 取自 `func_ctx.loop_labels.last()`。两个栈深度不同步：
  - `gen_loop_expr` 同时 push `loop_exprs` + `loop_labels`；
  - `gen_for` / `gen_while` 只 push `loop_labels`。
  - 内层 for/while 嵌入外层 `loop {}` 时，`loop_labels.last()` 指向内层、`loop_exprs.last()` 指向外层 → 写错位置。
- **最小复现**（`triage_round3::cand_u_break_value_in_nested_for_writes_outer_loop_result`）：
  ```rust
  fn main(){
      let x = loop {
          for i in 0..3 { break 42; }   // 写入 t1 + 跳 for 的 end label
          break 7;                       // 同样写入 t1 + 跳 loop 的 end label
      };
  }
  ```
  实证 IR：
  ```
  6: = 42 _ t1     <-- 内层 for 的 break 42 写入了外层 loop 的 result_place
  7: BREAK 42 _ L5  <-- 但跳的是 for 的 end
  13: = 7 _ t1
  14: BREAK 7 _ L2
  ```
  `42` 和 `7` 写入同一 `t1`，数据流与控制流不再一一对应。
- **修法**：在 `LoopLabels` 上加 `loop_expr_index: Option<usize>`，`for/while` 推 `None`，`loop {}` 推 `Some(loop_exprs.len()-1)`；`gen_break` 取 result_place 与 end_label 都通过同一个 LoopLabels 保证一致。`value.is_some()` 但当前循环是 for/while 时，直接报错"`break <expr>` 仅在 `loop` 表达式中允许"。

## 🟠 中度（漏检 / 误判合法代码） — 4 条

### BUG R3-2 `return ();` 在 Unit 函数中被错误地拒绝

- **位置**：`Easy_Analyzer/src/semantic.rs:565-580`（`gen_return` 的 `Some(expr)` 分支）
- **现象**：当函数声明返回类型为 Unit 时，只要 `return` 后面带任何表达式（即使该表达式类型也是 Unit）都被直接报"函数无返回类型，return 不能带表达式（规则 1.5）"。
- **最小复现**：
  - `triage_round3_extra::cand_hh_return_unit_literal_in_unit_function`：`fn main(){ return (); }` → 报错
  - `triage_round3_extra::cand_ii_return_block_unit_in_unit_function`：`fn main(){ return {}; }` → 报错
- **PDF 期望**（1.5）：`return expr;` 的类型应与函数声明返回类型一致。`()` 与 `{}` 的类型均为 Unit，与 Unit 兼容，理应被接受。
- **修法**：把 `if matches!(expected, Type::Unit)` 这一硬性短路改为统一的兼容性检查。

### BUG R3-3 函数声明返回非 Unit 但函数体无 `return`/无尾表达式时漏报

- **位置**：`Easy_Analyzer/src/semantic.rs:265-272`（`analyze_function` 末尾兜底 RETURN）
- **现象**：函数返回类型为 `i32` 但函数体既无 `return` 语句也无尾表达式时，当前实现只发射兜底 `RETURN _ _ _`（Unit），不报任何错。
- **最小复现**（`triage_round3::cand_bb_non_unit_function_falls_through_without_warning`）：
  ```rust
  fn main()->i32 { let a:i32 = 1; }   // 无任何错误
  ```
- **PDF 期望**（1.5）：函数若有 `-> T`，则必须保证有返回 T 的路径（或至少静态地存在一条 `return expr;` 路径）。
- **修法**：若函数体 `tail.is_none()` 且 return_type 非 Unit，至少检查函数体最后一条语句是否为 `Statement::Return`；否则报错。

### BUG R3-4 `loop` 中无值 `break;` 与带值 `break expr;` 混用时类型推断不一致漏检

- **位置**：`Easy_Analyzer/src/semantic.rs:742-790`（`gen_break`）
- **现象**：`gen_break` 仅在 `value.is_some()` 时更新 `loop_exprs.break_type`。无值 `break;` 完全不更新；这使得既出现无值 break、又出现带值 break 的 loop 表达式类型推断不一致问题被吞掉。
- **最小复现**（`triage_round3_extra::cand_rr_loop_with_mixed_break_kinds`）：
  ```rust
  fn main(){
      let x:i32 = loop {
          if 1 == 1 { break; }   // 无值 break：意味着 loop 类型应是 ()
          break 42;               // 有值 break：要求 loop 类型是 i32
      };
  }
  ```
- **修法**：把无值 break 也当作 `break_type = Unit` 写入，同样走类型一致性检查。

### BUG R3-5 同名 `fn` 与 `let` 共存时调用/取值解析二义性，且无报错

- **位置**：`Easy_Analyzer/src/symbol.rs`（函数表与变量表分立）+ `semantic.rs:896-921`（`gen_identifier`）+ `semantic.rs:1024-1048`（`gen_call`）
- **现象**：同一个标识符可以同时存在于函数表与变量表，且不报告冲突。结果：
  - 当 `x` 作 RValue 使用（`let y = x`），优先解析为变量；
  - 当 `x` 作被调用 `x(...)`，优先解析为函数。
  - 两种解析在同一作用域内表现不同。
- **最小复现**（`triage_round3_extra::cand_tt_function_and_var_same_name`）：
  ```rust
  fn x() -> i32 { 1 }
  fn main(){ let x:i32 = 2; let y:i32 = x(); }
  ```
- **修法**：在 `gen_let` 中检查是否与函数表同名，报告"标识符 `x` 与现有函数同名，可能导致调用/取值二义性"。

## 🟡 轻度（错误恢复 / 冗余 IR） — 2 条（详见 `discussions.md`）

### BUG R3-6 显式 `return` 后函数体末仍发射兜底 `RETURN _ _ _`

- 设计选择：保留终结子便于下游解释器统一处理。讨论见 `discussions.md`。

### BUG R3-7 静态越界报错后仍发射 `INDEX` 四元式

- 设计选择：错误恢复保留 IR 形态，便于即便有语义错也能生成完整的四元式序列以便观察。讨论见 `discussions.md`。

## ✅ 第三轮重新验证通过的范围

| 项目 | 测试 | 结果 |
|---|---|---|
| 递归调用 / 前向函数引用 `fact(n-1)` | cand_cc | ✅ |
| else-if 链式 if 表达式 | cand_dd | ✅ |
| 函数名出现在比较运算符两侧 | cand_ee | ✅ 专门提示"请加 `()` 调用" |
| 数组下标是 bool / 非 i32 | cand_ff | ✅ 报"下标类型 bool" |
| 函数返回数组 + 作为实参 | cand_gg | ✅ |
| `fn main() { () }` 尾 Unit 字面量 | cand_jj | ✅ |
| `let r = 0..3;` parser 拒绝（设计选择） | cand_kk | parser 错误，符合预期 |
| 动态 i32 下标不报"越界" | cand_ll | ✅ |
| 数组元素类型 `&i32` | cand_mm | ✅ |
| 元组中混合函数名 + i32 | cand_nn | ✅（宽松） |
| shadowing + mut 重新可赋值 | cand_oo | ✅ |
| `let y:i32 = &x;` 应报类型不匹配 | cand_pp | ✅ |
| `for i in 0..end()` 调用作 range 终点 | cand_qq | ✅ |
| Unit 函数内 `return 1;` 仍报"不能带表达式" | cand_ss | ✅ |
| `let x:i32 = loop { break; }` 报类型不匹配 | cand_w | ✅ |
| 越界 OOB 元组下标超大数字不 panic | cand_z | ✅ |
| for 非范围迭代器错误恢复不污染状态 | cand_aa | ✅ |

---

# 修复优先级

1. **必修**（影响 IR 正确性 / 强制规则交付）：#1, #2, #3 已修；R3-1（嵌套 break 窜流）应修。
2. **应修**（影响合法代码 / 易扣分）：#4, #5, #6 已修；R-1, R-2, R-3 已修；R3-2（合法 `return ();` 被拒）应修。
3. **建议修**（漏检）：R3-3, R3-4, R3-5 —— 取决于老师对静态检查严格度的要求。
4. **可修**（润色 IR / 错误信息）：#7-#16 已修；R-4, R-5, R-6 已修。
5. **可不修**（设计选择，详见 `discussions.md`）：R3-6（冗余 RETURN）、R3-7（OOB 后保留 INDEX）、R4-2（常量折叠下标越界）、R4-3（函数名绑定到变量）。

---

# 第四轮：R4-1（已修复）/ R4-2 / R4-3

基线：bug_report 中 #1-#16、R-1~R-6、R3-1~R3-5 已全部修复，R3-6 / R3-7 进入 `discussions.md` 作为设计选择。

本轮针对前三轮未覆盖的字面量取值范围 / 常量折叠 / 函数项绑定三个方向做穷尽式复审，发现 3 条候选。

## 🟠 中度（漏检合法/非法边界） — 1 条 ✅ 已修复

### BUG R4-1 i32 字面量超范围未做静态检查  ✅

- **位置**：`Easy_Analyzer/src/semantic.rs:1090`（`gen_expr` 的 `Expr::Number` 分支）
- **现象**：当前实现把任意 `Expr::Number { value }` 字面量原样塞进 `ExprValue.place`，从不校验 `value` 是否在 i32 范围内；语义阶段对超过 `i32::MAX` 的数字字面量完全沉默。
- **最小复现**：
  ```rust
  fn main(){ let a:i32 = 9999999999999999999; }   // 修复前：无任何错误
  ```
- **PDF 期望**（规则 0.1）：整数字面量应当在 i32 范围内；否则报错。
- **与 R-2 同源**：R-2 已经把数组下标的 `parse::<isize>()` 改为 `parse::<u128>()` 且失败即视为越界，本条是对"数字字面量本身"做同等粒度的检查。
- **修法**：在 `gen_expr` 的 `Expr::Number` 分支用 `value.parse::<i32>().is_err()` 做范围判定，失败时 `self.error(...)` 报"整数字面量 `{value}` 超出 i32 范围（规则 0.1）"，仍按 `Type::I32` 返回 `ExprValue` 以保持错误恢复时 IR 形态稳定。
- **验证**：临时复现源码 `fn main(){ let a:i32 = 9999999999999999999; }` 修复后正确报错；除 `cand_kk_range_value_stored_and_iterated`（parser 限制，原本就 FAIL）外全部 126 个测试通过。

## 🔵 设计选择（保留现状，详见 `discussions.md`） — 2 条

### BUG R4-2 数组下标静态越界检查不识别常量算术表达式

- **位置**：`Easy_Analyzer/src/semantic.rs:1045-1059`（`check_array_static_oob`）
- **现象**：当前只识别 `Expr::Number` 与 `Expr::Unary{Neg, Number}` 两种字面量形态。`a[0-1]` / `a[2+1]` 等常量算术表达式不触发静态越界报错，由运行时 bounds check 兜底。
- **与 R-1 / R-2 同源**：那两条已经修了字面量类越界，本条是"字面量算术折叠"层次的进一步推广。
- **设计选择**：保留不修。PDF 规则 8.3 未要求常量折叠；引入 `eval_const` 小型求值器的工作量与教学收益不成正比；运行时 bounds check 已经能拦住非法访问。详见 `discussions.md` 的 **D-6**。

### BUG R4-3 `let g = f;` 允许把函数名绑定到变量、类型为 `Type::Function`

- **位置**：`Easy_Analyzer/src/semantic.rs:341-375`（`gen_let` 的 `(None, expr_ty)` 推断分支）
- **现象**：
  ```rust
  fn f() -> i32 { 1 }
  fn main(){
      let g = f;           // 不报错；g 进入变量表，类型为 Type::Function
      let y:i32 = g;       // 报"变量 `y` 用函数 `g` 作为初始化表达式"
  }
  ```
  错误延迟到"使用点"才出现，且消息把已是变量的 `g` 仍称作"函数"，与 R3-5 同名提示风格不一致。
- **设计选择**：保留不修。当前实现错误能报、IR 形态稳定（`= f _ g` 形态对下游友好），仅措辞略失真；改严格的成本主要在错误信息测试预期的整体重新对齐。详见 `discussions.md` 的 **D-7**。
