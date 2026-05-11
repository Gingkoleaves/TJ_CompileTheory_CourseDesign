# TJ_CompileTheory

编译原理课程设计仓库，当前以 Rust 实现为主，包含词法分析器、语法分析器与一个 Web 服务原型。

## 仓库结构

- `Easy_Lexer/`：类 Rust 语言词法分析器（Rust）
- `Easy_Parser/`：基于 `Easy_Lexer` token 的递归下降语法分析器（Rust）
- `Easy_Server/`：基于 Axum 的在线分析服务原型
- `Resources/`：课程资料、设计报告与示意图
- `input_ast.json`：示例 AST 输出文件

## 已实现能力

### Easy_Lexer

- 识别关键字、标识符、整数、运算符、界符与结束符 `#`
- 支持 `//` 与 `/* ... */` 注释
- 提供行列号定位与词法错误收集

详细说明见：[`Easy_Lexer/README.md`](Easy_Lexer/README.md)

### Easy_Parser

- 复用 `Easy_Lexer` 的 token 流
- 递归下降分析，构建可序列化 AST（JSON）
- 支持函数、变量声明、赋值、`if/else`、`while`、`for`、`loop`、`break`、`continue` 等结构

详细说明见：[`Easy_Parser/README.md`](Easy_Parser/README.md)

## 快速开始

### 1) 运行词法分析

```bash
cd Easy_Lexer
cargo run -- examples/hello.mc
```

### 2) 运行语法分析并生成 AST

```bash
cd Easy_Parser
cargo run -- ../Easy_Lexer/tests/data/input.rs
```

成功后会在当前目录生成 `input_ast.json`。

## 测试

```bash
cd Easy_Lexer && cargo test
cd ../Easy_Parser && cargo test
```

## 参考资料

- 设计报告（Markdown）：[`Resources/DESIGN_REPORT.md`](Resources/DESIGN_REPORT.md)
- 设计报告（PDF）：[`Resources/DESIGN_REPORT.pdf`](Resources/DESIGN_REPORT.pdf)
