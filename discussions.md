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

## D-3 显式 `return` 后函数体末仍发射兜底 `RETURN _ _ _`

### 现状

- **来源**：第三轮 `tests/triage_round3.rs::cand_x_explicit_return_followed_by_terminator`
- **现象**：函数体里已有 `return 1;`，末尾不存在 tail 时，仍发射一条 `RETURN _ _ _`。
  同一函数会出现两条 RETURN —— 第二条不可达。
  ```rust
  fn f() -> i32 { return 1; }
  // 生成 IR：
  //   FUNC f
  //   RETURN 1 _ _
  //   RETURN _ _ _   ← 不可达
  //   END_FUNC f
  ```
- **位置**：`Easy_Analyzer/src/semantic.rs::analyze_function`（`else` 分支兜底 RETURN）
- **根因**：当前实现"无尾表达式 → 一律发射一条 `RETURN _ _ _` 作为函数终结子"，
  以让下游解释器/翻译器无需做控制流分析就可以稳定识别函数边界。

### 为何不修

1. **下游解释器统一性**。`ir_interpreter.rs` 中所有函数都靠"最后一条必然是 RETURN"
   来判定退出，免去走表分析。删除兜底反而要在解释器里多写一份"碰到 END_FUNC
   也算退出"的逻辑。
2. **不可达指令在生成式翻译里被自然消解**。任何朴素的"按顺序发射"目标代码生成
   都会在第一条 RETURN 处真实 ret，第二条 RETURN 永远不会执行；性能/正确性都无害。
3. **完整控制流分析超出 PDF 强制规则范围**。要严谨地知道"显式 return 之后所有
   路径都已退出"，需要做基础块分析（识别 if/loop/break 等控制流分汇点）。
   这不是 PDF 1.1-9.3 任何一条规则要求的产物。

### 可选改法（若答辩时被问及）

```rust
// analyze_function 末尾兜底 RETURN 之前：
let last_is_return = matches!(
    f.body.statements.last(),
    Some(Statement::Return { .. })
);
if !last_is_return {
    self.ir.emit("RETURN", PLACEHOLDER, PLACEHOLDER, PLACEHOLDER);
}
```

仅检查最后一条语句是否为显式 Return —— 朴素却不完美（无法识别"所有 if/else
分支都 return"的情形）。本项目当前选择"保留兜底、容忍冗余"的稳妥路线。

> **注**：与 R3-3 的修复（同一处分支）并不冲突：R3-3 检查"非 Unit 函数末尾既
> 不是 return 也无 tail"时报错，**仍然**继续发射兜底 RETURN，保持 IR 形态稳定。

---

## D-4 静态越界报错后仍发射 `INDEX` 四元式

### 现状

- **来源**：第三轮 `tests/triage_round3.rs::cand_y_oob_index_still_emits_index_quad`
- **现象**：
  ```rust
  fn main(){ let a:[i32;3]=[1,2,3]; let b:i32 = a[5]; }
  // 已正确报错："数组 `a` 下标 5 越界，合法范围 [0,3)（规则 8.3）"
  // 但 IR 仍发出一条：INDEX a 5 t
  ```
- **位置**：`Easy_Analyzer/src/semantic.rs::gen_index`（以及 `gen_assign` 中
  `Expr::Index` 作 LHS 的分支）
- **根因**：当前策略是"错误恢复保留 IR 形态"——只要源码语法正确，无论语义是否
  报错，IR 都按位置不变地生成出来。这样：
  - 后续依赖 INDEX 结果的表达式（例如 `let c = a[5] + 1;`）不至于因 `a[5]` 这
    条 IR 缺失而把 `+` 的左操作数变成 PLACEHOLDER，进而连锁触发更多虚假错误；
  - 报告里既能看到"哪条规则被违反"，也能看到"假设它合法时 IR 长什么样"，便于
    用户与老师对照打分。

### 为何不修

1. **错误恢复的设计基调一致**。整个 `semantic.rs` 都遵循"报错 + 尽力发 IR"
   的方针（参见 BUG #12 修复后的 break 类型不一致路径、BUG B-5 修复后的
   for 非范围迭代路径）。R3-7 是该方针在 OOB 上的体现，不应单独反例。
2. **真正"下游执行 IR"是上层职责**。我们的 IR 仅在解释器测试里被执行；解释
   器在 `lookup` 数组下标时本就要自带 bounds check，越界会直接 panic / 报错，
   不会"访问越界内存"。生成的目标代码（汇编/字节码）也会由目标平台 runtime
   做边界保护。语义阶段的报错已足够触发"这段程序不能上线"的拦截。
3. **修法在表达可读性上得不偿失**。若改成"OOB 后不发 INDEX，把 result 设为
   PLACEHOLDER"，后续 `let b:i32 = a[5];` 会变成 `= _ _ b`，对调试 IR 反而
   更迷惑。

### 可选改法（若答辩时被问及）

- 方案 A：检测到静态越界后，发 `INDEX a 5 t` 但**同时**把 `t` 的初值写为 0
  （`= 0 _ t`），明确告诉读者"这是占位值"。
- 方案 B：不再发射 INDEX，并把 result 替换为新增的 `<OOB>` 常量字面量，让 IR
  里仍有占位但语义清晰。
- 本项目当前选择最保守的方案：**保留 INDEX 不变**，靠 `semantic_errors` 输出
  给用户"这条 INDEX 实际不应执行"的信息。

---

## D-5 `let x = ();` 被语义阶段拒绝

### 现状

- **位置**：`Easy_Analyzer/src/semantic.rs::declare_variable`（处理 `(None, expr_ty)` 推断分支，约 L366-371）
- **现象**：
  ```rust
  fn main(){ let x = (); }
  // 报：无法用 `()` 初始化变量 `x`：函数无返回值不可作为表达式（规则 3.5）
  ```
- **代码**：
  ```rust
  (None, expr_ty) => {
      if matches!(expr_ty, Type::Unit) {
          self.error(format!(
              "无法用 `()` 初始化变量 `{}`：函数无返回值不可作为表达式（规则 3.5）",
              binding.name
          ));
          Type::Error
      } else { expr_ty.clone() }
  }
  ```
- **根因**：规则 3.5 的字面表述是「函数无返回值（Unit）不能作右值」，针对的是 `let x = foo();`（`foo` 返回 `()`）这种典型错误。当前实现把"无类型注解 + RHS 是 Unit"的全部场景都拦下了，连 `()` 字面量本身也被牵连。

### Rust 严格语义

`()` 是合法的 Unit 值字面量；`let x = ();` 在 rustc 下完全合法，`x: ()`。同理 `let x = {};`（空块表达式）也是 Unit。规则 3.5 的禁止点是「调用」`f()` 把 `void` 函数当值用，而非禁止 Unit 类型本身。

### 当前取舍的依据

1. **教学语义偏严**：本课程未引入 `()` 作为有用类型的场景（没有 `Option`/`Result`/`Vec::push` 等返回 Unit 的方法链），允许 `let x = ();` 在示例集中没有正向用例，反而容易掩盖 `let x = foo();`（`foo` 是 Unit 函数）这类真实错误。
2. **类型推断侧的便利**：若放行 `let x = ();`，`x` 类型被推断为 `Unit`，后续 `x + 1` 会触发"运算 `+` 仅支持 i32"——错误位移到使用点，对教学示例的可读性是负向影响。
3. **PDF 强制规则未要求**：规则 3.5 的描述用了「函数无返回值不可作表达式」这一更宽口径表述，"扩散"到任何 Unit 推断在课程语义内不构成冲突。

### 改严格的成本

- `declare_variable` 把上述 `if matches!(expr_ty, Type::Unit)` 改为只针对 `Type::Unit` 中"由函数调用产生"的子情形。但 `Type::Unit` 本身不携带"来源是调用"的信息——要么在 `ExprValue` 新增 origin 字段、要么在该分支前对 `init.as_ref()` 做 AST 形态识别（是否是 `Expr::Call`）。
- 同样的"扩散"也存在于 `gen_assign` 的 Identifier 分支（约 L432-436），需同步放松。
- IR 不变（Unit 推断时本就不发出额外 `=`）。

### 决策点

**是否对齐 Rust 字面 Unit 语义？**
- 倾向"保留宽松"以外的方案均要触动 AST 形态识别或 origin 元信息——工作量与教学收益不成正比。
- 现阶段保留"任何 Unit 推断都拒绝"，但在用户文档中明确：规则 3.5 在本实现中含覆盖到 Unit 字面量的扩展拦截。
- 若答辩被追问"`()` 不也是 Unit 吗？"，可指向本条 D-5：区分"规则字面"与"工程化扩散"是设计取舍。

---

## D-6 数组下标静态越界检查不识别常量算术表达式

### 现状

- **位置**：`Easy_Analyzer/src/semantic.rs:1045-1059`（`check_array_static_oob`）
- **现象**：当前只识别 `Expr::Number` 与 `Expr::Unary { op: Neg, expr: Number }` 两种字面量形态。常量算术表达式（如 `a[0-1]`、`a[2+1]`）在编译期不被折叠，因此静态越界检查不报错；运行时由解释器 / runtime 的 bounds check 兜底拦截。
  ```rust
  fn main(){ let a:[i32;3]=[1,2,3]; let b:i32 = a[0-1]; }   // 当前不报静态越界
  fn main(){ let a:[i32;3]=[1,2,3]; let b:i32 = a[2+1]; }   // 当前不报静态越界
  ```
- **与 R-1 / R-2 同源**：R-1（负字面量越界漏报）、R-2（超大正字面量解析失败漏报）已经修；本条遗留的是"字面量算术折叠"层次，是上一类问题的进一步推广。

### Rust 严格语义

Rust 编译器对常量上下文（包括数组下标）做完整的 const-eval，会在编译期对 `a[0 - 1]` / `a[2 + 1]` 直接报"index out of bounds"。本课程未引入 const-eval / 常量折叠机制。

### 当前取舍的依据

1. **PDF 规则 8.3 字面未要求常量折叠**：规则文本是"数组下标越界"，没有规定要在多深的语义层做静态判断；最朴素的"字面量比较"实现已经满足课程要求。
2. **运行时兜底有效**：本项目 IR 通过解释器执行，`ir_interpreter.rs` 在做 INDEX / 写下标时本就会做 bounds check 并 panic / 报错，不会出现真"越界访问内存"的安全问题。
3. **与 D-4 的设计基调一致**：D-4 谈"OOB 已报错但仍发射 INDEX 四元式"，背后是"语义阶段做检查、不做剪枝"的取舍；本条则是"语义阶段做检查的覆盖面到字面量为止，不做常量折叠"，方向一致。

### 改严格的成本

- 新增 `eval_const(&Expr) -> Option<i128>` 小型常量求值器，支持 `Expr::Number` / `Expr::Unary{Neg, ..}` / `Expr::Binary{Add|Sub|Mul|Div, ..}`，递归求值。
- `check_array_static_oob` 在原有两条分支之外，回退到 `eval_const(index)` 并对返回的 `i128` 做与现有同样的越界比较（`n < 0 || n >= len`）。
- IR 形态不变（仍按 D-4 策略发射 INDEX）。
- 测试面：需要补 `a[0-1]` / `a[2+1]` / `a[1+1+1]` 等 3-5 个静态越界用例，以及保留 `a[1+1]` 合法用例避免误报。

### 决策点

**是否实现常量折叠以扩大静态越界覆盖？**

- 倾向"保留现状"：PDF 不强制，运行时已有 bounds check，工作量集中在写求值器与回归测试。
- 若答辩被追问"`a[0-1]` 为什么不报错？"，可指本条 D-6：当前实现的静态越界检查范围限定于字面量与一元 Neg，常量算术折叠是已识别但有意未实现的扩展点。

---

## D-7 `let g = f;` 允许把函数名绑定到变量、变量类型为 `Type::Function`

### 现状

- **位置**：`Easy_Analyzer/src/semantic.rs:341-375`（`gen_let` 的 `(None, expr_ty)` 推断分支）
- **现象**：
  ```rust
  fn f() -> i32 { 1 }
  fn main(){
      let g = f;           // 不报错；g 进入变量表，类型为 Type::Function
      let y:i32 = g;       // 报"变量 `y` 用函数 `g` 作为初始化表达式"
  }
  ```
  错误在"使用点"才被发出，且消息把已是变量的 `g` 仍称作"函数"，与 R3-5（同名 fn / let 共存提示风格）的一致性较差。

### Rust 严格语义

`f` 作为函数项（function item）可以被绑定到变量，类型为零大小的 `fn() -> i32` 函数项类型；可通过 `g()` 调用，亦可强转为函数指针 `fn() -> i32`。本课程没有引入函数指针 / 函数项类型这一层抽象。

### 当前取舍的依据

1. **保留宽松带来 IR 形态稳定**：`let g = f;` 在 IR 中发射 `= f _ g`，下游不需要为"特殊禁止 Type::Function 绑定"做额外分支。
2. **错误延迟到使用点**：`let y:i32 = g;`（`g:Type::Function`）会落入 `(_, Type::Function)` 专门化分支并报错；语义检查覆盖不漏，仅措辞略失真。
3. **与 R3-5（同名函数与变量提示）方向一致**：R3-5 的取舍是"先报告冲突让用户改名"，本条则是"允许进入变量表但延迟报使用点"。两条共同选择都是"避免在 `gen_let` 推断分支做强类型禁止"。

### 改严格的成本

- 在 `gen_let` 的 `(None, expr_ty)` 推断分支新增 `Type::Function` 子分支，源头报错：
  ```rust
  (None, Type::Function) => {
      self.error(format!(
          "不能把函数 `{}` 作值绑定（应加 `()` 调用）（规则 3.5）",
          init_value.place
      ));
      Type::Error
  }
  ```
- 同时把 `final_ty` 置为 `Type::Error`，避免 `Type::Function` 渗入变量表导致后续使用点重复报错。
- 测试面：cand_n / cand_nn / 部分 R-5 / R-6 相关错误信息断言可能需要同步刷新预期。

### 决策点

**是否在 `let` 源头堵截 `Type::Function`？**

- 倾向"保留宽松"：当前实现错误能报、IR 形态稳定，只是措辞略失真，工程化成本相对收益偏高。
- 若答辩被追问"`let g = f;` 为什么不在赋值这一行直接报错？"，可指本条 D-7：保留宽松便于 IR 形态稳定 + 错误集中在使用点；改严格的成本不在实现，而在测试预期与错误措辞的整体重新对齐。

---

## 写在最后

D-1 / D-2 / D-5 / D-7 关乎"和 rustc 的语义距离"。
D-3 / D-4 / D-6 关乎"错误恢复 vs IR 严谨性 / 静态分析深度"的设计取舍——共同主张是：

> **语义阶段做检查、不做剪枝。** 报错给用户看；IR 给下游看，无论是否合法。

这把"诊断"与"目标代码生成"解耦，避免一处错误污染整条 IR，也便于课程评审在
代码不能执行时仍然能对 IR 形态打分。

报告里若要谈实现取舍，可以把这几条作为"在教学语义与生产语言间的设计偏好"展开。
如果未来引入"真·控制流分析"或"IR 输出可执行性保证"两条新需求，D-3 / D-4
应作为对应里程碑里的真 BUG 一并修复；D-6 则在引入"常量折叠 / const-eval"
里程碑时与之同步落地。
