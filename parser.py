"""
Recursive-Descent Parser

Parses a token stream produced by the Lexer into an Abstract Syntax Tree.

Grammar (EBNF):
    program      → stmt*  EOF
    stmt         → decl_stmt
                 | assign_stmt
                 | if_stmt
                 | while_stmt
                 | return_stmt
                 | block
    decl_stmt    → type_kw ID ( '=' expr )? ';'
    assign_stmt  → ID '=' expr ';'
    if_stmt      → 'if' '(' expr ')' stmt ( 'else' stmt )?
    while_stmt   → 'while' '(' expr ')' stmt
    return_stmt  → 'return' expr? ';'
    block        → '{' stmt* '}'

    expr         → or_expr
    or_expr      → and_expr ( '||' and_expr )*
    and_expr     → cmp_expr ( '&&' cmp_expr )*
    cmp_expr     → add_expr ( ( '==' | '!=' | '<' | '<=' | '>' | '>=' ) add_expr )?
    add_expr     → mul_expr ( ( '+' | '-' ) mul_expr )*
    mul_expr     → unary    ( ( '*' | '/' ) unary )*
    unary        → '-' unary | primary
    primary      → ID | INTEGER | FLOAT | '(' expr ')'
"""

from lexer import Lexer, Token, TokenType
from ast_nodes import (
    ASTNode, Program, Block,
    DeclStmt, AssignStmt, ReturnStmt,
    IfStmt, WhileStmt,
    BinOp, UnaryOp, NumLiteral, Identifier,
)


class ParseError(Exception):
    def __init__(self, message: str, token: Token):
        super().__init__(
            f"Parse error at line {token.line}, col {token.col}: {message} "
            f"(got {token.type.name} {token.value!r})"
        )
        self.token = token


_TYPE_KEYWORDS = {TokenType.INT, TokenType.FLOAT_KW}


class Parser:
    """Recursive-descent parser.

    Usage::

        tokens = Lexer(source).tokenize()
        ast    = Parser(tokens).parse()
    """

    def __init__(self, tokens: list[Token]):
        self._tokens = tokens
        self._pos    = 0

    # ------------------------------------------------------------------
    # Token navigation helpers
    # ------------------------------------------------------------------

    def _current(self) -> Token:
        return self._tokens[self._pos]

    def _peek(self, offset: int = 1) -> Token:
        idx = self._pos + offset
        if idx < len(self._tokens):
            return self._tokens[idx]
        return self._tokens[-1]  # EOF

    def _advance(self) -> Token:
        tok = self._tokens[self._pos]
        if self._pos < len(self._tokens) - 1:
            self._pos += 1
        return tok

    def _check(self, *types: TokenType) -> bool:
        return self._current().type in types

    def _match(self, *types: TokenType) -> bool:
        if self._check(*types):
            self._advance()
            return True
        return False

    def _expect(self, type_: TokenType) -> Token:
        tok = self._current()
        if tok.type != type_:
            raise ParseError(
                f"Expected {type_.name}",
                tok,
            )
        return self._advance()

    # ------------------------------------------------------------------
    # Top-level
    # ------------------------------------------------------------------

    def parse(self) -> Program:
        """Parse the entire token stream and return a Program node."""
        stmts = []
        while not self._check(TokenType.EOF):
            stmts.append(self._parse_stmt())
        self._expect(TokenType.EOF)
        return Program(stmts=stmts)

    # ------------------------------------------------------------------
    # Statements
    # ------------------------------------------------------------------

    def _parse_stmt(self) -> ASTNode:
        tok = self._current()

        if tok.type in _TYPE_KEYWORDS:
            return self._parse_decl_stmt()
        if tok.type == TokenType.IF:
            return self._parse_if_stmt()
        if tok.type == TokenType.WHILE:
            return self._parse_while_stmt()
        if tok.type == TokenType.RETURN:
            return self._parse_return_stmt()
        if tok.type == TokenType.LBRACE:
            return self._parse_block()
        if tok.type == TokenType.ID:
            # Look ahead: ID '=' → assignment, otherwise error
            if self._peek().type == TokenType.ASSIGN:
                return self._parse_assign_stmt()
        raise ParseError("Unexpected token; expected a statement", tok)

    def _parse_decl_stmt(self) -> DeclStmt:
        type_tok = self._advance()          # int | float
        type_str = type_tok.value
        name_tok = self._expect(TokenType.ID)
        init = None
        if self._match(TokenType.ASSIGN):
            init = self._parse_expr()
        self._expect(TokenType.SEMI)
        return DeclStmt(type_=type_str, name=name_tok.value, init=init)

    def _parse_assign_stmt(self) -> AssignStmt:
        name_tok = self._expect(TokenType.ID)
        self._expect(TokenType.ASSIGN)
        value = self._parse_expr()
        self._expect(TokenType.SEMI)
        return AssignStmt(name=name_tok.value, value=value)

    def _parse_if_stmt(self) -> IfStmt:
        self._expect(TokenType.IF)
        self._expect(TokenType.LPAREN)
        condition = self._parse_expr()
        self._expect(TokenType.RPAREN)
        then_branch = self._parse_stmt()
        else_branch = None
        if self._match(TokenType.ELSE):
            else_branch = self._parse_stmt()
        return IfStmt(condition=condition, then_branch=then_branch, else_branch=else_branch)

    def _parse_while_stmt(self) -> WhileStmt:
        self._expect(TokenType.WHILE)
        self._expect(TokenType.LPAREN)
        condition = self._parse_expr()
        self._expect(TokenType.RPAREN)
        body = self._parse_stmt()
        return WhileStmt(condition=condition, body=body)

    def _parse_return_stmt(self) -> ReturnStmt:
        self._expect(TokenType.RETURN)
        value = None
        if not self._check(TokenType.SEMI):
            value = self._parse_expr()
        self._expect(TokenType.SEMI)
        return ReturnStmt(value=value)

    def _parse_block(self) -> Block:
        self._expect(TokenType.LBRACE)
        stmts = []
        while not self._check(TokenType.RBRACE) and not self._check(TokenType.EOF):
            stmts.append(self._parse_stmt())
        self._expect(TokenType.RBRACE)
        return Block(stmts=stmts)

    # ------------------------------------------------------------------
    # Expressions  (precedence climbing via nested methods)
    # ------------------------------------------------------------------

    def _parse_expr(self) -> ASTNode:
        return self._parse_or_expr()

    def _parse_or_expr(self) -> ASTNode:
        node = self._parse_and_expr()
        while self._check(TokenType.OR):
            op = self._advance().value
            right = self._parse_and_expr()
            node = BinOp(op=op, left=node, right=right)
        return node

    def _parse_and_expr(self) -> ASTNode:
        node = self._parse_cmp_expr()
        while self._check(TokenType.AND):
            op = self._advance().value
            right = self._parse_cmp_expr()
            node = BinOp(op=op, left=node, right=right)
        return node

    def _parse_cmp_expr(self) -> ASTNode:
        node = self._parse_add_expr()
        _CMP = {TokenType.EQ, TokenType.NEQ, TokenType.LT,
                TokenType.LE, TokenType.GT, TokenType.GE}
        if self._check(*_CMP):
            op = self._advance().value
            right = self._parse_add_expr()
            node = BinOp(op=op, left=node, right=right)
        return node

    def _parse_add_expr(self) -> ASTNode:
        node = self._parse_mul_expr()
        while self._check(TokenType.PLUS, TokenType.MINUS):
            op = self._advance().value
            right = self._parse_mul_expr()
            node = BinOp(op=op, left=node, right=right)
        return node

    def _parse_mul_expr(self) -> ASTNode:
        node = self._parse_unary()
        while self._check(TokenType.STAR, TokenType.SLASH):
            op = self._advance().value
            right = self._parse_unary()
            node = BinOp(op=op, left=node, right=right)
        return node

    def _parse_unary(self) -> ASTNode:
        if self._check(TokenType.MINUS):
            op = self._advance().value
            operand = self._parse_unary()
            return UnaryOp(op=op, operand=operand)
        return self._parse_primary()

    def _parse_primary(self) -> ASTNode:
        tok = self._current()

        if tok.type == TokenType.INTEGER:
            self._advance()
            return NumLiteral(value=tok.value)

        if tok.type == TokenType.FLOAT:
            self._advance()
            return NumLiteral(value=tok.value)

        if tok.type == TokenType.ID:
            self._advance()
            return Identifier(name=tok.value)

        if tok.type == TokenType.LPAREN:
            self._advance()
            node = self._parse_expr()
            self._expect(TokenType.RPAREN)
            return node

        raise ParseError("Expected an expression (literal, identifier, or '(')", tok)


# ---------------------------------------------------------------------------
# Convenience function
# ---------------------------------------------------------------------------

def parse(source: str) -> Program:
    """Lex *source* and parse it, returning a Program AST node."""
    tokens = Lexer(source).tokenize()
    return Parser(tokens).parse()
