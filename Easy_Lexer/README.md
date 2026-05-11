# My_Compile — Compiler Theory Course Design

A complete, from-scratch implementation of a small compiler in Python,
written as a Compiler Theory course project. The compiler translates
programs written in **MiniLang** (a tiny C-like language) through five
classic compilation phases and finally executes them.

---

## Language — MiniLang

MiniLang supports:

| Feature | Example |
|---------|---------|
| Integer variable declaration | `int x = 5;` |
| Assignment | `x = x + 1;` |
| Arithmetic | `+ - * / %` |
| Comparison | `== != < <= > >=` |
| Logical | `&& \|\| !` |
| `if / else if / else` | `if (x > 0) { … } else { … }` |
| `while` loop | `while (i < 10) { … }` |
| `for` loop | `for (int i = 0; i < n; i = i + 1) { … }` |
| `print` | `print(expr);` |
| `return` | `return expr;` |
| Line comments | `// this is a comment` |
| Lexical scoping | inner blocks shadow outer variables |

---

## Architecture

```
Source text
    │
    ▼
┌─────────────────────────────────────┐
│  Phase 1 · Lexical Analysis         │  src/lexer.py
│  Lexer → list[Token]                │
└─────────────────────────────────────┘
    │
    ▼
┌─────────────────────────────────────┐
│  Phase 2 · Syntax Analysis          │  src/parser.py
│  Recursive-Descent Parser → AST     │  src/ast_nodes.py
└─────────────────────────────────────┘
    │
    ▼
┌─────────────────────────────────────┐
│  Phase 3 · Semantic Analysis        │  src/semantic.py
│  Symbol table + undeclared-var check│
└─────────────────────────────────────┘
    │
    ▼
┌─────────────────────────────────────┐
│  Phase 4 · Intermediate Code Gen    │  src/codegen.py
│  Three-address code (quadruples)    │
└─────────────────────────────────────┘
    │
    ▼
┌─────────────────────────────────────┐
│  Phase 5 · Execution                │  src/interpreter.py
│  Tree-walking interpreter           │
└─────────────────────────────────────┘
```

### Source files

| File | Description |
|------|-------------|
| `src/lexer.py` | Tokenizer — converts source text to a list of `Token` objects |
| `src/ast_nodes.py` | Dataclass definitions for every AST node type |
| `src/parser.py` | Recursive-descent parser — builds the AST from the token list |
| `src/semantic.py` | Semantic analyzer — scoped symbol table and error collection |
| `src/codegen.py` | Intermediate code generator — emits three-address code quadruples |
| `src/interpreter.py` | Tree-walking interpreter — executes the AST directly |
| `main.py` | CLI entry point |

---

## Usage

```bash
# Run a MiniLang program
python main.py examples/hello.mc

# Show the token list, AST, and quadruples before running
python main.py examples/factorial.mc --tokens --ast --quads
```

### CLI flags

| Flag | Description |
|------|-------------|
| `--tokens` | Print the token stream from the lexer |
| `--ast` | Print the abstract syntax tree |
| `--quads` | Print the intermediate code (quadruples) |

---

## Example programs

### Hello World (`examples/hello.mc`)

```c
// Hello World — simplest MiniLang program
int x = 42;
print(x);
```

Output:
```
42
```

### Fibonacci (`examples/fibonacci.mc`)

```c
int n = 10;
int a = 0;
int b = 1;
int i = 0;

while (i < n) {
    print(a);
    int tmp = a + b;
    a = b;
    b = tmp;
    i = i + 1;
}
```

Output: `0 1 1 2 3 5 8 13 21 34`

### Factorial (`examples/factorial.mc`)

```c
int n = 10;
int result = 1;

for (int i = 1; i <= n; i = i + 1) {
    result = result * i;
}

print(result);
```

Output: `3628800`

---

## Intermediate Code Example

Running `python main.py examples/factorial.mc --quads` shows:

```
=== Intermediate Code (Quadruples) ===
   0: (=, 10, _, n)
   1: (=, 1, _, result)
   2: (=, 1, _, i)
   3: (label, _, _, L1)
   4: (<=, i, n, t1)
   5: (jz, t1, _, L2)
   6: (*, result, i, t2)
   7: (=, t2, _, result)
   8: (+, i, 1, t3)
   9: (=, t3, _, i)
  10: (jmp, _, _, L1)
  11: (label, _, _, L2)
  12: (print, result, _, _)
```

---

## Tests

```bash
pip install pytest
python -m pytest tests/ -v
```

The test suite covers all five compilation phases (117 tests):

| File | Tests |
|------|-------|
| `tests/test_lexer.py` | Token types, keywords, operators, whitespace, comments, errors |
| `tests/test_parser.py` | All statement forms, expression precedence, error recovery |
| `tests/test_interpreter.py` | Arithmetic, comparisons, control flow, scoping, Fibonacci, errors |
| `tests/test_semantic_codegen.py` | Semantic errors, redeclaration, quadruple emission |

