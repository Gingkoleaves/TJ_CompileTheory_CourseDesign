"""
Tests for the lexer (lexical analyzer).
"""

import pytest
from lexer import Lexer, LexerError, Token, TokenType


def lex(source: str) -> list[Token]:
    """Helper: lex source and strip the trailing EOF token."""
    tokens = Lexer(source).tokenize()
    # last token is always EOF
    return tokens[:-1]


# ---------------------------------------------------------------------------
# Basic token recognition
# ---------------------------------------------------------------------------

class TestLiterals:
    def test_integer(self):
        toks = lex("42")
        assert len(toks) == 1
        assert toks[0].type  == TokenType.INTEGER
        assert toks[0].value == 42

    def test_float(self):
        toks = lex("3.14")
        assert len(toks) == 1
        assert toks[0].type  == TokenType.FLOAT
        assert toks[0].value == pytest.approx(3.14)

    def test_integer_before_float(self):
        """Make sure '3' and '3.14' are lexed correctly."""
        toks = lex("3 3.14")
        assert toks[0].type  == TokenType.INTEGER
        assert toks[1].type  == TokenType.FLOAT


class TestIdentifiers:
    def test_simple_id(self):
        toks = lex("foo")
        assert len(toks) == 1
        assert toks[0].type  == TokenType.ID
        assert toks[0].value == "foo"

    def test_id_with_digits(self):
        toks = lex("x1_y2")
        assert toks[0].type  == TokenType.ID
        assert toks[0].value == "x1_y2"

    def test_underscore_start(self):
        toks = lex("_count")
        assert toks[0].type  == TokenType.ID
        assert toks[0].value == "_count"


class TestKeywords:
    @pytest.mark.parametrize("kw, expected", [
        ("if",     TokenType.IF),
        ("else",   TokenType.ELSE),
        ("while",  TokenType.WHILE),
        ("int",    TokenType.INT),
        ("float",  TokenType.FLOAT_KW),
        ("return", TokenType.RETURN),
    ])
    def test_keyword(self, kw, expected):
        toks = lex(kw)
        assert len(toks) == 1
        assert toks[0].type == expected


class TestOperators:
    @pytest.mark.parametrize("src, expected", [
        ("+",  TokenType.PLUS),
        ("-",  TokenType.MINUS),
        ("*",  TokenType.STAR),
        ("/",  TokenType.SLASH),
        ("==", TokenType.EQ),
        ("!=", TokenType.NEQ),
        ("<",  TokenType.LT),
        ("<=", TokenType.LE),
        (">",  TokenType.GT),
        (">=", TokenType.GE),
        ("&&", TokenType.AND),
        ("||", TokenType.OR),
        ("=",  TokenType.ASSIGN),
    ])
    def test_operator(self, src, expected):
        toks = lex(src)
        assert len(toks) == 1
        assert toks[0].type == expected

    def test_le_vs_lt(self):
        toks = lex("<= <")
        assert toks[0].type == TokenType.LE
        assert toks[1].type == TokenType.LT

    def test_ge_vs_gt(self):
        toks = lex(">= >")
        assert toks[0].type == TokenType.GE
        assert toks[1].type == TokenType.GT


class TestDelimiters:
    @pytest.mark.parametrize("src, expected", [
        ("(", TokenType.LPAREN),
        (")", TokenType.RPAREN),
        ("{", TokenType.LBRACE),
        ("}", TokenType.RBRACE),
        (";", TokenType.SEMI),
        (",", TokenType.COMMA),
    ])
    def test_delimiter(self, src, expected):
        toks = lex(src)
        assert toks[0].type == expected


# ---------------------------------------------------------------------------
# Whitespace & comments
# ---------------------------------------------------------------------------

class TestSkipping:
    def test_whitespace_skipped(self):
        toks = lex("  \t  42  \t  ")
        assert len(toks) == 1
        assert toks[0].value == 42

    def test_newline_increments_line(self):
        all_toks = Lexer("a\nb").tokenize()
        assert all_toks[0].line == 1
        assert all_toks[1].line == 2

    def test_comment_skipped(self):
        toks = lex("42 // this is a comment\n 7")
        assert len(toks) == 2
        assert toks[0].value == 42
        assert toks[1].value == 7

    def test_comment_only(self):
        toks = lex("// nothing here")
        assert toks == []


# ---------------------------------------------------------------------------
# Line / column tracking
# ---------------------------------------------------------------------------

class TestPositions:
    def test_column_tracking(self):
        toks = lex("int x = 5;")
        # int → col 1,  x → col 5,  = → col 7,  5 → col 9,  ; → col 10
        assert toks[0].col == 1   # 'int'
        assert toks[1].col == 5   # 'x'
        assert toks[2].col == 7   # '='
        assert toks[3].col == 9   # '5'
        assert toks[4].col == 10  # ';'

    def test_multiline_column_reset(self):
        src = "a\nb"
        all_toks = Lexer(src).tokenize()
        assert all_toks[0].col == 1  # 'a'
        assert all_toks[1].col == 1  # 'b' on new line


# ---------------------------------------------------------------------------
# Error cases
# ---------------------------------------------------------------------------

class TestErrors:
    def test_unexpected_char(self):
        with pytest.raises(LexerError):
            Lexer("int x = @;").tokenize()

    def test_error_includes_line(self):
        try:
            Lexer("a\nb = @;").tokenize()
        except LexerError as exc:
            assert exc.line == 2
        else:
            pytest.fail("Expected LexerError")


# ---------------------------------------------------------------------------
# Multi-token sequences
# ---------------------------------------------------------------------------

class TestSequences:
    def test_expression(self):
        toks = lex("a + b * 2")
        types = [t.type for t in toks]
        assert types == [
            TokenType.ID, TokenType.PLUS,
            TokenType.ID, TokenType.STAR, TokenType.INTEGER,
        ]

    def test_declaration(self):
        toks = lex("int counter = 0;")
        assert toks[0].type == TokenType.INT
        assert toks[1].type == TokenType.ID
        assert toks[1].value == "counter"
        assert toks[2].type == TokenType.ASSIGN
        assert toks[3].type == TokenType.INTEGER
        assert toks[4].type == TokenType.SEMI

    def test_eof_always_present(self):
        all_toks = Lexer("").tokenize()
        assert all_toks[-1].type == TokenType.EOF
