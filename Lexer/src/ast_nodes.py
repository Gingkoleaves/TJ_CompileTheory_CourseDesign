"""
Abstract Syntax Tree (AST) Node Definitions
--------------------------------------------
Each class represents one grammatical construct in MiniLang.
"""

from dataclasses import dataclass, field
from typing import List, Optional


class ASTNode:
    """Base class for all AST nodes."""
    pass


# ------------------------------------------------------------------
# Program
# ------------------------------------------------------------------

@dataclass
class Program(ASTNode):
    statements: List[ASTNode]


# ------------------------------------------------------------------
# Statements
# ------------------------------------------------------------------

@dataclass
class VarDecl(ASTNode):
    """int <name> [= <init>];"""
    name: str
    init: Optional[ASTNode]
    line: int


@dataclass
class Assign(ASTNode):
    """<name> = <value>;"""
    name: str
    value: ASTNode
    line: int


@dataclass
class Block(ASTNode):
    """{ <statements> }"""
    statements: List[ASTNode]


@dataclass
class If(ASTNode):
    """if (<condition>) <then_block> [else <else_block>]"""
    condition: ASTNode
    then_block: ASTNode
    else_block: Optional[ASTNode]


@dataclass
class While(ASTNode):
    """while (<condition>) <body>"""
    condition: ASTNode
    body: ASTNode


@dataclass
class For(ASTNode):
    """for (<init>; <condition>; <update>) <body>"""
    init: Optional[ASTNode]
    condition: Optional[ASTNode]
    update: Optional[ASTNode]
    body: ASTNode


@dataclass
class Print(ASTNode):
    """print(<expr>);"""
    expr: ASTNode


@dataclass
class Return(ASTNode):
    """return [<expr>];"""
    expr: Optional[ASTNode]
    line: int


# ------------------------------------------------------------------
# Expressions
# ------------------------------------------------------------------

@dataclass
class BinOp(ASTNode):
    """<left> <op> <right>"""
    op: str
    left: ASTNode
    right: ASTNode
    line: int


@dataclass
class UnaryOp(ASTNode):
    """<op> <operand>"""
    op: str
    operand: ASTNode
    line: int


@dataclass
class Number(ASTNode):
    """Integer literal."""
    value: int
    line: int


@dataclass
class Var(ASTNode):
    """Variable reference."""
    name: str
    line: int
