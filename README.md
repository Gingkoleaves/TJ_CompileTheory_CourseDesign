# TJ_CompileTheory

这是编译原理课程大作业仓库，当前代码包含类 Rust 语言的词法分析、语法分析、语义分析与四元式中间代码生成，并提供一个本地 Web 展示页面。

本 README 描述当前代码版本的真实结构、功能和运行方式。历史报告、旧版设计文档和 `Resources/Rust.pptx` 中的项目描述可能已经过时，作业二完成度请以源码和测试结果为准。

## 项目结构

- `Easy_Lexer/`：词法分析器 crate，提供库函数 `easy_lexer::lex` 和命令行程序 `My_Lexer`。
- `Easy_Parser/`：语法分析器 crate，依赖 `Easy_Lexer`，输出 JSON AST，提供命令行程序 `My_Parser`。
- `Easy_Analyzer/`：作业二主体，依赖 `Easy_Lexer` 与 `Easy_Parser`，进行语义分析并生成四元式，提供命令行程序 `My_Analyzer`。
- `Easy_Server/`：Axum 本地服务，调用 lexer、parser、analyzer，向前端返回 token、AST、语义错误和四元式。
- `index.html`：本地 Web 前端页面，由 `Easy_Server` 内嵌提供。
- `ASSIGNMENT2_COMPLETION.md`：作业二中间代码生成器完成度分析。
- `Resources/`：课程资料与历史文件存放目录，不作为当前实现说明的依据。

当前仓库没有顶层 Cargo workspace，因此需要分别进入各 crate 目录运行构建、测试和命令行程序。

## 当前功能

### 词法分析

`Easy_Lexer` 支持：

- 关键字：`i32`、`let`、`if`、`else`、`while`、`return`、`mut`、`fn`、`for`、`in`、`loop`、`break`、`continue`。
- 标识符、整数、赋值号、算术/比较运算符、括号、分号、冒号、逗号、箭头、点、范围运算符 `..` 和结束符 `#`。
- 单行注释 `//` 与块注释 `/* ... */`。
- token 的原始文本、类别、行号和列号。
- 词法错误收集，例如非法字符、未闭合块注释等。

### 语法分析

`Easy_Parser` 是手写递归下降语法分析器，输入 token 流并输出 AST。当前支持：

- 函数声明、参数列表、`mut` 属性、返回类型和语句块。
- 空语句、变量声明、赋值、表达式语句、`return`。
- `i32`、引用类型、数组类型、元组类型和单元类型 `()`。
- 常量、变量、括号表达式、一元运算、算术运算、比较运算、函数调用。
- `if`、`else`、`else if`、`while`、`for`、`loop`、`break`、`continue`。
- 数组字面量、数组索引、元组字面量、元组字段访问。
- 表达式块、`if` 表达式和 `loop` 表达式。

### 语义分析与中间代码

`Easy_Analyzer` 是作业二的核心实现。公共入口是 `easy_analyzer::analyze(&Program)`，输出：

- `semanticErrors`：语义错误列表。
- `quadruples`：四元式中间代码列表。

已实现的主要检查与生成能力包括：

- 函数签名收集、重复函数检查、参数声明、`main` 入口检查。
- 变量作用域、shadowing、类型推断、未初始化使用检查。
- 不可变变量二次赋值检查。
- 返回值类型与函数声明返回类型匹配检查。
- 表达式类型检查：整数算术、比较、函数实参数量与类型、无返回值函数作为右值等。
- 控制流四元式：`LABEL`、`GOTO`、`IF_FALSE`。
- 函数四元式：`FUNC`、`PARAM_DECL`、`PARAM`、`CALL`、`RETURN`、`END_FUNC`。
- 复合类型四元式：`ARRAY`、`TUPLE`、`INDEX`、`FIELD`、`[]=`、`.=`。
- 引用相关检查：不可变引用、可变引用、解引用读写、基础借用冲突检查。

作业二 PPT 第 25 页要求的最低规则范围：

```text
0.1, 0.2, 0.3,
1.1, 1.2, 1.3, 1.4, 1.5,
2.0, 2.1, 2.2,
3.1, 3.2, 3.3, 3.4, 3.5,
4.1,
5.0, 5.1
```

这些强制项在当前 analyzer 测试中已覆盖并通过。非强制扩展规则的完成情况见 `ASSIGNMENT2_COMPLETION.md`。

## 本地运行

### 环境要求

- Rust stable
- Cargo
- 首次构建需要能够访问 crates.io 下载依赖

### 运行词法分析器

```powershell
cd D:\TJ_CompileTheory\Easy_Lexer
cargo run --bin My_Lexer -- examples\hello.mc
```

也可以从标准输入读取：

```powershell
cd D:\TJ_CompileTheory\Easy_Lexer
"if=123" | cargo run --bin My_Lexer
```

### 运行语法分析器

```powershell
cd D:\TJ_CompileTheory\Easy_Parser
cargo run --bin My_Parser -- ..\Easy_Lexer\examples\hello.mc
```

从标准输入读取时会直接打印 JSON AST：

```powershell
cd D:\TJ_CompileTheory\Easy_Parser
@'
fn main() {
    let mut x:i32 = 1;
    while x < 10 {
        x = x + 1;
    }
    return;
}
'@ | cargo run --bin My_Parser
```

### 运行语义分析与四元式生成

```powershell
cd D:\TJ_CompileTheory\Easy_Analyzer
@'
fn add(mut a:i32, b:i32) -> i32 {
    let mut acc:i32;
    acc = a + b * 2;
    if acc > 10 {
        acc = acc - 1;
    }
    while acc != 0 {
        acc = acc - 1;
    }
    return acc;
}

fn main() {
    let answer:i32 = add(1, 2);
}
'@ | cargo run --bin My_Analyzer
```

输出为 JSON，包含 `semanticErrors` 和 `quadruples`。

### 运行 Web 页面

```powershell
cd D:\TJ_CompileTheory\Easy_Server
cargo run
```

启动后访问：

```text
http://127.0.0.1:3000
```

Web 接口 `/api/analyze` 会返回 token、AST、词法/语法/语义错误以及四元式。前端编辑器增强能力依赖 CDN；离线环境下后端分析接口仍可本地运行。

## 测试

分别进入各 crate 运行：

```powershell
cd D:\TJ_CompileTheory\Easy_Lexer
cargo test

cd D:\TJ_CompileTheory\Easy_Parser
cargo test

cd D:\TJ_CompileTheory\Easy_Analyzer
cargo test --test requirements_coverage
cargo test --test assignment2_ppt_matrix
```

最近一次作业二针对性测试结果：

- `cargo test --test requirements_coverage`：8 passed。
- `cargo test --test assignment2_ppt_matrix`：6 passed。

注意：`Easy_Analyzer` 中还保留了一批探索性 triage/probe 测试，用于暴露边界行为和未来改进点；因此全量 `cargo test` 可能受到非强制扩展探索用例影响。评估作业二 PPT 要求时，优先查看 `requirements_coverage` 和 `assignment2_ppt_matrix`。
