"""Unit tests for the Lexer (src/lexer.py)."""

import pytest
from src.lexer import Lexer, Token, TokenType, LexerError


def tokenize(source: str):
    """Helper: return list of non-EOF tokens."""
    toks = Lexer(source).tokenize()
    return [t for t in toks if t.type != TokenType.EOF]


class TestNumbers:
    def test_single_digit(self):
        toks = tokenize("7")
        assert len(toks) == 1
        assert toks[0].type == TokenType.NUMBER
        assert toks[0].value == "7"

    def test_multi_digit(self):
        toks = tokenize("1234")
        assert toks[0].type == TokenType.NUMBER
        assert toks[0].value == "1234"

    def test_zero(self):
        toks = tokenize("0")
        assert toks[0].type == TokenType.NUMBER
        assert toks[0].value == "0"


class TestIdentifiersAndKeywords:
    def test_identifier(self):
        toks = tokenize("myVar")
        assert toks[0].type == TokenType.IDENT
        assert toks[0].value == "myVar"

    def test_underscore_ident(self):
        toks = tokenize("_count")
        assert toks[0].type == TokenType.IDENT

    def test_keywords(self):
        keywords = {
            "int": TokenType.INT,
            "if": TokenType.IF,
            "else": TokenType.ELSE,
            "while": TokenType.WHILE,
            "for": TokenType.FOR,
            "print": TokenType.PRINT,
            "return": TokenType.RETURN,
        }
        for word, expected in keywords.items():
            toks = tokenize(word)
            assert toks[0].type == expected, f"Failed for keyword {word!r}"

    def test_keyword_prefix_is_ident(self):
        toks = tokenize("integer")
        assert toks[0].type == TokenType.IDENT


class TestOperators:
    def test_arithmetic(self):
        for sym, tt in [
            ("+", TokenType.PLUS),
            ("-", TokenType.MINUS),
            ("*", TokenType.STAR),
            ("/", TokenType.SLASH),
            ("%", TokenType.MOD),
        ]:
            toks = tokenize(sym)
            assert toks[0].type == tt

    def test_comparison_two_char(self):
        for sym, tt in [
            ("==", TokenType.EQ),
            ("!=", TokenType.NEQ),
            ("<=", TokenType.LE),
            (">=", TokenType.GE),
        ]:
            toks = tokenize(sym)
            assert toks[0].type == tt

    def test_comparison_one_char(self):
        for sym, tt in [("<", TokenType.LT), (">", TokenType.GT)]:
            toks = tokenize(sym)
            assert toks[0].type == tt

    def test_logical(self):
        for sym, tt in [
            ("&&", TokenType.AND),
            ("||", TokenType.OR),
            ("!", TokenType.NOT),
        ]:
            toks = tokenize(sym)
            assert toks[0].type == tt

    def test_assign(self):
        toks = tokenize("=")
        assert toks[0].type == TokenType.ASSIGN


class TestDelimiters:
    def test_delimiters(self):
        for sym, tt in [
            ("(", TokenType.LPAREN),
            (")", TokenType.RPAREN),
            ("{", TokenType.LBRACE),
            ("}", TokenType.RBRACE),
            (";", TokenType.SEMICOLON),
            (",", TokenType.COMMA),
        ]:
            toks = tokenize(sym)
            assert toks[0].type == tt


class TestWhitespaceAndComments:
    def test_whitespace_ignored(self):
        toks = tokenize("  \t\n  42  ")
        assert len(toks) == 1
        assert toks[0].value == "42"

    def test_line_comment_ignored(self):
        toks = tokenize("// this is a comment\n42")
        assert len(toks) == 1
        assert toks[0].value == "42"

    def test_comment_at_end_of_file(self):
        toks = tokenize("// only a comment")
        assert len(toks) == 0

    def test_inline_comment(self):
        toks = tokenize("int x = 5; // x is five")
        types = [t.type for t in toks]
        assert TokenType.NUMBER in types
        assert TokenType.IDENT in types


class TestLineTracking:
    def test_line_number(self):
        toks = tokenize("42\nx")
        assert toks[0].line == 1
        assert toks[1].line == 2

    def test_col_number(self):
        toks = tokenize("  42")
        assert toks[0].col == 3  # 1-based, two spaces then '4'


class TestMultipleTokens:
    def test_simple_declaration(self):
        toks = tokenize("int x = 5;")
        expected_types = [
            TokenType.INT,
            TokenType.IDENT,
            TokenType.ASSIGN,
            TokenType.NUMBER,
            TokenType.SEMICOLON,
        ]
        assert [t.type for t in toks] == expected_types

    def test_expression(self):
        toks = tokenize("a + b * 3")
        types = [t.type for t in toks]
        assert types == [
            TokenType.IDENT,
            TokenType.PLUS,
            TokenType.IDENT,
            TokenType.STAR,
            TokenType.NUMBER,
        ]


class TestErrors:
    def test_invalid_character(self):
        with pytest.raises(LexerError):
            Lexer("@").tokenize()

    def test_invalid_character_mid_source(self):
        with pytest.raises(LexerError):
            Lexer("int x = 5 # bad;").tokenize()
