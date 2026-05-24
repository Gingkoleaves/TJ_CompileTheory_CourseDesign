# 设计讨论事项

本文档专门收纳"非明确缺陷、但与 Rust 严格语义有偏差"或"PDF 未要求但值得展开"的设计取舍。每条都没到必须立即修复的程度，但答辩、代码评审、未来扩展时可能被问到——先把背景、当前取舍、参考做法和决策点写清楚。

---

## D-1（原 B-6）`if` 条件接受 i32 字面量 / 表达式

### 现状
`Easy_Analyzer/src/semantic.rs` 中 `check_condition_type` 同时接受 `Type::Bool` 与 `Type::I32` 作为 `if` / `while` 的条件表达式：

```rust
fn check_condition_type(&mut self, ty: &Type, ctx: &str) {
    if !ty.is_known() {
        return;
    }
    if !matches!(ty, Type::Bool | Type::I32) {
        self.error(format!(
            "{} 条件表达式类型 {} 不可作为条件",
            ctx,
            ty.display()
        ));
    }
}
```

因此 `if 1 { ... }` 与 `while 1 { ... }` 在当前分析器中不报错；四元式生成出的 `IF_FALSE` 对 i32 采用"== 0 视为假，否则真"的语义。

### Rust 严格语义
Rust 的 `if` / `while` 条件**只接受 `bool`**。`if 1 {}` 在 rustc 下报：
> mismatched types: expected `bool`, found integer

理由是避免 C 风格的 0/非 0 隐式真值混淆（与 `if (x = 1)` 这类常见 bug 同源）。

### 当前取舍的依据
1. PDF 强制规则 4.1 / 5.1 只给出 `if 表达式 语句块` / `while 表达式 语句块` 的产生式，**没有要求条件类型**。
2. 本语言没有 `true` / `false` 字面量；`Type::Bool` 只能由比较运算（`< > == != ...`）产生。如果严格要求 `Bool`，那么 `if x { ... }`（`x:i32`）将不可写，必须改成 `if x != 0 { ... }`，对教学示例和 PDF 中的程序写法会增加额外的语义检查面。
3. 四元式 `IF_FALSE` 把"i32 是否为 0"和"bool 是否为假"自然统一为一个零判断。

### 改严格的成本
- `check_condition_type` 仅放行 `Type::Bool`。
- 全部含 `if x { ... }`（x 为整数变量）的现有测试和示例需改写为 `if x != 0 { ... }`。
- IR 不变（已经按 0 / 非 0 处理）。

### 决策点
**是否对齐 Rust 严格语义？**
- 倾向"保留宽松"：教学语义不强调，PDF 不要求。
- 若想"对齐 Rust"：动一行代码加 N 个测试改写。

---

## D-2（原 B-7）发散表达式（`Never` 类型 / `!`）未被使用

### 现状
`Type::Never` 已在 `types.rs` 定义并参与 `compatible`（与任意类型兼容），但**没有任何路径**会推断出 `Never`。具体：
- `return expr;` 作为 **Statement**，不是表达式；调用 `gen_return` 不影响后续 `ExprValue`。
- `loop { ... }`（无 break-value）作为 Expr 的返回类型在 `gen_loop_expr` 末尾是 `Unit`（默认），不是 `Never`。

因此典型 Rust 代码：
```rust
let a:i32 = if cond { return 1; } else { 2 };
```
在当前分析器下 then 分支被视为 `Unit`，else 分支是 `I32`，触发"if 表达式分支类型不一致"。Rust 编译器认为 then 分支发散（`!`），与 i32 兼容，因此合法。

### Rust 严格语义
`!`（Never）类型实现了"strong bottom"：可以与任何类型 unify，作为不返回控制流的占位。`return` / `panic!` / 无限 `loop {}` 都具有 `!` 类型，因此 `if cond { return; } else { 1 }` 整体类型推断为 `i32`。

### 改严格的成本
要让分析器正确处理需要做的事：
1. `gen_return` 增加返回 `ExprValue { ty: Type::Never, place: PLACEHOLDER }`——但 Return 当前是 Statement，要么改 AST，要么在调用点（块的尾部表达式）做特殊识别。
2. `gen_loop_expr` 检测 body 是否包含 `break <expr>;`：若 break 永远不会被执行（控制流分析），返回 `Never`；否则返回 break 值类型。当前已经收集 `break_type`，只在"完全没有 break"时整段 loop 表达式才是 `Never`，否则是 break 的类型——这部分基本能改。
3. `if` / `match`（暂无）分支推断要在比较时把 `Never` 视为单位元，最终类型取另一边。

技术上不算难，但要小心控制流的边界情况。

### 不修的代价
- 答辩演示时若写 `let x:i32 = if cond { return; } else { 0 };` 会被分析器误报。
- PDF 提供的 27 个 `program_X_Y__Z` 例子里没有发散表达式形态，因此正向影响有限。

### 决策点
**是否实现 Never 类型？**
- 倾向"保留现状"：PDF 例子不覆盖，工作量与收益不成正比。
- 实现时建议同时把 `gen_loop_expr` 的尾部默认改成 `Never`（仅在 body 无 break-value 时），可顺手解决"`loop {}` 作为 `let a:i32 = loop {};` 的合法 unify"。

---

## 写在最后

D-1 / D-2 都不影响 PDF 强制规则的检查正确性；它们关乎"和 rustc 的语义距离"。报告里若要谈实现取舍，可以把这两条作为"在教学语义与生产语言间的设计偏好"展开。
