"""
Lexical Analyzer (Scanner)
--------------------------
Converts a source string into a flat list of tokens.
Supports:
  - Integer literals
  - Identifiers and keywords: int, if, else, while, for, print, return
  - Arithmetic operators: + - * / %
  - Comparison operators: == != < <= > >=
  - Logical operators: && || !
  - Assignment: =
  - Delimiters: ( ) { } ; ,
  - Single-line comments: // ...
"""

from enum import Enum, auto


class TokenType(Enum):
    # Literals
    NUMBER = "NUMBER"
    IDENT = "IDENT"

    # Keywords
    INT = "int"
    IF = "if"
    ELSE = "else"
    WHILE = "while"
    FOR = "for"
    PRINT = "print"
    RETURN = "return"

    # Arithmetic operators
    PLUS = "+"
    MINUS = "-"
    STAR = "*"
    SLASH = "/"
    MOD = "%"

    # Comparison operators
    EQ = "=="
    NEQ = "!="
    LT = "<"
    LE = "<="
    GT = ">"
    GE = ">="

    # Logical operators
    AND = "&&"
    OR = "||"
    NOT = "!"

    # Assignment
    ASSIGN = "="

    # Delimiters
    LPAREN = "("
    RPAREN = ")"
    LBRACE = "{"
    RBRACE = "}"
    SEMICOLON = ";"
    COMMA = ","

    # Special
    EOF = "EOF"


KEYWORDS = {
    "int": TokenType.INT,
    "if": TokenType.IF,
    "else": TokenType.ELSE,
    "while": TokenType.WHILE,
    "for": TokenType.FOR,
    "print": TokenType.PRINT,
    "return": TokenType.RETURN,
}

SINGLE_CHAR_TOKENS = {
    "+": TokenType.PLUS,
    "-": TokenType.MINUS,
    "*": TokenType.STAR,
    "/": TokenType.SLASH,
    "%": TokenType.MOD,
    "<": TokenType.LT,
    ">": TokenType.GT,
    "!": TokenType.NOT,
    "=": TokenType.ASSIGN,
    "(": TokenType.LPAREN,
    ")": TokenType.RPAREN,
    "{": TokenType.LBRACE,
    "}": TokenType.RBRACE,
    ";": TokenType.SEMICOLON,
    ",": TokenType.COMMA,
}


class Token:
    """Represents a single lexical token."""

    def __init__(self, type: TokenType, value: str, line: int, col: int):
        self.type = type
        self.value = value
        self.line = line
        self.col = col

    def __repr__(self):
        return f"Token({self.type.name}, {self.value!r}, line={self.line}, col={self.col})"

    def __eq__(self, other):
        if isinstance(other, Token):
            return self.type == other.type and self.value == other.value
        return NotImplemented


class LexerError(Exception):
    pass


class Lexer:
    """
    Lexical analyzer: transforms a source string into a sequence of tokens.

    Usage::

        lexer = Lexer("int x = 5;")
        tokens = lexer.tokenize()
    """

    def __init__(self, source: str):
        self.source = source
        self.pos = 0
        self.line = 1
        self.col = 1

    # ------------------------------------------------------------------
    # Internal helpers
    # ------------------------------------------------------------------

    def _error(self, msg: str):
        raise LexerError(f"Lexer error at line {self.line}, col {self.col}: {msg}")

    def _peek(self) -> str:
        if self.pos < len(self.source):
            return self.source[self.pos]
        return ""

    def _peek_next(self) -> str:
        if self.pos + 1 < len(self.source):
            return self.source[self.pos + 1]
        return ""

    def _advance(self) -> str:
        ch = self.source[self.pos]
        self.pos += 1
        if ch == "\n":
            self.line += 1
            self.col = 1
        else:
            self.col += 1
        return ch

    def _skip_whitespace(self):
        while self.pos < len(self.source) and self._peek() in " \t\r\n":
            self._advance()

    def _skip_line_comment(self):
        """Skip from '//' to end of line."""
        while self.pos < len(self.source) and self._peek() != "\n":
            self._advance()

    # ------------------------------------------------------------------
    # Token readers
    # ------------------------------------------------------------------

    def _read_number(self) -> Token:
        start_line, start_col = self.line, self.col
        num = ""
        while self.pos < len(self.source) and self._peek().isdigit():
            num += self._advance()
        return Token(TokenType.NUMBER, num, start_line, start_col)

    def _read_ident_or_keyword(self) -> Token:
        start_line, start_col = self.line, self.col
        ident = ""
        while self.pos < len(self.source) and (
            self._peek().isalnum() or self._peek() == "_"
        ):
            ident += self._advance()
        token_type = KEYWORDS.get(ident, TokenType.IDENT)
        return Token(token_type, ident, start_line, start_col)

    # ------------------------------------------------------------------
    # Public interface
    # ------------------------------------------------------------------

    def next_token(self) -> Token:
        """Return the next token from the source."""
        while self.pos < len(self.source):
            self._skip_whitespace()
            if self.pos >= len(self.source):
                break

            # Single-line comments
            if self._peek() == "/" and self._peek_next() == "/":
                self._skip_line_comment()
                continue

            ch = self._peek()
            line, col = self.line, self.col

            if ch.isdigit():
                return self._read_number()

            if ch.isalpha() or ch == "_":
                return self._read_ident_or_keyword()

            # Two-character operators
            two = ch + self._peek_next()
            if two == "==":
                self._advance()
                self._advance()
                return Token(TokenType.EQ, "==", line, col)
            if two == "!=":
                self._advance()
                self._advance()
                return Token(TokenType.NEQ, "!=", line, col)
            if two == "<=":
                self._advance()
                self._advance()
                return Token(TokenType.LE, "<=", line, col)
            if two == ">=":
                self._advance()
                self._advance()
                return Token(TokenType.GE, ">=", line, col)
            if two == "&&":
                self._advance()
                self._advance()
                return Token(TokenType.AND, "&&", line, col)
            if two == "||":
                self._advance()
                self._advance()
                return Token(TokenType.OR, "||", line, col)

            # Single-character tokens
            if ch in SINGLE_CHAR_TOKENS:
                self._advance()
                return Token(SINGLE_CHAR_TOKENS[ch], ch, line, col)

            self._error(f"Unexpected character: {ch!r}")

        return Token(TokenType.EOF, "", self.line, self.col)

    def tokenize(self) -> list:
        """Tokenize the entire source and return the list of tokens (including EOF)."""
        tokens = []
        while True:
            tok = self.next_token()
            tokens.append(tok)
            if tok.type == TokenType.EOF:
                break
        return tokens
