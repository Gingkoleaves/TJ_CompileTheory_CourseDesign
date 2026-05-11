"""Unit tests for the Interpreter (src/interpreter.py).

Each test runs a complete MiniLang source string through
Lexer → Parser → Interpreter and inspects the output list.
"""

import pytest
from src.lexer import Lexer
from src.parser import Parser
from src.interpreter import Interpreter, RuntimeError as MiniRuntimeError


def run(source: str) -> list:
    """Execute *source* and return the printed-output list (strings)."""
    tokens = Lexer(source).tokenize()
    ast = Parser(tokens).parse()
    interp = Interpreter()
    interp.execute(ast)
    return interp.output


class TestBasicOutput:
    def test_print_literal(self):
        assert run("print(42);") == ["42"]

    def test_print_zero(self):
        assert run("print(0);") == ["0"]

    def test_multiple_prints(self):
        assert run("print(1); print(2); print(3);") == ["1", "2", "3"]


class TestVariables:
    def test_var_decl_default(self):
        assert run("int x; print(x);") == ["0"]

    def test_var_decl_init(self):
        assert run("int x = 7; print(x);") == ["7"]

    def test_var_assign(self):
        assert run("int x = 1; x = 99; print(x);") == ["99"]

    def test_multiple_vars(self):
        assert run("int a = 3; int b = 4; print(a + b);") == ["7"]


class TestArithmetic:
    def test_addition(self):
        assert run("print(3 + 4);") == ["7"]

    def test_subtraction(self):
        assert run("print(10 - 3);") == ["7"]

    def test_multiplication(self):
        assert run("print(3 * 4);") == ["12"]

    def test_integer_division(self):
        assert run("print(10 / 3);") == ["3"]

    def test_modulo(self):
        assert run("print(10 % 3);") == ["1"]

    def test_operator_precedence(self):
        assert run("print(2 + 3 * 4);") == ["14"]

    def test_parentheses(self):
        assert run("print((2 + 3) * 4);") == ["20"]

    def test_unary_minus(self):
        assert run("int x = 5; print(-x);") == ["-5"]

    def test_nested_unary(self):
        assert run("print(-(-3));") == ["3"]


class TestComparisons:
    def test_eq_true(self):
        assert run("print(3 == 3);") == ["1"]

    def test_eq_false(self):
        assert run("print(3 == 4);") == ["0"]

    def test_neq(self):
        assert run("print(3 != 4);") == ["1"]

    def test_lt(self):
        assert run("print(2 < 5);") == ["1"]

    def test_le_equal(self):
        assert run("print(5 <= 5);") == ["1"]

    def test_gt(self):
        assert run("print(6 > 3);") == ["1"]

    def test_ge(self):
        assert run("print(7 >= 7);") == ["1"]


class TestLogical:
    def test_and_true(self):
        assert run("print(1 && 1);") == ["1"]

    def test_and_false(self):
        assert run("print(1 && 0);") == ["0"]

    def test_or_true(self):
        assert run("print(0 || 1);") == ["1"]

    def test_or_false(self):
        assert run("print(0 || 0);") == ["0"]

    def test_not_true(self):
        assert run("print(!0);") == ["1"]

    def test_not_false(self):
        assert run("print(!1);") == ["0"]


class TestIf:
    def test_if_taken(self):
        assert run("int x = 1; if (x) { print(99); }") == ["99"]

    def test_if_not_taken(self):
        assert run("int x = 0; if (x) { print(99); }") == []

    def test_if_else_taken(self):
        assert run("int x = 0; if (x) { print(1); } else { print(2); }") == ["2"]

    def test_if_else_if(self):
        src = """
        int x = 2;
        if (x == 1) { print(1); }
        else if (x == 2) { print(2); }
        else { print(3); }
        """
        assert run(src) == ["2"]


class TestWhile:
    def test_while_counts(self):
        src = "int i = 0; while (i < 5) { print(i); i = i + 1; }"
        assert run(src) == ["0", "1", "2", "3", "4"]

    def test_while_not_entered(self):
        assert run("int x = 0; while (x > 0) { print(x); }") == []

    def test_while_sum(self):
        src = """
        int sum = 0;
        int i = 1;
        while (i <= 10) {
            sum = sum + i;
            i = i + 1;
        }
        print(sum);
        """
        assert run(src) == ["55"]


class TestFor:
    def test_for_basic(self):
        src = "for (int i = 0; i < 3; i = i + 1) { print(i); }"
        assert run(src) == ["0", "1", "2"]

    def test_for_factorial(self):
        src = """
        int result = 1;
        for (int i = 1; i <= 5; i = i + 1) {
            result = result * i;
        }
        print(result);
        """
        assert run(src) == ["120"]

    def test_for_empty_init(self):
        src = "int i = 0; for (; i < 3; i = i + 1) { print(i); }"
        assert run(src) == ["0", "1", "2"]


class TestScopes:
    def test_inner_scope_does_not_leak(self):
        src = """
        int x = 10;
        {
            int x = 99;
            print(x);
        }
        print(x);
        """
        assert run(src) == ["99", "10"]

    def test_inner_scope_reads_outer(self):
        src = """
        int x = 5;
        {
            print(x);
        }
        """
        assert run(src) == ["5"]

    def test_inner_scope_modifies_outer(self):
        src = """
        int x = 1;
        {
            x = 42;
        }
        print(x);
        """
        assert run(src) == ["42"]


class TestFibonacci:
    def test_first_10_fibonacci(self):
        src = """
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
        """
        expected = ["0", "1", "1", "2", "3", "5", "8", "13", "21", "34"]
        assert run(src) == expected


class TestErrors:
    def test_division_by_zero(self):
        with pytest.raises(MiniRuntimeError):
            run("print(1 / 0);")

    def test_modulo_by_zero(self):
        with pytest.raises(MiniRuntimeError):
            run("print(5 % 0);")

    def test_undefined_variable(self):
        with pytest.raises(MiniRuntimeError):
            run("print(undeclared);")

    def test_assign_to_undeclared(self):
        with pytest.raises(MiniRuntimeError):
            run("x = 5;")
