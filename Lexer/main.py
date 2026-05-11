#!/usr/bin/env python3
"""
MiniLang Compiler — Main Entry Point
=====================================
Usage:
    python main.py <source_file> [--tokens] [--ast] [--quads]

Options:
    --tokens   Print the token list produced by the lexer
    --ast      Print the AST produced by the parser
    --quads    Print the intermediate code (quadruples) before execution
"""

import sys
from src.lexer import Lexer, LexerError
from src.parser import Parser, ParseError
from src.semantic import SemanticAnalyzer
from src.codegen import CodeGenerator
from src.interpreter import Interpreter


def compile_and_run(
    source: str,
    *,
    show_tokens: bool = False,
    show_ast: bool = False,
    show_quads: bool = False,
) -> bool:
    """
    Compile and run *source*.

    Returns ``True`` on success, ``False`` if any error occurred.
    """
    # ------------------------------------------------------------------ #
    # Phase 1 — Lexical Analysis                                           #
    # ------------------------------------------------------------------ #
    try:
        tokens = Lexer(source).tokenize()
    except LexerError as exc:
        print(f"[Lexer Error] {exc}", file=sys.stderr)
        return False

    if show_tokens:
        print("=== Tokens ===")
        for tok in tokens:
            print(f"  {tok}")
        print()

    # ------------------------------------------------------------------ #
    # Phase 2 — Syntax Analysis                                            #
    # ------------------------------------------------------------------ #
    try:
        ast = Parser(tokens).parse()
    except ParseError as exc:
        print(f"[Parse Error] {exc}", file=sys.stderr)
        return False

    if show_ast:
        print("=== Abstract Syntax Tree ===")
        print(ast)
        print()

    # ------------------------------------------------------------------ #
    # Phase 3 — Semantic Analysis                                          #
    # ------------------------------------------------------------------ #
    analyzer = SemanticAnalyzer()
    analyzer.analyze(ast)
    if analyzer.errors:
        print("[Semantic Errors]", file=sys.stderr)
        for msg in analyzer.errors:
            print(f"  {msg}", file=sys.stderr)
        return False

    # ------------------------------------------------------------------ #
    # Phase 4 — Intermediate Code Generation                               #
    # ------------------------------------------------------------------ #
    codegen = CodeGenerator()
    codegen.generate(ast)

    if show_quads:
        print("=== Intermediate Code (Quadruples) ===")
        print(codegen.get_code())
        print()

    # ------------------------------------------------------------------ #
    # Phase 5 — Execution                                                  #
    # ------------------------------------------------------------------ #
    print("=== Output ===")
    Interpreter().execute(ast)
    return True


def main():
    if len(sys.argv) < 2:
        print(__doc__)
        sys.exit(1)

    filename = sys.argv[1]
    show_tokens = "--tokens" in sys.argv
    show_ast = "--ast" in sys.argv
    show_quads = "--quads" in sys.argv

    try:
        with open(filename, "r", encoding="utf-8") as fh:
            source = fh.read()
    except FileNotFoundError:
        print(f"[Error] File not found: {filename}", file=sys.stderr)
        sys.exit(1)
    except OSError as exc:
        print(f"[Error] Cannot read file: {exc}", file=sys.stderr)
        sys.exit(1)

    success = compile_and_run(
        source,
        show_tokens=show_tokens,
        show_ast=show_ast,
        show_quads=show_quads,
    )
    sys.exit(0 if success else 1)


if __name__ == "__main__":
    main()
