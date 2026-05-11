"""
Main entry point for the Compile Theory Course Design parser.

Usage:
    python main.py [source_file]

If a source_file is provided, it is parsed and the resulting AST is printed.
If no argument is given, the program enters an interactive REPL where you can
type or paste code and see the token stream and AST.
"""

import sys

from lexer import Lexer, LexerError
from parser import Parser, ParseError
from ast_nodes import ASTPrinter


def run_source(source: str, *, show_tokens: bool = False) -> bool:
    """Lex + parse *source* and print the AST.

    Returns True on success, False on error.
    """
    # ---- Lexical analysis ----
    try:
        tokens = Lexer(source).tokenize()
    except LexerError as exc:
        print(f"[Lexer Error] {exc}", file=sys.stderr)
        return False

    if show_tokens:
        print("=== Token Stream ===")
        for tok in tokens:
            print(f"  {tok}")
        print()

    # ---- Syntactic analysis ----
    try:
        ast = Parser(tokens).parse()
    except ParseError as exc:
        print(f"[Parser Error] {exc}", file=sys.stderr)
        return False

    # ---- Print AST ----
    print("=== Abstract Syntax Tree ===")
    printer = ASTPrinter()
    print(ast.accept(printer))
    return True


def run_file(path: str) -> None:
    try:
        with open(path, encoding='utf-8') as fh:
            source = fh.read()
    except OSError as exc:
        print(f"Error reading file: {exc}", file=sys.stderr)
        sys.exit(1)

    print(f"=== Parsing: {path} ===\n")
    success = run_source(source, show_tokens=True)
    sys.exit(0 if success else 1)


def repl() -> None:
    print("Simple C-like Language Parser  (Compile Theory Course Design)")
    print("Type 'quit' or 'exit' to leave.  Type 'tokens' to toggle token display.")
    print("End multi-line input with a blank line.\n")

    show_tokens = False

    while True:
        try:
            lines = []
            prompt = ">>> "
            while True:
                try:
                    line = input(prompt)
                except EOFError:
                    print()
                    return
                if line.strip().lower() in ('quit', 'exit'):
                    return
                if line.strip().lower() == 'tokens':
                    show_tokens = not show_tokens
                    print(f"Token display {'ON' if show_tokens else 'OFF'}")
                    break
                if line == '' and lines:
                    break
                lines.append(line)
                prompt = "... "

            source = '\n'.join(lines)
            if source.strip():
                run_source(source, show_tokens=show_tokens)
            print()
        except KeyboardInterrupt:
            print()
            continue


def main() -> None:
    if len(sys.argv) > 1:
        run_file(sys.argv[1])
    else:
        repl()


if __name__ == '__main__':
    main()
