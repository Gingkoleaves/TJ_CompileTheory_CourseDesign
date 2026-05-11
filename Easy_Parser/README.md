# Easy_Parser — Compile Theory Course Design

A hand-written **lexer + recursive-descent parser** for a simple C-like language, implemented in Rust as a Compiler Theory course design project.

---

## Features

| Component | Description |
|-----------|-------------|
| **Lexer** (`src/lib.rs`) | Converts source text into a token stream |
| **AST / Parser** (`src/lib.rs`) | Recursive-descent parser that builds an AST from tokens |
| **Main** (`src/main.rs`) | CLI tool: parse a file from the command line |

---

## Supported Language

### Types
```
int    float
```

### Statements
```
int x;                      // declaration
int x = 0;                  // declaration with initializer
x = expr;                   // assignment
if (cond) stmt              // if
if (cond) stmt else stmt    // if-else
while (cond) stmt           // while loop
return expr;                // return
{ stmt* }                   // block
```

### Expressions
| Precedence | Operators |
|------------|-----------|
| Lowest  | `\|\|` (logical OR) |
| | `&&` (logical AND) |
| | `==`  `!=`  `<`  `<=`  `>`  `>=` |
| | `+`  `-` |
| | `*`  `/` |
| Highest | unary `-`, `(expr)`, literals, identifiers |

### Comments
```
// single-line comments are supported
```

---

## Running

### Parse a source file
```bash
cargo run -- program.c
```

---

## Example

Given `example.c`:
```c
int sum = 0;
int i = 1;
while (i <= 10) {
    sum = sum + i;
    i = i + 1;
}
return sum;
```

Running `cargo run -- example.c` prints the token stream and the AST:

```
=== Token Stream ===
  Token(INT, 'int', line=1, col=1)
  Token(ID, 'sum', line=1, col=5)
  ...

=== Abstract Syntax Tree ===
Program
  Decl(int sum =
      Num(0))
  Decl(int i =
      Num(1))
  While
    Condition:
      BinOp(<=)
        Id(i)
        Num(10)
    Body:
      Block
        Assign(sum)
          BinOp(+)
            Id(sum)
            Id(i)
        Assign(i)
          BinOp(+)
            Id(i)
            Num(1)
  Return
    Id(sum)
```

---

## Tests

```bash
cargo test
```

Rust tests covering lexer token recognition, parser statement/expression rules, operator precedence, error handling, and more.

---

## Grammar (EBNF)

```
program      → stmt*  EOF
stmt         → decl_stmt | assign_stmt | if_stmt
             | while_stmt | return_stmt | block
decl_stmt    → type ID ( '=' expr )? ';'
assign_stmt  → ID '=' expr ';'
if_stmt      → 'if' '(' expr ')' stmt ( 'else' stmt )?
while_stmt   → 'while' '(' expr ')' stmt
return_stmt  → 'return' expr? ';'
block        → '{' stmt* '}'

expr         → or_expr
or_expr      → and_expr ( '||' and_expr )*
and_expr     → cmp_expr ( '&&' cmp_expr )*
cmp_expr     → add_expr ( ( '==' | '!=' | '<' | '<=' | '>' | '>=' ) add_expr )?
add_expr     → mul_expr ( ( '+' | '-' ) mul_expr )*
mul_expr     → unary   ( ( '*' | '/' ) unary )*
unary        → '-' unary | primary
primary      → ID | INTEGER | FLOAT | '(' expr ')'
```

---

## File Structure

```
Easy_Parser/
├── src/
│   ├── lib.rs      # Lexer, AST, parser
│   └── main.rs     # CLI entry point
├── tests/          # Rust integration tests
└── README.md
```
