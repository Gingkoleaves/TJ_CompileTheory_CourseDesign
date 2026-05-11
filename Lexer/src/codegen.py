"""
Intermediate Code Generator (Three-Address Code / Quadruples)
-------------------------------------------------------------
Translates the AST into a sequence of quadruples of the form:

    (operator, arg1, arg2, result)

where '_' is used as a placeholder for unused fields.

Example quadruples:
    (=,  5,   _,  x)        x = 5
    (+,  x,   3,  t1)       t1 = x + 3
    (jz, t1,  _,  L1)       if t1 == 0 goto L1
    (jmp, _,  _,  L2)       goto L2
    (label, _, _, L1)       L1:
    (print, t1, _, _)       print t1
"""

from .ast_nodes import (
    ASTNode, Program, VarDecl, Assign, Block, If, While, For,
    Print, Return, BinOp, UnaryOp, Number, Var,
)


class Quadruple:
    """A single three-address-code instruction."""

    def __init__(self, op: str, arg1: str, arg2: str, result: str):
        self.op = op
        self.arg1 = arg1
        self.arg2 = arg2
        self.result = result

    def __repr__(self):
        return f"({self.op}, {self.arg1}, {self.arg2}, {self.result})"


class CodeGenerator:
    """
    Generates a flat list of quadruples from an AST.

    Usage::

        gen = CodeGenerator()
        gen.generate(ast)
        print(gen.get_code())
    """

    def __init__(self):
        self.quads: list = []
        self._temp_count = 0
        self._label_count = 0

    # ------------------------------------------------------------------
    # Name factories
    # ------------------------------------------------------------------

    def _new_temp(self) -> str:
        self._temp_count += 1
        return f"t{self._temp_count}"

    def _new_label(self) -> str:
        self._label_count += 1
        return f"L{self._label_count}"

    # ------------------------------------------------------------------
    # Emission
    # ------------------------------------------------------------------

    def _emit(self, op: str, arg1: str = "_", arg2: str = "_", result: str = "_") -> Quadruple:
        q = Quadruple(op, arg1, arg2, result)
        self.quads.append(q)
        return q

    # ------------------------------------------------------------------
    # Visitor dispatch
    # ------------------------------------------------------------------

    def generate(self, node: ASTNode) -> str:
        """Generate code for *node* and return the place (temp/var name) that holds
        the value when *node* is an expression, or an empty string otherwise."""
        method_name = f"_gen_{type(node).__name__}"
        gen = getattr(self, method_name, self._generic_gen)
        return gen(node)

    def _generic_gen(self, node: ASTNode) -> str:
        return ""

    # ------------------------------------------------------------------
    # Statement generators
    # ------------------------------------------------------------------

    def _gen_Program(self, node: Program) -> str:
        for stmt in node.statements:
            self.generate(stmt)
        return ""

    def _gen_VarDecl(self, node: VarDecl) -> str:
        if node.init is not None:
            place = self.generate(node.init)
            self._emit("=", place, "_", node.name)
        else:
            self._emit("=", "0", "_", node.name)
        return ""

    def _gen_Assign(self, node: Assign) -> str:
        place = self.generate(node.value)
        self._emit("=", place, "_", node.name)
        return ""

    def _gen_Block(self, node: Block) -> str:
        for stmt in node.statements:
            self.generate(stmt)
        return ""

    def _gen_If(self, node: If) -> str:
        cond = self.generate(node.condition)
        false_label = self._new_label()
        end_label = self._new_label()

        self._emit("jz", cond, "_", false_label)
        self.generate(node.then_block)
        if node.else_block is not None:
            self._emit("jmp", "_", "_", end_label)
        self._emit("label", "_", "_", false_label)
        if node.else_block is not None:
            self.generate(node.else_block)
            self._emit("label", "_", "_", end_label)
        return ""

    def _gen_While(self, node: While) -> str:
        start_label = self._new_label()
        end_label = self._new_label()

        self._emit("label", "_", "_", start_label)
        cond = self.generate(node.condition)
        self._emit("jz", cond, "_", end_label)
        self.generate(node.body)
        self._emit("jmp", "_", "_", start_label)
        self._emit("label", "_", "_", end_label)
        return ""

    def _gen_For(self, node: For) -> str:
        start_label = self._new_label()
        end_label = self._new_label()

        if node.init is not None:
            self.generate(node.init)
        self._emit("label", "_", "_", start_label)
        if node.condition is not None:
            cond = self.generate(node.condition)
            self._emit("jz", cond, "_", end_label)
        self.generate(node.body)
        if node.update is not None:
            self.generate(node.update)
        self._emit("jmp", "_", "_", start_label)
        self._emit("label", "_", "_", end_label)
        return ""

    def _gen_Print(self, node: Print) -> str:
        place = self.generate(node.expr)
        self._emit("print", place, "_", "_")
        return ""

    def _gen_Return(self, node: Return) -> str:
        if node.expr is not None:
            place = self.generate(node.expr)
            self._emit("return", place, "_", "_")
        else:
            self._emit("return", "_", "_", "_")
        return ""

    # ------------------------------------------------------------------
    # Expression generators
    # ------------------------------------------------------------------

    def _gen_BinOp(self, node: BinOp) -> str:
        left = self.generate(node.left)
        right = self.generate(node.right)
        t = self._new_temp()
        self._emit(node.op, left, right, t)
        return t

    def _gen_UnaryOp(self, node: UnaryOp) -> str:
        operand = self.generate(node.operand)
        t = self._new_temp()
        self._emit(f"u{node.op}", operand, "_", t)
        return t

    def _gen_Number(self, node: Number) -> str:
        return str(node.value)

    def _gen_Var(self, node: Var) -> str:
        return node.name

    # ------------------------------------------------------------------
    # Formatting
    # ------------------------------------------------------------------

    def get_code(self) -> str:
        """Return a human-readable listing of all generated quadruples."""
        lines = []
        for i, q in enumerate(self.quads):
            lines.append(f"{i:4d}: {q}")
        return "\n".join(lines)
