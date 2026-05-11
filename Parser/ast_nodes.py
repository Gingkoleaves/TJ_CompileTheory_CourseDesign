"""
Abstract Syntax Tree (AST) Node Definitions

Each class represents a node in the AST produced by the parser.
"""

from dataclasses import dataclass, field
from typing import Optional


# ---------------------------------------------------------------------------
# Base
# ---------------------------------------------------------------------------

class ASTNode:
    """Base class for all AST nodes."""

    def accept(self, visitor):
        """Accept a visitor (Visitor pattern)."""
        method_name = f'visit_{type(self).__name__}'
        method = getattr(visitor, method_name, visitor.generic_visit)
        return method(self)


# ---------------------------------------------------------------------------
# Expressions
# ---------------------------------------------------------------------------

@dataclass
class NumLiteral(ASTNode):
    """An integer or float literal, e.g. 42, 3.14"""
    value: int | float

    def accept(self, visitor):
        return visitor.visit_NumLiteral(self)


@dataclass
class Identifier(ASTNode):
    """A variable reference, e.g. x"""
    name: str

    def accept(self, visitor):
        return visitor.visit_Identifier(self)


@dataclass
class BinOp(ASTNode):
    """A binary operation, e.g. a + b, x == y"""
    op:    str
    left:  ASTNode
    right: ASTNode

    def accept(self, visitor):
        return visitor.visit_BinOp(self)


@dataclass
class UnaryOp(ASTNode):
    """A unary operation, e.g. -x"""
    op:      str
    operand: ASTNode

    def accept(self, visitor):
        return visitor.visit_UnaryOp(self)


# ---------------------------------------------------------------------------
# Statements
# ---------------------------------------------------------------------------

@dataclass
class AssignStmt(ASTNode):
    """Variable assignment, e.g. x = expr;"""
    name:  str
    value: ASTNode

    def accept(self, visitor):
        return visitor.visit_AssignStmt(self)


@dataclass
class DeclStmt(ASTNode):
    """Variable declaration with optional initializer, e.g. int x = 0;"""
    type_:    str
    name:     str
    init: Optional[ASTNode] = None

    def accept(self, visitor):
        return visitor.visit_DeclStmt(self)


@dataclass
class ReturnStmt(ASTNode):
    """Return statement, e.g. return expr;"""
    value: Optional[ASTNode] = None

    def accept(self, visitor):
        return visitor.visit_ReturnStmt(self)


@dataclass
class IfStmt(ASTNode):
    """If / if-else statement."""
    condition:   ASTNode
    then_branch: ASTNode
    else_branch: Optional[ASTNode] = None

    def accept(self, visitor):
        return visitor.visit_IfStmt(self)


@dataclass
class WhileStmt(ASTNode):
    """While loop."""
    condition: ASTNode
    body:      ASTNode

    def accept(self, visitor):
        return visitor.visit_WhileStmt(self)


@dataclass
class Block(ASTNode):
    """A braced block of statements."""
    stmts: list = field(default_factory=list)

    def accept(self, visitor):
        return visitor.visit_Block(self)


@dataclass
class Program(ASTNode):
    """Root node: a sequence of top-level statements."""
    stmts: list = field(default_factory=list)

    def accept(self, visitor):
        return visitor.visit_Program(self)


# ---------------------------------------------------------------------------
# Pretty-printer visitor
# ---------------------------------------------------------------------------

class ASTPrinter:
    """Converts an AST back to a human-readable indented string."""

    def __init__(self):
        self._indent = 0

    def _line(self, text: str) -> str:
        return '  ' * self._indent + text

    def generic_visit(self, node: ASTNode) -> str:
        return self._line(f'<{type(node).__name__}>')

    def visit_Program(self, node: Program) -> str:
        lines = [self._line('Program')]
        self._indent += 1
        for stmt in node.stmts:
            lines.append(stmt.accept(self))
        self._indent -= 1
        return '\n'.join(lines)

    def visit_Block(self, node: Block) -> str:
        lines = [self._line('Block')]
        self._indent += 1
        for stmt in node.stmts:
            lines.append(stmt.accept(self))
        self._indent -= 1
        return '\n'.join(lines)

    def visit_DeclStmt(self, node: DeclStmt) -> str:
        init_str = ''
        if node.init is not None:
            self._indent += 2
            init_str = ' = \n' + node.init.accept(self)
            self._indent -= 2
        return self._line(f'Decl({node.type_} {node.name}{init_str})')

    def visit_AssignStmt(self, node: AssignStmt) -> str:
        lines = [self._line(f'Assign({node.name})')]
        self._indent += 1
        lines.append(node.value.accept(self))
        self._indent -= 1
        return '\n'.join(lines)

    def visit_ReturnStmt(self, node: ReturnStmt) -> str:
        if node.value is None:
            return self._line('Return')
        lines = [self._line('Return')]
        self._indent += 1
        lines.append(node.value.accept(self))
        self._indent -= 1
        return '\n'.join(lines)

    def visit_IfStmt(self, node: IfStmt) -> str:
        lines = [self._line('If')]
        self._indent += 1
        lines.append(self._line('Condition:'))
        self._indent += 1
        lines.append(node.condition.accept(self))
        self._indent -= 1
        lines.append(self._line('Then:'))
        self._indent += 1
        lines.append(node.then_branch.accept(self))
        self._indent -= 1
        if node.else_branch is not None:
            lines.append(self._line('Else:'))
            self._indent += 1
            lines.append(node.else_branch.accept(self))
            self._indent -= 1
        self._indent -= 1
        return '\n'.join(lines)

    def visit_WhileStmt(self, node: WhileStmt) -> str:
        lines = [self._line('While')]
        self._indent += 1
        lines.append(self._line('Condition:'))
        self._indent += 1
        lines.append(node.condition.accept(self))
        self._indent -= 1
        lines.append(self._line('Body:'))
        self._indent += 1
        lines.append(node.body.accept(self))
        self._indent -= 1
        self._indent -= 1
        return '\n'.join(lines)

    def visit_BinOp(self, node: BinOp) -> str:
        lines = [self._line(f'BinOp({node.op})')]
        self._indent += 1
        lines.append(node.left.accept(self))
        lines.append(node.right.accept(self))
        self._indent -= 1
        return '\n'.join(lines)

    def visit_UnaryOp(self, node: UnaryOp) -> str:
        lines = [self._line(f'UnaryOp({node.op})')]
        self._indent += 1
        lines.append(node.operand.accept(self))
        self._indent -= 1
        return '\n'.join(lines)

    def visit_NumLiteral(self, node: NumLiteral) -> str:
        return self._line(f'Num({node.value})')

    def visit_Identifier(self, node: Identifier) -> str:
        return self._line(f'Id({node.name})')
