"""
Semantic Analyzer
-----------------
Performs a single pass over the AST to detect semantic errors:
  - Use of undeclared variables
  - Redeclaration of a variable in the same scope

Errors are collected in ``self.errors`` (list of strings); the analyzer
does *not* raise on the first error so that all errors are reported.
"""

from .ast_nodes import (
    ASTNode, Program, VarDecl, Assign, Block, If, While, For,
    Print, Return, BinOp, UnaryOp, Number, Var,
)


class SemanticError(Exception):
    pass


class SymbolTable:
    """Scoped symbol table implemented as a linked chain of dicts."""

    def __init__(self, parent: "SymbolTable | None" = None):
        self._table: dict = {}
        self.parent = parent

    def define(self, name: str, var_type: str, line: int):
        if name in self._table:
            raise SemanticError(
                f"Variable {name!r} already declared in this scope (line {line})"
            )
        self._table[name] = var_type

    def lookup(self, name: str) -> "str | None":
        if name in self._table:
            return self._table[name]
        if self.parent is not None:
            return self.parent.lookup(name)
        return None


class SemanticAnalyzer:
    """
    Visitor-style semantic analyzer.

    Usage::

        analyzer = SemanticAnalyzer()
        analyzer.analyze(ast)
        if analyzer.errors:
            for msg in analyzer.errors:
                print(msg)
    """

    def __init__(self):
        self.scope = SymbolTable()
        self.errors: list = []

    # ------------------------------------------------------------------
    # Visitor dispatch
    # ------------------------------------------------------------------

    def analyze(self, node: ASTNode):
        method_name = f"_visit_{type(node).__name__}"
        visitor = getattr(self, method_name, self._generic_visit)
        visitor(node)

    def _generic_visit(self, node: ASTNode):
        pass

    def _error(self, msg: str):
        self.errors.append(msg)

    # ------------------------------------------------------------------
    # Visitors
    # ------------------------------------------------------------------

    def _visit_Program(self, node: Program):
        for stmt in node.statements:
            self.analyze(stmt)

    def _visit_VarDecl(self, node: VarDecl):
        if node.init is not None:
            self.analyze(node.init)
        try:
            self.scope.define(node.name, "int", node.line)
        except SemanticError as exc:
            self._error(str(exc))

    def _visit_Assign(self, node: Assign):
        if self.scope.lookup(node.name) is None:
            self._error(
                f"Assignment to undeclared variable {node.name!r} at line {node.line}"
            )
        self.analyze(node.value)

    def _visit_Block(self, node: Block):
        saved = self.scope
        self.scope = SymbolTable(saved)
        for stmt in node.statements:
            self.analyze(stmt)
        self.scope = saved

    def _visit_If(self, node: If):
        self.analyze(node.condition)
        self.analyze(node.then_block)
        if node.else_block is not None:
            self.analyze(node.else_block)

    def _visit_While(self, node: While):
        self.analyze(node.condition)
        self.analyze(node.body)

    def _visit_For(self, node: For):
        saved = self.scope
        self.scope = SymbolTable(saved)
        if node.init is not None:
            self.analyze(node.init)
        if node.condition is not None:
            self.analyze(node.condition)
        if node.update is not None:
            self.analyze(node.update)
        # body is a Block; visit_Block will push another nested scope
        self.analyze(node.body)
        self.scope = saved

    def _visit_Print(self, node: Print):
        self.analyze(node.expr)

    def _visit_Return(self, node: Return):
        if node.expr is not None:
            self.analyze(node.expr)

    def _visit_BinOp(self, node: BinOp):
        self.analyze(node.left)
        self.analyze(node.right)

    def _visit_UnaryOp(self, node: UnaryOp):
        self.analyze(node.operand)

    def _visit_Number(self, node: Number):
        pass

    def _visit_Var(self, node: Var):
        if self.scope.lookup(node.name) is None:
            self._error(
                f"Use of undeclared variable {node.name!r} at line {node.line}"
            )
