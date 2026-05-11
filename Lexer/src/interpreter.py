"""
Tree-Walking Interpreter
------------------------
Directly executes the AST produced by the parser.

All values are Python ``int``s (0 = false, non-zero = true for booleans).
Division (/) performs integer (floor) division.
"""

from .ast_nodes import (
    ASTNode, Program, VarDecl, Assign, Block, If, While, For,
    Print, Return, BinOp, UnaryOp, Number, Var,
)


class RuntimeError(Exception):
    pass


class _ReturnSignal(Exception):
    """Internal signal used to implement ``return`` statements."""
    def __init__(self, value):
        self.value = value


class Environment:
    """
    Scoped variable store implemented as a linked chain of dicts.

    * ``define`` creates a new binding in the *current* scope.
    * ``set``    updates an existing binding, searching parent scopes.
    * ``get``    retrieves a value, searching parent scopes.
    """

    def __init__(self, parent: "Environment | None" = None):
        self._vars: dict = {}
        self.parent = parent

    def define(self, name: str, value):
        self._vars[name] = value

    def set(self, name: str, value):
        if name in self._vars:
            self._vars[name] = value
        elif self.parent is not None:
            self.parent.set(name, value)
        else:
            raise RuntimeError(f"Assignment to undeclared variable {name!r}")

    def get(self, name: str):
        if name in self._vars:
            return self._vars[name]
        if self.parent is not None:
            return self.parent.get(name)
        raise RuntimeError(f"Undefined variable {name!r}")


class Interpreter:
    """
    Executes a MiniLang AST.

    Usage::

        interp = Interpreter()
        interp.execute(ast)
        # printed output also available in interp.output (list of strings)
    """

    def __init__(self):
        self.env = Environment()
        self.output: list = []   # collects every print() value as a string

    # ------------------------------------------------------------------
    # Visitor dispatch
    # ------------------------------------------------------------------

    def execute(self, node: ASTNode):
        method_name = f"_exec_{type(node).__name__}"
        executor = getattr(self, method_name, None)
        if executor is None:
            raise RuntimeError(f"Unknown node type: {type(node).__name__}")
        return executor(node)

    def _eval(self, node: ASTNode) -> int:
        method_name = f"_eval_{type(node).__name__}"
        evaluator = getattr(self, method_name, None)
        if evaluator is None:
            raise RuntimeError(f"Cannot evaluate node type: {type(node).__name__}")
        return evaluator(node)

    # ------------------------------------------------------------------
    # Statement executors
    # ------------------------------------------------------------------

    def _exec_Program(self, node: Program):
        for stmt in node.statements:
            self.execute(stmt)

    def _exec_VarDecl(self, node: VarDecl):
        value = 0 if node.init is None else self._eval(node.init)
        self.env.define(node.name, value)

    def _exec_Assign(self, node: Assign):
        value = self._eval(node.value)
        self.env.set(node.name, value)

    def _exec_Block(self, node: Block):
        saved = self.env
        self.env = Environment(saved)
        try:
            for stmt in node.statements:
                self.execute(stmt)
        finally:
            self.env = saved

    def _exec_If(self, node: If):
        if self._eval(node.condition):
            self.execute(node.then_block)
        elif node.else_block is not None:
            self.execute(node.else_block)

    def _exec_While(self, node: While):
        while self._eval(node.condition):
            self.execute(node.body)

    def _exec_For(self, node: For):
        saved = self.env
        self.env = Environment(saved)
        try:
            if node.init is not None:
                self.execute(node.init)
            while True:
                if node.condition is not None and not self._eval(node.condition):
                    break
                self.execute(node.body)
                if node.update is not None:
                    self.execute(node.update)
        finally:
            self.env = saved

    def _exec_Print(self, node: Print):
        value = self._eval(node.expr)
        text = str(value)
        self.output.append(text)
        print(value)

    def _exec_Return(self, node: Return):
        value = None if node.expr is None else self._eval(node.expr)
        raise _ReturnSignal(value)

    # ------------------------------------------------------------------
    # Expression evaluators
    # ------------------------------------------------------------------

    def _eval_Number(self, node: Number) -> int:
        return node.value

    def _eval_Var(self, node: Var) -> int:
        return self.env.get(node.name)

    def _eval_BinOp(self, node: BinOp) -> int:
        left = self._eval(node.left)
        right = self._eval(node.right)
        op = node.op
        if op == "+":
            return left + right
        if op == "-":
            return left - right
        if op == "*":
            return left * right
        if op == "/":
            if right == 0:
                raise RuntimeError(f"Division by zero at line {node.line}")
            return left // right
        if op == "%":
            if right == 0:
                raise RuntimeError(f"Modulo by zero at line {node.line}")
            return left % right
        if op == "==":
            return int(left == right)
        if op == "!=":
            return int(left != right)
        if op == "<":
            return int(left < right)
        if op == "<=":
            return int(left <= right)
        if op == ">":
            return int(left > right)
        if op == ">=":
            return int(left >= right)
        if op == "&&":
            return int(bool(left) and bool(right))
        if op == "||":
            return int(bool(left) or bool(right))
        raise RuntimeError(f"Unknown operator: {op!r}")

    def _eval_UnaryOp(self, node: UnaryOp) -> int:
        val = self._eval(node.operand)
        if node.op == "-":
            return -val
        if node.op == "!":
            return int(not val)
        raise RuntimeError(f"Unknown unary operator: {node.op!r}")
