# Easy_Analyzer BUG 清单

调查方式：5 路并行 agent 静态代码追踪 + 已有 21 个 cargo test 印证。
最严重的 4 条由人工核对源码确认属实。其余建议落盘新测试动态验证。

调查范围：`Easy_Analyzer/src/{types,symbol,semantic,ir}.rs` + `Easy_Server/src/main.rs` + `index.html`。

---

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

---

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

---

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

---

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

---

## ✅ 已确认正确处理

**28 条 PDF 语义检查**：1.5/2.1/2.2/2.3/3.5/5.2/5.4/6.1/6.3/6.4/7.4/8.2/8.3/9.2/9.3 全部在源码层正确实现，`requirements_coverage.rs` + `smoke.rs` 21 个测试印证。

**算术/比较 IR**：操作数顺序（`a-b → (-, a, b, t)`）、IF_FALSE 跳 else/end、while cond/end 标签结构、`=` 方向、CALL 返回值写 result、PARAM FIFO、临时变量单调不复用 —— 全部正确。

**边界**：自递归、互递归、shadowing 链（含类型变化）、深嵌套 if/while、空函数/空语句、多错误恢复、条件中的复杂表达式与函数调用、多个不可变引用、嵌套元组/数组、数组静态越界、未声明函数调用 —— 全部行为符合预期。

**Server**：JSON 字段 `lexerErrors/parseError/semanticErrors/quadruples` 与前端期望一致（仅 `typeEnum` 除外），quadruple 子字段 `index/op/arg1/arg2/result` 命名一致，Vec 空时返回 `[]` 而非 `null`，监听 `127.0.0.1:3000` 限本机。

**未发现可达 panic**：全文 `unwrap/expect` 仅 `semantic.rs:189` 一处，被"先全部 declare 再 analyze"的顺序保护。

---

## 修复优先级

1. **必修**（影响强制规则交付）：#1, #2, #3 — IR 含 break/continue 但跳不出去
2. **应修**（影响合法代码 / 易扣分）：#4, #5, #6
3. **可修**（润色 IR / 错误信息）：#7-#16