"""
Recursive-Descent Parser
------------------------
Transforms the flat token list produced by the Lexer into an AST.

Grammar (simplified):
    program        = statement*
    statement      = var_decl | assign_stmt | if_stmt | while_stmt
                   | for_stmt | print_stmt | return_stmt | block
    var_decl       = 'int' IDENT ['=' expr] ';'
    assign_stmt    = IDENT '=' expr ';'
    if_stmt        = 'if' '(' expr ')' block ['else' (if_stmt | block)]
    while_stmt     = 'while' '(' expr ')' block
    for_stmt       = 'for' '(' for_init ';' [expr] ';' [assign_no_semi] ')' block
    for_init       = var_decl | assign_stmt | <empty>
    print_stmt     = 'print' '(' expr ')' ';'
    return_stmt    = 'return' [expr] ';'
    block          = '{' statement* '}'
    expr           = or_expr
    or_expr        = and_expr  ('||' and_expr)*
    and_expr       = equality  ('&&' equality)*
    equality       = relational (('==' | '!=') relational)*
    relational     = additive  (('<' | '<=' | '>' | '>=') additive)*
    additive       = multiplicative (('+' | '-') multiplicative)*
    multiplicative = unary (('*' | '/' | '%') unary)*
    unary          = ('-' | '!') unary | primary
    primary        = NUMBER | IDENT | '(' expr ')'
"""

from .lexer import Token, TokenType
from .ast_nodes import (
    ASTNode, Program, VarDecl, Assign, Block, If, While, For,
    Print, Return, BinOp, UnaryOp, Number, Var,
)


class ParseError(Exception):
    pass


class Parser:
    """
    Recursive-descent parser for MiniLang.

    Usage::

        from src.lexer import Lexer
        from src.parser import Parser

        tokens = Lexer(source).tokenize()
        ast = Parser(tokens).parse()
    """

    def __init__(self, tokens: list):
        self.tokens = tokens
        self.pos = 0

    # ------------------------------------------------------------------
    # Navigation helpers
    # ------------------------------------------------------------------

    def _current(self) -> Token:
        return self.tokens[self.pos]

    def _peek_ahead(self, offset: int = 1) -> Token:
        idx = self.pos + offset
        if idx < len(self.tokens):
            return self.tokens[idx]
        return self.tokens[-1]  # EOF

    def _advance(self) -> Token:
        tok = self.tokens[self.pos]
        if self.pos < len(self.tokens) - 1:
            self.pos += 1
        return tok

    def _expect(self, token_type: TokenType) -> Token:
        tok = self._current()
        if tok.type != token_type:
            raise ParseError(
                f"Expected {token_type.value!r} at line {tok.line}, col {tok.col}, "
                f"got {tok.type.value!r} ({tok.value!r})"
            )
        return self._advance()

    def _match(self, *types: TokenType) -> bool:
        return self._current().type in types

    # ------------------------------------------------------------------
    # Top-level
    # ------------------------------------------------------------------

    def parse(self) -> Program:
        """Parse the full program and return its AST."""
        stmts = []
        while not self._match(TokenType.EOF):
            stmts.append(self._parse_statement())
        return Program(stmts)

    # ------------------------------------------------------------------
    # Statements
    # ------------------------------------------------------------------

    def _parse_statement(self) -> ASTNode:
        tok = self._current()
        if tok.type == TokenType.INT:
            return self._parse_var_decl()
        if tok.type == TokenType.IF:
            return self._parse_if()
        if tok.type == TokenType.WHILE:
            return self._parse_while()
        if tok.type == TokenType.FOR:
            return self._parse_for()
        if tok.type == TokenType.PRINT:
            return self._parse_print()
        if tok.type == TokenType.RETURN:
            return self._parse_return()
        if tok.type == TokenType.LBRACE:
            return self._parse_block()
        if tok.type == TokenType.IDENT:
            return self._parse_assign_stmt()
        raise ParseError(
            f"Unexpected token {tok.type.value!r} at line {tok.line}, col {tok.col}"
        )

    def _parse_var_decl(self) -> VarDecl:
        line = self._current().line
        self._expect(TokenType.INT)
        name = self._expect(TokenType.IDENT).value
        init = None
        if self._match(TokenType.ASSIGN):
            self._advance()
            init = self._parse_expr()
        self._expect(TokenType.SEMICOLON)
        return VarDecl(name, init, line)

    def _parse_assign_stmt(self) -> Assign:
        line = self._current().line
        name = self._expect(TokenType.IDENT).value
        self._expect(TokenType.ASSIGN)
        value = self._parse_expr()
        self._expect(TokenType.SEMICOLON)
        return Assign(name, value, line)

    def _parse_assign_no_semi(self) -> Assign:
        """Assignment without a trailing semicolon (used in for-loop update)."""
        line = self._current().line
        name = self._expect(TokenType.IDENT).value
        self._expect(TokenType.ASSIGN)
        value = self._parse_expr()
        return Assign(name, value, line)

    def _parse_if(self) -> If:
        self._expect(TokenType.IF)
        self._expect(TokenType.LPAREN)
        cond = self._parse_expr()
        self._expect(TokenType.RPAREN)
        then_block = self._parse_block()
        else_block = None
        if self._match(TokenType.ELSE):
            self._advance()
            if self._match(TokenType.IF):
                else_block = self._parse_if()
            else:
                else_block = self._parse_block()
        return If(cond, then_block, else_block)

    def _parse_while(self) -> While:
        self._expect(TokenType.WHILE)
        self._expect(TokenType.LPAREN)
        cond = self._parse_expr()
        self._expect(TokenType.RPAREN)
        body = self._parse_block()
        return While(cond, body)

    def _parse_for(self) -> For:
        """for ( [init]; [cond]; [update] ) block"""
        self._expect(TokenType.FOR)
        self._expect(TokenType.LPAREN)

        # init clause
        init = None
        if self._match(TokenType.INT):
            init = self._parse_var_decl()       # consumes its own ';'
        elif self._match(TokenType.IDENT):
            init = self._parse_assign_stmt()    # consumes its own ';'
        else:
            self._expect(TokenType.SEMICOLON)   # empty init

        # condition clause
        cond = None
        if not self._match(TokenType.SEMICOLON):
            cond = self._parse_expr()
        self._expect(TokenType.SEMICOLON)

        # update clause
        update = None
        if not self._match(TokenType.RPAREN):
            update = self._parse_assign_no_semi()
        self._expect(TokenType.RPAREN)

        body = self._parse_block()
        return For(init, cond, update, body)

    def _parse_print(self) -> Print:
        self._expect(TokenType.PRINT)
        self._expect(TokenType.LPAREN)
        expr = self._parse_expr()
        self._expect(TokenType.RPAREN)
        self._expect(TokenType.SEMICOLON)
        return Print(expr)

    def _parse_return(self) -> Return:
        line = self._current().line
        self._expect(TokenType.RETURN)
        expr = None
        if not self._match(TokenType.SEMICOLON):
            expr = self._parse_expr()
        self._expect(TokenType.SEMICOLON)
        return Return(expr, line)

    def _parse_block(self) -> Block:
        self._expect(TokenType.LBRACE)
        stmts = []
        while not self._match(TokenType.RBRACE, TokenType.EOF):
            stmts.append(self._parse_statement())
        self._expect(TokenType.RBRACE)
        return Block(stmts)

    # ------------------------------------------------------------------
    # Expressions  (precedence climbing via recursive descent)
    # ------------------------------------------------------------------

    def _parse_expr(self) -> ASTNode:
        return self._parse_or()

    def _parse_or(self) -> ASTNode:
        left = self._parse_and()
        while self._match(TokenType.OR):
            line = self._current().line
            op = self._advance().value
            right = self._parse_and()
            left = BinOp(op, left, right, line)
        return left

    def _parse_and(self) -> ASTNode:
        left = self._parse_equality()
        while self._match(TokenType.AND):
            line = self._current().line
            op = self._advance().value
            right = self._parse_equality()
            left = BinOp(op, left, right, line)
        return left

    def _parse_equality(self) -> ASTNode:
        left = self._parse_relational()
        while self._match(TokenType.EQ, TokenType.NEQ):
            line = self._current().line
            op = self._advance().value
            right = self._parse_relational()
            left = BinOp(op, left, right, line)
        return left

    def _parse_relational(self) -> ASTNode:
        left = self._parse_additive()
        while self._match(TokenType.LT, TokenType.LE, TokenType.GT, TokenType.GE):
            line = self._current().line
            op = self._advance().value
            right = self._parse_additive()
            left = BinOp(op, left, right, line)
        return left

    def _parse_additive(self) -> ASTNode:
        left = self._parse_multiplicative()
        while self._match(TokenType.PLUS, TokenType.MINUS):
            line = self._current().line
            op = self._advance().value
            right = self._parse_multiplicative()
            left = BinOp(op, left, right, line)
        return left

    def _parse_multiplicative(self) -> ASTNode:
        left = self._parse_unary()
        while self._match(TokenType.STAR, TokenType.SLASH, TokenType.MOD):
            line = self._current().line
            op = self._advance().value
            right = self._parse_unary()
            left = BinOp(op, left, right, line)
        return left

    def _parse_unary(self) -> ASTNode:
        if self._match(TokenType.MINUS, TokenType.NOT):
            line = self._current().line
            op = self._advance().value
            operand = self._parse_unary()
            return UnaryOp(op, operand, line)
        return self._parse_primary()

    def _parse_primary(self) -> ASTNode:
        tok = self._current()
        if tok.type == TokenType.NUMBER:
            self._advance()
            return Number(int(tok.value), tok.line)
        if tok.type == TokenType.IDENT:
            self._advance()
            return Var(tok.value, tok.line)
        if tok.type == TokenType.LPAREN:
            self._advance()
            expr = self._parse_expr()
            self._expect(TokenType.RPAREN)
            return expr
        raise ParseError(
            f"Unexpected token {tok.type.value!r} at line {tok.line}, col {tok.col}"
        )
