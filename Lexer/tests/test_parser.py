"""Unit tests for the Parser (src/parser.py)."""

import pytest
from src.lexer import Lexer
from src.parser import Parser, ParseError
from src.ast_nodes import (
    Program, VarDecl, Assign, Block, If, While, For,
    Print, Return, BinOp, UnaryOp, Number, Var,
)


def parse(source: str):
    """Helper: lex + parse, return the Program node."""
    tokens = Lexer(source).tokenize()
    return Parser(tokens).parse()


class TestVarDecl:
    def test_int_no_init(self):
        prog = parse("int x;")
        stmt = prog.statements[0]
        assert isinstance(stmt, VarDecl)
        assert stmt.name == "x"
        assert stmt.init is None

    def test_int_with_literal(self):
        prog = parse("int x = 42;")
        stmt = prog.statements[0]
        assert isinstance(stmt, VarDecl)
        assert isinstance(stmt.init, Number)
        assert stmt.init.value == 42

    def test_int_with_expr(self):
        prog = parse("int x = 2 + 3;")
        stmt = prog.statements[0]
        assert isinstance(stmt.init, BinOp)
        assert stmt.init.op == "+"


class TestAssign:
    def test_simple_assign(self):
        prog = parse("int x; x = 10;")
        stmt = prog.statements[1]
        assert isinstance(stmt, Assign)
        assert stmt.name == "x"
        assert isinstance(stmt.value, Number)
        assert stmt.value.value == 10

    def test_assign_expr(self):
        prog = parse("int x; int y; x = y + 1;")
        stmt = prog.statements[2]
        assert isinstance(stmt, Assign)
        assert isinstance(stmt.value, BinOp)


class TestBlock:
    def test_empty_block(self):
        prog = parse("{}")
        stmt = prog.statements[0]
        assert isinstance(stmt, Block)
        assert stmt.statements == []

    def test_nested_block(self):
        prog = parse("{ int x = 1; { int y = 2; } }")
        outer = prog.statements[0]
        assert isinstance(outer, Block)
        inner = outer.statements[1]
        assert isinstance(inner, Block)


class TestIf:
    def test_if_no_else(self):
        prog = parse("int x = 1; if (x) { print(x); }")
        stmt = prog.statements[1]
        assert isinstance(stmt, If)
        assert stmt.else_block is None

    def test_if_else(self):
        prog = parse("int x = 0; if (x) { print(x); } else { print(0); }")
        stmt = prog.statements[1]
        assert isinstance(stmt, If)
        assert stmt.else_block is not None

    def test_if_else_if(self):
        prog = parse("int x = 1; if (x == 1) { print(1); } else if (x == 2) { print(2); }")
        stmt = prog.statements[1]
        assert isinstance(stmt, If)
        assert isinstance(stmt.else_block, If)


class TestWhile:
    def test_while_loop(self):
        prog = parse("int i = 0; while (i < 10) { i = i + 1; }")
        stmt = prog.statements[1]
        assert isinstance(stmt, While)
        assert isinstance(stmt.condition, BinOp)
        assert stmt.condition.op == "<"
        assert isinstance(stmt.body, Block)


class TestFor:
    def test_for_with_var_decl(self):
        prog = parse("for (int i = 0; i < 5; i = i + 1) { print(i); }")
        stmt = prog.statements[0]
        assert isinstance(stmt, For)
        assert isinstance(stmt.init, VarDecl)
        assert isinstance(stmt.condition, BinOp)
        assert isinstance(stmt.update, Assign)

    def test_for_empty_init(self):
        prog = parse("int i = 0; for (; i < 3; i = i + 1) { print(i); }")
        stmt = prog.statements[1]
        assert isinstance(stmt, For)
        assert stmt.init is None

    def test_for_empty_condition(self):
        # infinite-loop style with no condition
        prog = parse("int i = 0; for (int j = 0; ; j = j + 1) { i = i + 1; }")
        stmt = prog.statements[1]
        assert isinstance(stmt, For)
        assert stmt.condition is None


class TestPrint:
    def test_print_literal(self):
        prog = parse("print(42);")
        stmt = prog.statements[0]
        assert isinstance(stmt, Print)
        assert isinstance(stmt.expr, Number)
        assert stmt.expr.value == 42

    def test_print_var(self):
        prog = parse("int x = 1; print(x);")
        stmt = prog.statements[1]
        assert isinstance(stmt, Print)
        assert isinstance(stmt.expr, Var)


class TestReturn:
    def test_return_value(self):
        prog = parse("return 0;")
        stmt = prog.statements[0]
        assert isinstance(stmt, Return)
        assert isinstance(stmt.expr, Number)

    def test_return_no_value(self):
        prog = parse("return;")
        stmt = prog.statements[0]
        assert isinstance(stmt, Return)
        assert stmt.expr is None


class TestExpressions:
    def test_precedence_mul_over_add(self):
        """2 + 3 * 4  =>  BinOp('+', 2, BinOp('*', 3, 4))"""
        prog = parse("int x = 2 + 3 * 4;")
        init = prog.statements[0].init
        assert isinstance(init, BinOp)
        assert init.op == "+"
        assert isinstance(init.right, BinOp)
        assert init.right.op == "*"

    def test_parentheses_override_precedence(self):
        """(2 + 3) * 4  =>  BinOp('*', BinOp('+', 2, 3), 4)"""
        prog = parse("int x = (2 + 3) * 4;")
        init = prog.statements[0].init
        assert isinstance(init, BinOp)
        assert init.op == "*"
        assert isinstance(init.left, BinOp)
        assert init.left.op == "+"

    def test_unary_minus(self):
        prog = parse("int x = -5;")
        init = prog.statements[0].init
        assert isinstance(init, UnaryOp)
        assert init.op == "-"

    def test_logical_and_or(self):
        prog = parse("int x = 1 && 0 || 1;")
        init = prog.statements[0].init
        assert isinstance(init, BinOp)
        assert init.op == "||"

    def test_not_operator(self):
        prog = parse("int x = !0;")
        init = prog.statements[0].init
        assert isinstance(init, UnaryOp)
        assert init.op == "!"


class TestErrors:
    def test_missing_semicolon(self):
        with pytest.raises(ParseError):
            parse("int x = 5")

    def test_missing_rparen(self):
        with pytest.raises(ParseError):
            parse("print(5;")

    def test_unexpected_token(self):
        with pytest.raises(ParseError):
            parse("42;")   # bare expression statement not supported

    def test_empty_source(self):
        prog = parse("")
        assert prog.statements == []
