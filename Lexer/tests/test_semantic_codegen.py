"""Unit tests for the SemanticAnalyzer (src/semantic.py) and
CodeGenerator (src/codegen.py)."""

import pytest
from src.lexer import Lexer
from src.parser import Parser
from src.semantic import SemanticAnalyzer
from src.codegen import CodeGenerator


def analyze(source: str) -> list:
    """Return list of semantic error messages for *source*."""
    tokens = Lexer(source).tokenize()
    ast = Parser(tokens).parse()
    analyzer = SemanticAnalyzer()
    analyzer.analyze(ast)
    return analyzer.errors


def generate(source: str) -> list:
    """Return list of Quadruple objects for *source*."""
    tokens = Lexer(source).tokenize()
    ast = Parser(tokens).parse()
    gen = CodeGenerator()
    gen.generate(ast)
    return gen.quads


# -----------------------------------------------------------------------
# Semantic Analyzer tests
# -----------------------------------------------------------------------

class TestSemanticOk:
    def test_simple_decl(self):
        assert analyze("int x = 5;") == []

    def test_use_after_decl(self):
        assert analyze("int x = 5; print(x);") == []

    def test_assign_declared(self):
        assert analyze("int x; x = 3;") == []

    def test_shadow_in_inner_scope(self):
        # Redeclaration in a nested scope is valid
        assert analyze("int x = 1; { int x = 2; }") == []

    def test_for_loop_counter(self):
        assert analyze("for (int i = 0; i < 5; i = i + 1) { print(i); }") == []

    def test_if_else(self):
        src = "int x = 1; if (x) { int y = 2; print(y); } else { int z = 3; print(z); }"
        assert analyze(src) == []


class TestSemanticErrors:
    def test_undeclared_use(self):
        errors = analyze("print(x);")
        assert len(errors) == 1
        assert "x" in errors[0]

    def test_undeclared_assignment(self):
        errors = analyze("x = 5;")
        assert len(errors) >= 1
        assert "x" in errors[0]

    def test_redeclaration_same_scope(self):
        errors = analyze("int x = 1; int x = 2;")
        assert len(errors) == 1
        assert "x" in errors[0]

    def test_use_before_decl(self):
        errors = analyze("print(x); int x = 1;")
        assert len(errors) >= 1

    def test_multiple_errors_collected(self):
        errors = analyze("print(a); print(b); print(c);")
        assert len(errors) == 3


# -----------------------------------------------------------------------
# Code Generator tests
# -----------------------------------------------------------------------

class TestCodeGenBasic:
    def test_var_decl_generates_assign(self):
        quads = generate("int x = 5;")
        assert len(quads) == 1
        q = quads[0]
        assert q.op == "="
        assert q.arg1 == "5"
        assert q.result == "x"

    def test_var_decl_no_init_is_zero(self):
        quads = generate("int x;")
        assert quads[0].op == "="
        assert quads[0].arg1 == "0"

    def test_binop_creates_temp(self):
        quads = generate("int x = 2 + 3;")
        # First quad: (+, 2, 3, t1); second: (=, t1, _, x)
        assert quads[0].op == "+"
        assert quads[0].result.startswith("t")
        assert quads[1].op == "="
        assert quads[1].arg1 == quads[0].result

    def test_print_quad(self):
        quads = generate("print(42);")
        last = quads[-1]
        assert last.op == "print"

    def test_while_has_labels_and_jumps(self):
        src = "int i = 0; while (i < 3) { i = i + 1; }"
        quads = generate(src)
        ops = [q.op for q in quads]
        assert "label" in ops
        assert "jz" in ops
        assert "jmp" in ops

    def test_if_has_conditional_jump(self):
        quads = generate("int x = 1; if (x) { print(x); }")
        ops = [q.op for q in quads]
        assert "jz" in ops
        assert "label" in ops

    def test_if_else_has_unconditional_jump(self):
        quads = generate("int x = 1; if (x) { print(1); } else { print(0); }")
        ops = [q.op for q in quads]
        assert "jmp" in ops

    def test_for_loop(self):
        src = "for (int i = 0; i < 3; i = i + 1) { print(i); }"
        quads = generate(src)
        ops = [q.op for q in quads]
        assert "label" in ops
        assert "jz" in ops
        assert "jmp" in ops

    def test_get_code_string(self):
        gen = CodeGenerator()
        tokens = Lexer("int x = 1;").tokenize()
        ast = Parser(tokens).parse()
        gen.generate(ast)
        code = gen.get_code()
        assert "=" in code
        assert "x" in code
