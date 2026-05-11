# TJ_CompileTheory — 迷你编译原理练习仓库

## 简介

- 本仓库用于展示和练习编译原理中的词法分析、语法分析、语义检查与简单代码生成等环节。
- 项目包含两个主要子工程：词法器（Lexer）和解析器（Parser），并附带示例、解释器与若干测试用例。

## 主要功能

- 实现词法分析器与示例输入，支持将源代码分解为记号序列。
- 实现语法分析与 AST 节点，能解析简单表达式与控制结构。
- 包含语义检查、解释器（Python）与部分代码生成（Rust）示例。

## 仓库结构

- Lexer: 词法器实现与示例 [Lexer/README.md](Lexer/README.md)
- Parser: 解析器实现与测试 [Parser/README.md](Parser/README.md)
- tests: Python 与 Rust 的测试用例

## 快速开始

1. 安装依赖（如需）并进入工作目录。
2. 运行词法器示例：

   ```sh
   python3 Lexer/main.py Lexer/examples/hello.mc
   ```

3. 运行解析器示例：

   ```sh
   cd Easy_Parser && cargo run -- sample.c
   ```

4. 运行 Python 测试：

   ```sh
   pytest -q
   ```

5. 如果要运行子模块中的 Rust 测试（如果存在 Cargo 配置）：

   ```sh
   cd Lexer && cargo test
   cd Parser && cargo test
   ```

（注意：Rust 测试命令仅在对应子目录包含 Cargo.toml 时可用。）

## 示例文件

演示程序位于 [Lexer/examples](Lexer/examples) 目录：factorial.mc、fibonacci.mc、fizzbuzz.mc、hello.mc 等。

## 贡献

欢迎提交 Issue 或 Pull Request。提 PR 时请附带描述与可复现的步骤。

## 许可证

仓库当前未指定许可证；如需开源发布，请在仓库根目录添加 LICENSE 文件。
