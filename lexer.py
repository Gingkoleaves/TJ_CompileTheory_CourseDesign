"""
Lexical Analyzer (Lexer / Scanner)

Tokenizes source code of a simple C-like language into a stream of tokens.

Supported token types:
  - Keywords:  if, else, while, int, float, return
  - Identifiers: [a-zA-Z_][a-zA-Z0-9_]*
  - Integer literals: [0-9]+
  - Float literals:   [0-9]+.[0-9]+
  - Operators: + - * / = == != < <= > >= && ||
  - Delimiters: ( ) { } ; ,
  - Comments: // ... (single-line, skipped)
"""

import re
from enum import Enum, auto


class TokenType(Enum):
    # Literals
    INTEGER = auto()
    FLOAT   = auto()
    ID      = auto()

    # Keywords
    IF      = auto()
    ELSE    = auto()
    WHILE   = auto()
    INT     = auto()
    FLOAT_KW = auto()
    RETURN  = auto()

    # Arithmetic operators
    PLUS    = auto()
    MINUS   = auto()
    STAR    = auto()
    SLASH   = auto()

    # Comparison operators
    EQ      = auto()   # ==
    NEQ     = auto()   # !=
    LT      = auto()   # <
    LE      = auto()   # <=
    GT      = auto()   # >
    GE      = auto()   # >=

    # Logical operators
    AND     = auto()   # &&
    OR      = auto()   # ||

    # Assignment
    ASSIGN  = auto()   # =

    # Delimiters
    LPAREN  = auto()   # (
    RPAREN  = auto()   # )
    LBRACE  = auto()   # {
    RBRACE  = auto()   # }
    SEMI    = auto()   # ;
    COMMA   = auto()   # ,

    # Special
    EOF     = auto()


KEYWORDS = {
    'if':     TokenType.IF,
    'else':   TokenType.ELSE,
    'while':  TokenType.WHILE,
    'int':    TokenType.INT,
    'float':  TokenType.FLOAT_KW,
    'return': TokenType.RETURN,
}


class Token:
    """Represents a single lexical token."""

    def __init__(self, type_: TokenType, value, line: int, col: int):
        self.type  = type_
        self.value = value
        self.line  = line
        self.col   = col

    def __repr__(self):
        return f"Token({self.type.name}, {self.value!r}, line={self.line}, col={self.col})"


class LexerError(Exception):
    def __init__(self, message: str, line: int, col: int):
        super().__init__(f"Lexer error at line {line}, col {col}: {message}")
        self.line = line
        self.col  = col


# ---------------------------------------------------------------------------
# Token specification as a list of (name, pattern) pairs.
# Patterns are tried in order; first match wins.
# ---------------------------------------------------------------------------
_TOKEN_SPEC = [
    ('COMMENT',  r'//[^\n]*'),               # single-line comment (skipped)
    ('NEWLINE',  r'\n'),                     # newline (for line tracking)
    ('SKIP',     r'[ \t\r]+'),              # whitespace (skipped)
    ('FLOAT',    r'\d+\.\d+'),              # float literal (before INTEGER)
    ('INTEGER',  r'\d+'),                   # integer literal
    ('GE',       r'>='),
    ('LE',       r'<='),
    ('EQ',       r'=='),
    ('NEQ',      r'!='),
    ('AND',      r'&&'),
    ('OR',       r'\|\|'),
    ('GT',       r'>'),
    ('LT',       r'<'),
    ('ASSIGN',   r'='),
    ('PLUS',     r'\+'),
    ('MINUS',    r'-'),
    ('STAR',     r'\*'),
    ('SLASH',    r'/'),
    ('LPAREN',   r'\('),
    ('RPAREN',   r'\)'),
    ('LBRACE',   r'\{'),
    ('RBRACE',   r'\}'),
    ('SEMI',     r';'),
    ('COMMA',    r','),
    ('ID',       r'[A-Za-z_]\w*'),          # identifier / keyword
]

_MASTER_PATTERN = re.compile(
    '|'.join(f'(?P<{name}>{pattern})' for name, pattern in _TOKEN_SPEC)
)


class Lexer:
    """Converts source text into a list of Tokens."""

    def __init__(self, source: str):
        self.source = source

    def tokenize(self) -> list[Token]:
        tokens: list[Token] = []
        line = 1
        line_start = 0

        for mo in _MASTER_PATTERN.finditer(self.source):
            kind  = mo.lastgroup
            value = mo.group()
            col   = mo.start() - line_start + 1

            if kind == 'NEWLINE':
                line       += 1
                line_start  = mo.end()
                continue
            elif kind in ('SKIP', 'COMMENT'):
                continue
            elif kind == 'FLOAT':
                tokens.append(Token(TokenType.FLOAT, float(value), line, col))
            elif kind == 'INTEGER':
                tokens.append(Token(TokenType.INTEGER, int(value), line, col))
            elif kind == 'ID':
                ttype = KEYWORDS.get(value, TokenType.ID)
                tokens.append(Token(ttype, value, line, col))
            else:
                # Operator / delimiter: map name -> TokenType enum member
                ttype = TokenType[kind]
                tokens.append(Token(ttype, value, line, col))

        # Check for any characters that did not match any pattern
        matched_end = 0
        for mo in _MASTER_PATTERN.finditer(self.source):
            if mo.start() != matched_end:
                # Gap between matches — unrecognized character
                bad_char = self.source[matched_end]
                # Compute line/col for the gap
                lines_before = self.source[:matched_end].count('\n')
                last_nl      = self.source[:matched_end].rfind('\n')
                bad_col      = matched_end - last_nl
                raise LexerError(
                    f"Unexpected character {bad_char!r}",
                    lines_before + 1,
                    bad_col,
                )
            matched_end = mo.end()

        # Final check for trailing unrecognized characters
        if matched_end != len(self.source):
            bad_char     = self.source[matched_end]
            lines_before = self.source[:matched_end].count('\n')
            last_nl      = self.source[:matched_end].rfind('\n')
            bad_col      = matched_end - last_nl
            raise LexerError(
                f"Unexpected character {bad_char!r}",
                lines_before + 1,
                bad_col,
            )

        tokens.append(Token(TokenType.EOF, None, line, col if tokens else 1))
        return tokens
