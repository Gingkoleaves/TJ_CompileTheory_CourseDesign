# TJ_CompileTheory

这是编译原理课程大作业 1 的实现仓库，当前项目以 Rust 为主，实现了类 Rust 语言的词法分析、语法分析和一个本地 Web 可视化分析页面。

本 README 描述当前代码的实际结构和运行方式，不依赖历史设计报告或旧版课程资料。

## 项目结构

- `Easy_Lexer/`：词法分析器 crate，提供库函数和命令行程序 `My_Lexer`
- `Easy_Parser/`：语法分析器 crate，依赖 `Easy_Lexer`，提供库函数和命令行程序 `My_Parser`
- `Easy_Server/`：Axum Web 服务，提供在线词法/语法分析接口和前端页面
- `index.html`：Web 前端页面，由 `Easy_Server` 内嵌提供
- `input_ast.json`：示例 AST 输出文件
- `Resources/`：课程资料和历史文档，仅作为资料存放目录

当前仓库没有顶层 Cargo workspace，因此需要分别进入三个 crate 目录执行构建、运行和测试命令。

## 已实现功能概览

### 词法分析

`Easy_Lexer` 能识别课程要求中的主要词法单元：

- 关键字：`i32`、`let`、`if`、`else`、`while`、`return`、`mut`、`fn`、`for`、`in`、`loop`、`break`、`continue`
- 标识符、整数、赋值号、算符、界符、分隔符、特殊符号和结束符 `#`
- 单行注释 `//` 和块注释 `/* ... */`
- token 的原始文本、类别、行号和列号
- 词法错误收集，例如无法识别字符、未闭合块注释

词法器满足作业中特别强调的最长匹配和关键字边界要求：

- `if123` 会被识别为一个标识符
- `if=123` 会被识别为 `if`、`=`、`123` 三个 token

### 语法分析

`Easy_Parser` 是手写递归下降语法分析器，输入来自 `Easy_Lexer` 的 token 流，输出 JSON AST。它覆盖了作业要求的强制语法主线：

- 基础程序和函数声明
- 形参列表、`mut` 属性、`i32` 类型、返回类型
- 语句块、空语句、返回语句
- 变量声明、赋值语句、变量声明并初始化
- 基础表达式、比较运算、加减运算、乘除运算
- 函数调用
- `if` 选择结构
- `while` 循环结构

代码中还实现了一部分非强制扩展语法：

- `else`、`else if`
- `for`、`loop`、`break`、`continue`
- 引用、可变引用和解引用
- 数组类型、数组字面量、数组索引
- 元组类型、元组字面量、元组字段访问
- 表达式块、`if` 表达式、`loop` 表达式

需要注意：当前项目主要做词法和语法分析，不包含完整语义分析。变量是否已声明、类型是否匹配、不可变变量是否被赋值、函数返回类型是否一致等语义约束目前没有系统检查。

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

也可以从标准输入读取源码：

```powershell
cd D:\TJ_CompileTheory\Easy_Lexer
"if=123" | cargo run --bin My_Lexer
```

### 运行语法分析器

```powershell
cd D:\TJ_CompileTheory\Easy_Parser
cargo run --bin My_Parser -- ..\Easy_Lexer\examples\hello.mc
```

从文件读取时，程序会在当前目录生成 `<输入文件名>_ast.json`。从标准输入读取时，程序会直接在终端打印 JSON AST：

```powershell
cd D:\TJ_CompileTheory\Easy_Parser
@'
fn main() {
    let mut x:i32=1;
    while x<10 {
        x=x+1;
    }
    return;
}
'@ | cargo run --bin My_Parser
```

### 运行 Web 页面

```powershell
cd D:\TJ_CompileTheory\Easy_Server
cargo run
```

启动后访问：

```text
http://127.0.0.1:3000
```

Web 页面会调用本地 `/api/analyze` 接口，展示 token 表、AST 树和错误信息。页面中的 CodeMirror 资源来自 CDN，因此离线环境下编辑器增强功能可能无法加载，但后端分析接口仍是本地运行的。

## 测试

分别进入各 crate 运行：

```powershell
cd D:\TJ_CompileTheory\Easy_Lexer
cargo test

cd D:\TJ_CompileTheory\Easy_Parser
cargo test

cd D:\TJ_CompileTheory\Easy_Server
cargo test
```

当前测试状态：

- `Easy_Lexer` 的核心单元测试和文件输出集成测试均通过
- `Easy_Parser` 的核心单元测试和文件输出集成测试均通过
- `Easy_Server` 可编译，当前没有实质性单元测试

更完整的完成度分析见 `COMPLETION_REPORT.md`。
