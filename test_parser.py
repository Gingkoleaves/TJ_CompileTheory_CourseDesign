"""
Tests for the recursive-descent parser.
"""

import pytest
from parser import parse, Parser, ParseError
from lexer import Lexer
from ast_nodes import (
    Program, Block,
    DeclStmt, AssignStmt, ReturnStmt,
    IfStmt, WhileStmt,
    BinOp, UnaryOp, NumLiteral, Identifier,
)


# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------

def first_stmt(source: str):
    """Parse source and return the first statement."""
    prog = parse(source)
    assert isinstance(prog, Program)
    assert len(prog.stmts) >= 1
    return prog.stmts[0]


# ---------------------------------------------------------------------------
# Declaration statements
# ---------------------------------------------------------------------------

class TestDeclStmt:
    def test_int_no_init(self):
        node = first_stmt("int x;")
        assert isinstance(node, DeclStmt)
        assert node.type_ == "int"
        assert node.name  == "x"
        assert node.init  is None

    def test_float_no_init(self):
        node = first_stmt("float y;")
        assert isinstance(node, DeclStmt)
        assert node.type_ == "float"
        assert node.name  == "y"

    def test_int_with_init(self):
        node = first_stmt("int count = 0;")
        assert isinstance(node, DeclStmt)
        assert node.name  == "count"
        assert isinstance(node.init, NumLiteral)
        assert node.init.value == 0

    def test_float_with_init(self):
        node = first_stmt("float pi = 3.14;")
        assert isinstance(node, DeclStmt)
        assert isinstance(node.init, NumLiteral)
        assert node.init.value == pytest.approx(3.14)


# ---------------------------------------------------------------------------
# Assignment statements
# ---------------------------------------------------------------------------

class TestAssignStmt:
    def test_assign_literal(self):
        node = first_stmt("x = 5;")
        assert isinstance(node, AssignStmt)
        assert node.name == "x"
        assert isinstance(node.value, NumLiteral)
        assert node.value.value == 5

    def test_assign_expr(self):
        node = first_stmt("result = a + b;")
        assert isinstance(node, AssignStmt)
        assert isinstance(node.value, BinOp)
        assert node.value.op == "+"


# ---------------------------------------------------------------------------
# If statements
# ---------------------------------------------------------------------------

class TestIfStmt:
    def test_if_no_else(self):
        src = "if (x > 0) { y = 1; }"
        node = first_stmt(src)
        assert isinstance(node, IfStmt)
        assert node.else_branch is None
        assert isinstance(node.condition, BinOp)
        assert node.condition.op == ">"

    def test_if_else(self):
        src = "if (x == 0) { y = 0; } else { y = 1; }"
        node = first_stmt(src)
        assert isinstance(node, IfStmt)
        assert node.else_branch is not None

    def test_nested_if(self):
        src = "if (a) { if (b) { x = 1; } }"
        node = first_stmt(src)
        assert isinstance(node, IfStmt)
        inner = node.then_branch.stmts[0]
        assert isinstance(inner, IfStmt)

    def test_else_if(self):
        src = "if (x < 0) { y = -1; } else if (x == 0) { y = 0; } else { y = 1; }"
        node = first_stmt(src)
        assert isinstance(node, IfStmt)
        assert isinstance(node.else_branch, IfStmt)


# ---------------------------------------------------------------------------
# While statements
# ---------------------------------------------------------------------------

class TestWhileStmt:
    def test_while(self):
        src = "while (i < 10) { i = i + 1; }"
        node = first_stmt(src)
        assert isinstance(node, WhileStmt)
        assert isinstance(node.condition, BinOp)
        assert node.condition.op == "<"
        assert isinstance(node.body, Block)

    def test_while_body_is_block(self):
        src = "while (x != 0) { x = x - 1; }"
        node = first_stmt(src)
        assert isinstance(node.body, Block)
        assert len(node.body.stmts) == 1


# ---------------------------------------------------------------------------
# Return statements
# ---------------------------------------------------------------------------

class TestReturnStmt:
    def test_return_value(self):
        node = first_stmt("return 42;")
        assert isinstance(node, ReturnStmt)
        assert isinstance(node.value, NumLiteral)
        assert node.value.value == 42

    def test_return_void(self):
        node = first_stmt("return;")
        assert isinstance(node, ReturnStmt)
        assert node.value is None

    def test_return_expr(self):
        node = first_stmt("return a + b;")
        assert isinstance(node, ReturnStmt)
        assert isinstance(node.value, BinOp)


# ---------------------------------------------------------------------------
# Blocks
# ---------------------------------------------------------------------------

class TestBlock:
    def test_empty_block(self):
        node = first_stmt("{}")
        assert isinstance(node, Block)
        assert node.stmts == []

    def test_block_with_stmts(self):
        src = "{ int a; int b; }"
        node = first_stmt(src)
        assert isinstance(node, Block)
        assert len(node.stmts) == 2


# ---------------------------------------------------------------------------
# Expressions
# ---------------------------------------------------------------------------

class TestExpressions:
    def test_integer_literal(self):
        node = first_stmt("x = 99;")
        assert isinstance(node.value, NumLiteral)
        assert node.value.value == 99

    def test_float_literal(self):
        node = first_stmt("x = 1.5;")
        assert isinstance(node.value, NumLiteral)

    def test_identifier(self):
        node = first_stmt("x = y;")
        assert isinstance(node.value, Identifier)
        assert node.value.name == "y"

    def test_addition(self):
        node = first_stmt("x = a + b;")
        binop = node.value
        assert isinstance(binop, BinOp)
        assert binop.op == "+"

    def test_subtraction(self):
        node = first_stmt("x = a - 1;")
        assert node.value.op == "-"

    def test_multiplication(self):
        node = first_stmt("x = a * b;")
        assert node.value.op == "*"

    def test_division(self):
        node = first_stmt("x = a / 2;")
        assert node.value.op == "/"

    def test_precedence_mul_over_add(self):
        # a + b * c  →  BinOp(+, a, BinOp(*, b, c))
        node = first_stmt("x = a + b * c;")
        outer = node.value
        assert isinstance(outer, BinOp)
        assert outer.op == "+"
        assert isinstance(outer.right, BinOp)
        assert outer.right.op == "*"

    def test_precedence_parens(self):
        # (a + b) * c  →  BinOp(*, BinOp(+, a, b), c)
        node = first_stmt("x = (a + b) * c;")
        outer = node.value
        assert isinstance(outer, BinOp)
        assert outer.op == "*"
        assert isinstance(outer.left, BinOp)
        assert outer.left.op == "+"

    def test_unary_minus(self):
        node = first_stmt("x = -a;")
        assert isinstance(node.value, UnaryOp)
        assert node.value.op == "-"

    def test_comparison_eq(self):
        node = first_stmt("if (x == 0) { y = 1; }")
        assert node.condition.op == "=="

    def test_comparison_ne(self):
        node = first_stmt("if (x != 0) { y = 1; }")
        assert node.condition.op == "!="

    def test_logical_and(self):
        node = first_stmt("if (a && b) { x = 1; }")
        assert node.condition.op == "&&"

    def test_logical_or(self):
        node = first_stmt("if (a || b) { x = 1; }")
        assert node.condition.op == "||"

    def test_chained_addition(self):
        node = first_stmt("x = 1 + 2 + 3;")
        outer = node.value
        # Left-associative: ((1 + 2) + 3)
        assert isinstance(outer, BinOp)
        assert outer.op == "+"
        assert isinstance(outer.left, BinOp)


# ---------------------------------------------------------------------------
# Multi-statement programs
# ---------------------------------------------------------------------------

class TestProgram:
    def test_multiple_stmts(self):
        src = "int a; int b; a = 1; b = 2;"
        prog = parse(src)
        assert len(prog.stmts) == 4

    def test_complete_program(self):
        src = """
        int sum = 0;
        int i = 1;
        while (i <= 10) {
            sum = sum + i;
            i = i + 1;
        }
        return sum;
        """
        prog = parse(src)
        assert len(prog.stmts) == 4  # decl, decl, while, return
        assert isinstance(prog.stmts[0], DeclStmt)
        assert isinstance(prog.stmts[2], WhileStmt)
        assert isinstance(prog.stmts[3], ReturnStmt)


# ---------------------------------------------------------------------------
# Error cases
# ---------------------------------------------------------------------------

class TestParseErrors:
    def test_missing_semicolon(self):
        with pytest.raises(ParseError):
            parse("int x = 5")  # missing ;

    def test_missing_rparen(self):
        with pytest.raises(ParseError):
            parse("if (x > 0 { y = 1; }")

    def test_missing_condition_paren(self):
        with pytest.raises(ParseError):
            parse("while x < 10 { x = x + 1; }")

    def test_unexpected_token(self):
        with pytest.raises(ParseError):
            parse("= 5;")  # no LHS

    def test_incomplete_expression(self):
        with pytest.raises(ParseError):
            parse("x = ;")
