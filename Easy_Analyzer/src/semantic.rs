//! 语义分析器：遍历 AST，输出语义错误列表与四元式序列。
//!
//! 覆盖 PDF 强制规则（0.1-0.3, 1.1-1.5, 2.0-2.2, 3.1-3.5, 4.1, 5.0, 5.1）
//! 的全部静态语义检查，并对扩展规则做尽力而为的处理与 IR 生成。

use std::collections::HashMap;

use serde::Serialize;

use easy_parser::{
    BinaryOp, Block, ElseBranch, Expr, FunctionDecl, Program, Statement, TypeNode, UnaryOp,
};

use crate::ir::{IrBuilder, Quadruple, PLACEHOLDER};
use crate::symbol::{FunctionSig, SymbolTable, VarSymbol};
use crate::types::{from_node, Type};

#[derive(Debug, Clone, Serialize)]
pub struct SemanticError {
    pub message: String,
}

impl SemanticError {
    pub fn new(message: impl Into<String>) -> Self {
        Self { message: message.into() }
    }
}

/// 表达式求值结果：(类型, 操作数名)。
/// 操作数名是四元式中可被引用的字符串（标识符、临时变量、数字字面量）。
struct ExprValue {
    ty: Type,
    place: String,
}

impl ExprValue {
    fn new(ty: Type, place: impl Into<String>) -> Self {
        Self { ty, place: place.into() }
    }
    fn error() -> Self {
        Self { ty: Type::Error, place: PLACEHOLDER.to_string() }
    }
}

/// 函数级遍历状态：当前函数返回类型、循环嵌套深度。
struct LoopExprCtx {
    result_place: String,
    break_type: Option<Type>,
}

struct LoopLabels {
    /// 回到的标签：while/for 是 cond 标签；loop 是 body 起点标签。
    start: String,
    /// 跳出的标签：循环结构结束位置。
    end: String,
}

struct FuncCtx {
    return_type: Type,
    loop_depth: usize,
    loop_exprs: Vec<LoopExprCtx>,
    loop_labels: Vec<LoopLabels>,
}

#[derive(Default)]
struct BorrowState {
    immutable: usize,
    mutable: usize,
}

pub struct Analyzer {
    table: SymbolTable,
    ir: IrBuilder,
    errors: Vec<SemanticError>,
    func_ctx: Option<FuncCtx>,
    borrow_scopes: Vec<HashMap<String, BorrowState>>,
}

impl Analyzer {
    pub fn new() -> Self {
        Self {
            table: SymbolTable::new(),
            ir: IrBuilder::new(),
            errors: Vec::new(),
            func_ctx: None,
            borrow_scopes: vec![HashMap::new()],
        }
    }

    pub fn finish(self) -> (Vec<SemanticError>, Vec<Quadruple>) {
        (self.errors, self.ir.quads)
    }

    fn error(&mut self, msg: impl Into<String>) {
        self.errors.push(SemanticError::new(msg));
    }

    fn push_scope(&mut self) {
        self.table.push_scope();
        self.borrow_scopes.push(HashMap::new());
    }

    fn pop_scope(&mut self) {
        self.table.pop_scope();
        if self.borrow_scopes.len() > 1 {
            self.borrow_scopes.pop();
        }
    }

    fn register_borrow(&mut self, name: &str, mutable: bool) {
        let existing_immutable = self
            .borrow_scopes
            .iter()
            .map(|scope| scope.get(name).map(|state| state.immutable).unwrap_or(0))
            .sum::<usize>();
        let existing_mutable = self
            .borrow_scopes
            .iter()
            .map(|scope| scope.get(name).map(|state| state.mutable).unwrap_or(0))
            .sum::<usize>();

        if mutable {
            if existing_immutable > 0 || existing_mutable > 0 {
                self.error(format!(
                    "不能在已有引用存在时创建变量 `{}` 的可变引用（规则 6.3）",
                    name
                ));
            }
        } else if existing_mutable > 0 {
            self.error(format!(
                "不能在可变引用存在时创建变量 `{}` 的其他引用（规则 6.3）",
                name
            ));
        }

        if let Some(scope) = self.borrow_scopes.last_mut() {
            let state = scope.entry(name.to_string()).or_default();
            if mutable {
                state.mutable += 1;
            } else {
                state.immutable += 1;
            }
        }
    }

    // ------------------------------------------------------------------
    // 程序与函数
    // ------------------------------------------------------------------

    pub fn analyze_program(&mut self, program: &Program) {
        // 先收集所有函数签名（支持前向调用）。
        // 重复定义保留首次签名；后续同名函数标记跳过，避免 IR 中出现两份同名 FUNC/END_FUNC。
        let mut skip = vec![false; program.functions.len()];
        for (i, f) in program.functions.iter().enumerate() {
            let sig = self.build_function_sig(f);
            if self.table.lookup_function(&sig.name).is_some() {
                self.error(format!("函数 `{}` 重复定义", sig.name));
                skip[i] = true;
            } else {
                self.table.declare_function(sig);
            }
        }
        for (i, f) in program.functions.iter().enumerate() {
            if skip[i] {
                continue;
            }
            self.analyze_function(f);
        }
    }

    fn build_function_sig(&self, f: &FunctionDecl) -> FunctionSig {
        let params = f
            .params
            .iter()
            .map(|b| {
                let ty = b.ty.as_ref().map(from_node).unwrap_or(Type::Error);
                (b.name.clone(), ty, b.mutable)
            })
            .collect::<Vec<_>>();
        let return_type = f
            .return_type
            .as_ref()
            .map(from_node)
            .unwrap_or(Type::Unit);
        FunctionSig {
            name: f.name.clone(),
            params,
            return_type,
        }
    }

    fn analyze_function(&mut self, f: &FunctionDecl) {
        // 形参类型缺失（语法已保证带类型，但稳妥防御）
        for p in &f.params {
            if p.ty.is_none() {
                self.error(format!("函数 `{}` 形参 `{}` 缺少类型", f.name, p.name));
            }
        }

        // 形参重名检查：保留首次出现的形参，重名仅报错并跳过其 PARAM_DECL / declare。
        let mut seen_param_names = std::collections::HashSet::new();
        let mut param_skip = vec![false; f.params.len()];
        for (i, p) in f.params.iter().enumerate() {
            if !seen_param_names.insert(p.name.clone()) {
                self.error(format!("函数 `{}` 形参 `{}` 重名", f.name, p.name));
                param_skip[i] = true;
            }
        }

        let sig = self
            .table
            .lookup_function(&f.name)
            .cloned()
            .expect("函数签名应已登记");
        let return_type = sig.return_type.clone();

        // 生成函数头
        self.ir.emit("FUNC", &f.name, PLACEHOLDER, PLACEHOLDER);
        for (i, (pname, pty, _pmut)) in sig.params.iter().enumerate() {
            if param_skip[i] {
                continue;
            }
            self.ir
                .emit("PARAM_DECL", pname.clone(), pty.display(), PLACEHOLDER);
        }

        // 进入函数作用域，登记形参
        self.push_scope();
        for (i, (pname, pty, pmut)) in sig.params.iter().enumerate() {
            if param_skip[i] {
                continue;
            }
            self.table.declare(VarSymbol::new(
                pname.clone(),
                pty.clone(),
                *pmut,
                true,
            ));
        }

        // 进入函数级上下文
        let prev = self.func_ctx.replace(FuncCtx {
            return_type: return_type.clone(),
            loop_depth: 0,
            loop_exprs: Vec::new(),
            loop_labels: Vec::new(),
        });

        // 函数体（不再额外 push_scope，因为形参与本块同作用域）
        for stmt in &f.body.statements {
            self.gen_stmt(stmt);
        }
        // 末尾表达式（rule 7.2）：等价 return
        if let Some(tail) = &f.body.tail {
            let v = self.gen_expr(tail);
            if !v.ty.compatible(&return_type) && v.ty.is_known() && return_type.is_known() {
                self.error(format!(
                    "函数 `{}` 末尾表达式类型 {} 与返回类型 {} 不一致",
                    f.name,
                    v.ty.display(),
                    return_type.display()
                ));
            }
            self.ir.emit("RETURN", v.place, PLACEHOLDER, PLACEHOLDER);
        } else {
            // 无尾表达式时始终发射 RETURN 作为函数终结子，便于下游解释/翻译。
            // 非 Unit 返回类型的源码若所有路径都已通过显式 return 退出，
            // 该终结子不可达；若存在 fall-through 路径，则该终结子被执行（值未定义）。
            // 这里不做控制流分析，不额外报错。
            self.ir
                .emit("RETURN", PLACEHOLDER, PLACEHOLDER, PLACEHOLDER);
        }

        self.check_current_scope_uninferred();
        self.func_ctx = prev;
        self.pop_scope();
        self.ir.emit("END_FUNC", &f.name, PLACEHOLDER, PLACEHOLDER);
    }

    // ------------------------------------------------------------------
    // 语句
    // ------------------------------------------------------------------

    fn gen_stmt(&mut self, stmt: &Statement) {
        match stmt {
            Statement::Empty => {}
            Statement::Let { binding, init } => self.gen_let(binding, init.as_deref()),
            Statement::Assign { target, value } => self.gen_assign(target, value),
            Statement::Expr { expr } => {
                self.gen_expr(expr);
            }
            Statement::Return { value } => self.gen_return(value.as_deref()),
            Statement::While { condition, body } => self.gen_while(condition, body),
            Statement::For { binding, iterable, body } => self.gen_for(binding, iterable, body),
            Statement::Break { value } => self.gen_break(value.as_deref()),
            Statement::Continue => self.gen_continue(),
        }
    }

    fn gen_let(&mut self, binding: &easy_parser::Binding, init: Option<&Expr>) {
        // 声明类型
        let declared_ty = binding.ty.as_ref().map(from_node);

        let (final_ty, initialized) = if let Some(expr) = init {
            let v = self.gen_expr(expr);
            let init_ty = v.ty.clone();
            let ty = match (&declared_ty, &init_ty) {
                (Some(decl), expr_ty) => {
                    if !decl.compatible(expr_ty)
                        && decl.is_known()
                        && expr_ty.is_known()
                    {
                        self.error(format!(
                            "变量 `{}` 声明类型 {} 与初始化表达式类型 {} 不匹配",
                            binding.name,
                            decl.display(),
                            expr_ty.display()
                        ));
                    }
                    decl.clone()
                }
                (None, expr_ty) => {
                    // 类型推断（rule 2.3）
                    if matches!(expr_ty, Type::Unit) {
                        self.error(format!(
                            "无法用 `()` 初始化变量 `{}`：函数无返回值不可作为表达式（规则 3.5）",
                            binding.name
                        ));
                        Type::Error
                    } else {
                        expr_ty.clone()
                    }
                }
            };
            // 发出赋值四元式
            self.ir.emit("=", v.place, PLACEHOLDER, &binding.name);
            (ty, true)
        } else {
            // 无初始化
            let ty = declared_ty.unwrap_or(Type::Unknown);
            (ty, false)
        };

        self.table.declare(VarSymbol::new(
            binding.name.clone(),
            final_ty,
            binding.mutable,
            initialized,
        ));
    }

    fn gen_assign(&mut self, target: &Expr, value: &Expr) {
        // 仅强制规则要求支持的 LHS：标识符。
        // 其他 LHS（解引用 / 索引 / 字段）做基础检查后尽力生成 IR。
        let value_val = self.gen_expr(value);

        match target {
            Expr::Identifier { name } => {
                // 先复制需要的字段，避免借用冲突
                let info = self
                    .table
                    .lookup(name)
                    .map(|s| (s.ty.clone(), s.mutable, s.initialized));
                let Some((cur_ty, mutable, initialized)) = info else {
                    self.error(format!("变量 `{}` 未声明（规则 2.2）", name));
                    return;
                };

                // 不可变变量第二次赋值（规则 6.1）
                if initialized && !mutable {
                    self.error(format!(
                        "不可变变量 `{}` 不能再次赋值（规则 6.1）",
                        name
                    ));
                }

                // 类型匹配
                let new_ty = if cur_ty.is_known() {
                    if !cur_ty.compatible(&value_val.ty) && value_val.ty.is_known() {
                        self.error(format!(
                            "变量 `{}` 类型 {} 与表达式类型 {} 不匹配（规则 2.2）",
                            name,
                            cur_ty.display(),
                            value_val.ty.display()
                        ));
                    }
                    cur_ty
                } else {
                    // 之前类型未知，本次赋值确定类型
                    if matches!(value_val.ty, Type::Unit) {
                        self.error(format!(
                            "无法用 `()` 初始化变量 `{}`：函数无返回值不可作为表达式",
                            name
                        ));
                        Type::Error
                    } else {
                        value_val.ty.clone()
                    }
                };

                if let Some(sym) = self.table.lookup_mut(name) {
                    sym.ty = new_ty;
                    sym.initialized = true;
                }
                self.ir.emit("=", value_val.place, PLACEHOLDER, name);
            }
            Expr::Unary { op: UnaryOp::Deref, expr } => {
                // 通过引用赋值（规则 6.4）
                let inner = self.gen_expr(expr);
                match &inner.ty {
                    Type::Ref { mutable: true, inner: _ } => {
                        self.ir.emit("*=", value_val.place, PLACEHOLDER, inner.place);
                    }
                    Type::Ref { mutable: false, .. } => {
                        self.error("不能通过不可变引用修改数据（规则 6.4）".to_string());
                    }
                    other if other.is_known() => {
                        self.error(format!(
                            "无法解引用非引用类型 {}（规则 6.4）",
                            other.display()
                        ));
                    }
                    _ => {}
                }
            }
            Expr::Index { .. } | Expr::Field { .. } => {
                if let Some(name) = root_identifier(target) {
                    if let Some(sym) = self.table.lookup(name) {
                        if !sym.mutable {
                            self.error(format!(
                                "不可变变量 `{}` 的元素不能被赋值（规则 8.3/9.3）",
                                name
                            ));
                        }
                    }
                }
                let tgt = self.gen_expr(target);
                if tgt.ty.is_known() && value_val.ty.is_known() && !tgt.ty.compatible(&value_val.ty) {
                    self.error(format!(
                        "赋值目标类型 {} 与表达式类型 {} 不匹配",
                        tgt.ty.display(),
                        value_val.ty.display()
                    ));
                }
                self.ir.emit("=", value_val.place, PLACEHOLDER, tgt.place);
            }
            _ => {
                self.error("赋值语句左侧不是合法左值".to_string());
            }
        }
    }

    fn gen_return(&mut self, value: Option<&Expr>) {
        // 规则 1.5：返回值类型必须匹配
        let expected = self
            .func_ctx
            .as_ref()
            .map(|c| c.return_type.clone())
            .unwrap_or(Type::Unit);

        match value {
            Some(expr) => {
                let v = self.gen_expr(expr);
                if matches!(expected, Type::Unit) {
                    self.error("函数无返回类型，return 不能带表达式（规则 1.5）".to_string());
                } else if !expected.compatible(&v.ty)
                    && v.ty.is_known()
                    && expected.is_known()
                {
                    self.error(format!(
                        "return 表达式类型 {} 与函数声明返回类型 {} 不一致（规则 1.5）",
                        v.ty.display(),
                        expected.display()
                    ));
                }
                self.ir.emit("RETURN", v.place, PLACEHOLDER, PLACEHOLDER);
            }
            None => {
                if !matches!(expected, Type::Unit) {
                    self.error(format!(
                        "函数声明返回类型 {}，return 必须带表达式（规则 1.5）",
                        expected.display()
                    ));
                }
                self.ir
                    .emit("RETURN", PLACEHOLDER, PLACEHOLDER, PLACEHOLDER);
            }
        }
    }

    fn gen_while(&mut self, condition: &Expr, body: &Block) {
        let label_start = self.ir.new_label();
        let label_end = self.ir.new_label();
        self.ir.emit_label(&label_start);
        let cond = self.gen_expr(condition);
        self.check_condition_type(&cond.ty, "while");
        self.ir.emit_if_false(&cond.place, &label_end);

        if let Some(ctx) = self.func_ctx.as_mut() {
            ctx.loop_depth += 1;
            ctx.loop_labels.push(LoopLabels {
                start: label_start.clone(),
                end: label_end.clone(),
            });
        }
        self.gen_block_stmt(body);
        if let Some(ctx) = self.func_ctx.as_mut() {
            ctx.loop_depth -= 1;
            ctx.loop_labels.pop();
        }

        self.ir.emit_goto(&label_start);
        self.ir.emit_label(&label_end);
    }

    fn gen_for(&mut self, binding: &easy_parser::Binding, iterable: &Expr, body: &Block) {
        // 仅支持 a..b 形式（规则 5.2）。
        let (start_ty, start_place, end_place) = match iterable {
            Expr::Binary { left, op: BinaryOp::Range, right } => {
                let l = self.gen_expr(left);
                let r = self.gen_expr(right);
                if l.ty.is_known() && !l.ty.is_integer() {
                    self.error(format!(
                        "for 迭代结构起点类型 {} 不是整数（规则 5.2）",
                        l.ty.display()
                    ));
                }
                if r.ty.is_known() && !r.ty.is_integer() {
                    self.error(format!(
                        "for 迭代结构终点类型 {} 不是整数（规则 5.2）",
                        r.ty.display()
                    ));
                }
                (Type::I32, l.place, r.place)
            }
            other => {
                let v = self.gen_expr(other);
                self.error(format!(
                    "for 迭代结构必须是范围 `a..b`（实际类型 {}）",
                    v.ty.display()
                ));
                (Type::Error, PLACEHOLDER.to_string(), PLACEHOLDER.to_string())
            }
        };

        self.push_scope();
        // 循环变量
        self.table.declare(VarSymbol::new(
            binding.name.clone(),
            start_ty.clone(),
            binding.mutable,
            true,
        ));
        let label_start = self.ir.new_label();
        let label_end = self.ir.new_label();
        // i = start
        self.ir.emit("=", start_place, PLACEHOLDER, &binding.name);
        self.ir.emit_label(&label_start);
        // t = i < end
        let t = self.ir.new_temp();
        self.ir.emit("<", binding.name.clone(), end_place, &t);
        self.ir.emit_if_false(&t, &label_end);
        if let Some(ctx) = self.func_ctx.as_mut() {
            ctx.loop_depth += 1;
            ctx.loop_labels.push(LoopLabels {
                start: label_start.clone(),
                end: label_end.clone(),
            });
        }
        self.gen_block_stmt(body);
        if let Some(ctx) = self.func_ctx.as_mut() {
            ctx.loop_depth -= 1;
            ctx.loop_labels.pop();
        }
        // i = i + 1
        let t2 = self.ir.new_temp();
        self.ir
            .emit("+", binding.name.clone(), "1".to_string(), &t2);
        self.ir.emit("=", t2, PLACEHOLDER, &binding.name);
        self.ir.emit_goto(&label_start);
        self.ir.emit_label(&label_end);
        self.pop_scope();
    }

    fn gen_break(&mut self, value: Option<&Expr>) {
        let in_loop = self
            .func_ctx
            .as_ref()
            .map(|c| c.loop_depth > 0)
            .unwrap_or(false);
        if !in_loop {
            self.error("`break` 必须位于循环体内（规则 5.4）".to_string());
        }
        let v = value.map(|e| self.gen_expr(e));
        let arg = if let Some(v) = v {
            let mut result_place = None;
            let mut type_error = None;
            if let Some(ctx) = self.func_ctx.as_mut().and_then(|ctx| ctx.loop_exprs.last_mut()) {
                match &ctx.break_type {
                    Some(ty) if ty.is_known() && v.ty.is_known() && !ty.compatible(&v.ty) => {
                        type_error = Some((ty.clone(), v.ty.clone()));
                    }
                    None => ctx.break_type = Some(v.ty.clone()),
                    _ => {}
                }
                result_place = Some(ctx.result_place.clone());
            }
            let has_type_error = type_error.is_some();
            if let Some((expected, actual)) = type_error {
                self.error(format!(
                    "loop 表达式多个 break 类型不一致：{} vs {}（规则 7.4）",
                    expected.display(),
                    actual.display()
                ));
            }
            // 类型不一致时不再发射 = 赋值 IR，避免错误结果污染 loop 结果临时变量。
            if !has_type_error {
                if let Some(result_place) = result_place {
                    self.ir.emit("=", v.place.clone(), PLACEHOLDER, result_place);
                }
            }
            v.place
        } else {
            PLACEHOLDER.to_string()
        };
        let end_label = self
            .func_ctx
            .as_ref()
            .and_then(|c| c.loop_labels.last())
            .map(|l| l.end.clone())
            .unwrap_or_else(|| PLACEHOLDER.to_string());
        self.ir.emit("BREAK", arg, PLACEHOLDER, end_label);
    }

    fn gen_continue(&mut self) {
        let in_loop = self
            .func_ctx
            .as_ref()
            .map(|c| c.loop_depth > 0)
            .unwrap_or(false);
        if !in_loop {
            self.error("`continue` 必须位于循环体内（规则 5.4）".to_string());
        }
        let start_label = self
            .func_ctx
            .as_ref()
            .and_then(|c| c.loop_labels.last())
            .map(|l| l.start.clone())
            .unwrap_or_else(|| PLACEHOLDER.to_string());
        self.ir
            .emit("CONTINUE", PLACEHOLDER, PLACEHOLDER, start_label);
    }

    /// 把一个语句块作为子语句处理（push 新作用域）。
    fn gen_block_stmt(&mut self, block: &Block) {
        self.push_scope();
        for s in &block.statements {
            self.gen_stmt(s);
        }
        if let Some(tail) = &block.tail {
            // 作为语句的块：尾表达式只求值，结果丢弃
            self.gen_expr(tail);
        }
        self.check_current_scope_uninferred();
        self.pop_scope();
    }

    fn check_current_scope_uninferred(&mut self) {
        for name in self.table.current_scope_uninferred_names() {
            self.error(format!("变量 `{}` 无法推断类型（规则 2.1）", name));
        }
    }

    fn check_condition_type(&mut self, ty: &Type, ctx: &str) {
        if !ty.is_known() {
            return;
        }
        if !matches!(ty, Type::Bool | Type::I32) {
            self.error(format!(
                "{} 条件表达式类型 {} 不可作为条件",
                ctx,
                ty.display()
            ));
        }
    }

    // ------------------------------------------------------------------
    // 表达式
    // ------------------------------------------------------------------

    fn gen_expr(&mut self, expr: &Expr) -> ExprValue {
        match expr {
            Expr::Number { value } => ExprValue::new(Type::I32, value.clone()),
            Expr::Identifier { name } => self.gen_identifier(name),
            Expr::Unary { op, expr } => self.gen_unary(*op, expr),
            Expr::Binary { left, op, right } => self.gen_binary(left, *op, right),
            Expr::Call { callee, args } => self.gen_call(callee, args),
            Expr::Index { base, index } => self.gen_index(base, index),
            Expr::Field { base, field } => self.gen_field(base, field),
            Expr::Array { elements } => self.gen_array(elements),
            Expr::Tuple { elements } => self.gen_tuple(elements),
            Expr::Block { block } => self.gen_block_expr(block),
            Expr::If { condition, then_branch, else_branch } => {
                self.gen_if_expr(condition, then_branch, else_branch)
            }
            Expr::Loop { body } => self.gen_loop_expr(body),
        }
    }

    fn gen_identifier(&mut self, name: &str) -> ExprValue {
        let info = self
            .table
            .lookup(name)
            .map(|s| (s.ty.clone(), s.initialized));
        match info {
            None => {
                // 变量查不到时回退到函数表：若是函数名被当作表达式使用，
                // 让后续的 gen_call 等以"类型不匹配"形式报错（PDF 例 program_3_3__4）。
                if self.table.lookup_function(name).is_some() {
                    return ExprValue::new(Type::Function, name.to_string());
                }
                self.error(format!("变量 `{}` 未声明（规则 2.2）", name));
                ExprValue::error()
            }
            Some((ty, initialized)) => {
                if !initialized {
                    self.error(format!(
                        "变量 `{}` 在赋值前被使用（规则 2.2）",
                        name
                    ));
                }
                ExprValue::new(ty, name.to_string())
            }
        }
    }

    fn gen_unary(&mut self, op: UnaryOp, inner: &Expr) -> ExprValue {
        let v = self.gen_expr(inner);
        match op {
            UnaryOp::Neg => {
                if v.ty.is_known() && !v.ty.is_integer() {
                    self.error(format!(
                        "一元 `-` 仅支持 i32，实际类型 {}",
                        v.ty.display()
                    ));
                }
                let t = self.ir.new_temp();
                self.ir.emit("NEG", v.place, PLACEHOLDER, &t);
                ExprValue::new(Type::I32, t)
            }
            UnaryOp::Ref => {
                if let Some(name) = root_identifier(inner) {
                    self.register_borrow(name, false);
                }
                ExprValue::new(
                    Type::Ref { mutable: false, inner: Box::new(v.ty) },
                    format!("&{}", v.place),
                )
            }
            UnaryOp::RefMut => {
                if let Some(name) = root_identifier(inner) {
                    if let Some(sym) = self.table.lookup(name) {
                        if !sym.mutable {
                            self.error(format!(
                                "不能对不可变变量 `{}` 创建可变引用（规则 6.3）",
                                name
                            ));
                        }
                    }
                    self.register_borrow(name, true);
                }
                ExprValue::new(
                    Type::Ref { mutable: true, inner: Box::new(v.ty) },
                    format!("&mut {}", v.place),
                )
            }
            UnaryOp::Deref => match v.ty {
                Type::Ref { inner, .. } => {
                    let t = self.ir.new_temp();
                    self.ir.emit("DEREF", v.place, PLACEHOLDER, &t);
                    ExprValue::new(*inner, t)
                }
                Type::Error => ExprValue::error(),
                other => {
                    if other.is_known() {
                        self.error(format!(
                            "无法解引用非引用类型 {}（规则 6.4）",
                            other.display()
                        ));
                    }
                    ExprValue::error()
                }
            },
        }
    }

    fn gen_binary(&mut self, left: &Expr, op: BinaryOp, right: &Expr) -> ExprValue {
        if matches!(op, BinaryOp::Range) {
            let _ = self.gen_expr(left);
            let _ = self.gen_expr(right);
            return ExprValue::new(Type::Range, PLACEHOLDER.to_string());
        }
        let l = self.gen_expr(left);
        let r = self.gen_expr(right);
        let (op_str, is_cmp) = bin_op_info(op);
        // 类型检查：算术与比较都要求 i32
        let both_known = l.ty.is_known() && r.ty.is_known();
        if both_known && (!l.ty.is_integer() || !r.ty.is_integer()) {
            self.error(format!(
                "运算 `{}` 仅支持 i32，实际类型 {} 与 {}",
                op_str,
                l.ty.display(),
                r.ty.display()
            ));
        }
        let t = self.ir.new_temp();
        self.ir.emit(op_str, l.place, r.place, &t);
        let result_ty = if is_cmp { Type::Bool } else { Type::I32 };
        ExprValue::new(result_ty, t)
    }

    fn gen_call(&mut self, callee: &Expr, args: &[Expr]) -> ExprValue {
        let name = match callee {
            Expr::Identifier { name } => name.clone(),
            _ => {
                self.error("仅支持具名函数调用".to_string());
                return ExprValue::error();
            }
        };

        let sig = self.table.lookup_function(&name).cloned();
        let Some(sig) = sig else {
            self.error(format!("调用未声明的函数 `{}`（规则 3.5）", name));
            for a in args {
                self.gen_expr(a);
            }
            return ExprValue::error();
        };

        // 实参数量检查（规则 3.5）
        if sig.params.len() != args.len() {
            self.error(format!(
                "函数 `{}` 形参数量 {}，实参数量 {} 不一致（规则 3.5）",
                name,
                sig.params.len(),
                args.len()
            ));
        }

        // 求值各实参并发出 PARAM
        let mut arg_places = Vec::with_capacity(args.len());
        for (i, a) in args.iter().enumerate() {
            let v = self.gen_expr(a);
            if let Some((pname, pty, _)) = sig.params.get(i) {
                if pty.is_known()
                    && v.ty.is_known()
                    && !pty.compatible(&v.ty)
                {
                    self.error(format!(
                        "函数 `{}` 第 {} 个实参类型 {} 与形参 `{}` 类型 {} 不匹配（规则 3.5）",
                        name,
                        i + 1,
                        v.ty.display(),
                        pname,
                        pty.display()
                    ));
                }
            }
            arg_places.push(v.place);
        }
        for p in &arg_places {
            self.ir
                .emit("PARAM", p.clone(), PLACEHOLDER, PLACEHOLDER);
        }

        if matches!(sig.return_type, Type::Unit) {
            self.ir.emit(
                "CALL",
                name,
                arg_places.len().to_string(),
                PLACEHOLDER,
            );
            ExprValue::new(Type::Unit, PLACEHOLDER.to_string())
        } else {
            let t = self.ir.new_temp();
            self.ir
                .emit("CALL", name, arg_places.len().to_string(), &t);
            ExprValue::new(sig.return_type, t)
        }
    }

    fn gen_index(&mut self, base: &Expr, index: &Expr) -> ExprValue {
        let b = self.gen_expr(base);
        let i = self.gen_expr(index);
        if i.ty.is_known() && !i.ty.is_integer() {
            self.error(format!(
                "数组下标类型 {} 不是整数（规则 8.3）",
                i.ty.display()
            ));
        }
        let (elem_ty, len) = match &b.ty {
            Type::Array { element, length } => ((**element).clone(), Some(*length)),
            Type::Error => return ExprValue::error(),
            other if other.is_known() => {
                self.error(format!(
                    "类型 {} 不可索引（规则 8.3）",
                    other.display()
                ));
                return ExprValue::error();
            }
            _ => (Type::Error, None),
        };
        // 静态越界检查：支持正字面量、负字面量（Unary{Neg, Number}）、
        // 以及超出 u128 的极大正字面量（解析失败一律视为越界）。
        if let Some(len) = len {
            let oob: Option<(bool, String)> = match index {
                Expr::Number { value } => match value.parse::<u128>() {
                    Ok(n) => Some((n >= len as u128, value.clone())),
                    Err(_) => Some((true, value.clone())),
                },
                Expr::Unary { op: UnaryOp::Neg, expr } => {
                    if let Expr::Number { value } = expr.as_ref() {
                        // 负数对任意非空数组都越界；空数组 (len=0) 也越界。
                        Some((true, format!("-{}", value)))
                    } else {
                        None
                    }
                }
                _ => None,
            };
            if let Some((true, shown)) = oob {
                let name = root_identifier(base)
                    .map(|s| format!("数组 `{}` ", s))
                    .unwrap_or_default();
                self.error(format!(
                    "{}下标 {} 越界，合法范围 [0,{})（规则 8.3）",
                    name, shown, len
                ));
            }
        }
        let t = self.ir.new_temp();
        self.ir.emit("INDEX", b.place, i.place, &t);
        ExprValue::new(elem_ty, t)
    }

    fn gen_field(&mut self, base: &Expr, field: &str) -> ExprValue {
        let b = self.gen_expr(base);
        // 元组字段访问（规则 9.3）
        let idx = match field.parse::<usize>() {
            Ok(i) => i,
            Err(_) => {
                self.error(format!(
                    "元组字段 `{}` 不是合法整数下标（规则 9.3）",
                    field
                ));
                return ExprValue::error();
            }
        };
        let elem_ty = match &b.ty {
            Type::Tuple { elements } => {
                if idx >= elements.len() {
                    self.error(format!(
                        "元组下标 {} 越界，合法范围 [0,{})（规则 9.3）",
                        idx,
                        elements.len()
                    ));
                    Type::Error
                } else {
                    elements[idx].clone()
                }
            }
            Type::Error => return ExprValue::error(),
            other if other.is_known() => {
                self.error(format!(
                    "类型 {} 不支持点字段访问（规则 9.3）",
                    other.display()
                ));
                Type::Error
            }
            _ => Type::Error,
        };
        let t = self.ir.new_temp();
        self.ir.emit("FIELD", b.place, field.to_string(), &t);
        ExprValue::new(elem_ty, t)
    }

    fn gen_array(&mut self, elements: &[Expr]) -> ExprValue {
        let mut elem_ty: Option<Type> = None;
        let mut places = Vec::new();
        for e in elements {
            let v = self.gen_expr(e);
            if let Some(et) = &elem_ty {
                if et.is_known() && v.ty.is_known() && !et.compatible(&v.ty) {
                    self.error(format!(
                        "数组元素类型不一致：{} vs {}（规则 8.2）",
                        et.display(),
                        v.ty.display()
                    ));
                }
            } else if v.ty.is_known() {
                elem_ty = Some(v.ty.clone());
            }
            places.push(v.place);
        }
        // 空数组字面量 `[]` 没有元素可推断类型，用 Error 作元素类型，
        // 经 `compatible` 通配传播，避免对 `let a:[i32;0]=[];` 误报。
        let inferred = Type::Array {
            element: Box::new(elem_ty.unwrap_or(Type::Error)),
            length: elements.len(),
        };
        let t = self.ir.new_temp();
        self.ir.emit(
            "ARRAY",
            places.join(","),
            elements.len().to_string(),
            &t,
        );
        ExprValue::new(inferred, t)
    }

    fn gen_tuple(&mut self, elements: &[Expr]) -> ExprValue {
        let mut tys = Vec::new();
        let mut places = Vec::new();
        for e in elements {
            let v = self.gen_expr(e);
            tys.push(v.ty);
            places.push(v.place);
        }
        let ty = if tys.is_empty() {
            Type::Unit
        } else {
            Type::Tuple { elements: tys }
        };
        let t = self.ir.new_temp();
        self.ir.emit(
            "TUPLE",
            places.join(","),
            elements.len().to_string(),
            &t,
        );
        ExprValue::new(ty, t)
    }

    fn gen_block_expr(&mut self, block: &Block) -> ExprValue {
        self.push_scope();
        for s in &block.statements {
            self.gen_stmt(s);
        }
        let result = if let Some(tail) = &block.tail {
            self.gen_expr(tail)
        } else {
            ExprValue::new(Type::Unit, PLACEHOLDER.to_string())
        };
        self.check_current_scope_uninferred();
        self.pop_scope();
        result
    }

    fn gen_if_expr(
        &mut self,
        condition: &Expr,
        then_branch: &Block,
        else_branch: &ElseBranch,
    ) -> ExprValue {
        let cond = self.gen_expr(condition);
        self.check_condition_type(&cond.ty, "if");

        let label_else = self.ir.new_label();
        let label_end = self.ir.new_label();
        let has_else = !matches!(else_branch, ElseBranch::None);
        let jump_target = if has_else { &label_else } else { &label_end };
        self.ir.emit_if_false(&cond.place, jump_target);

        // then 分支
        let then_val = self.gen_block_expr(then_branch);
        // 只有分支真正产生值时才分配临时变量，避免 Unit/Unit 时的悬空 temp。
        let mut result_temp: Option<String> = None;
        let mut result_ty = then_val.ty.clone();
        if !matches!(then_val.ty, Type::Unit) {
            let t = self.ir.new_temp();
            self.ir.emit("=", then_val.place, PLACEHOLDER, &t);
            result_temp = Some(t);
        }
        if has_else {
            self.ir.emit_goto(&label_end);
            self.ir.emit_label(&label_else);
            let else_val = match else_branch {
                ElseBranch::Block { block } => self.gen_block_expr(block),
                ElseBranch::ElseIf { expr } => self.gen_expr(expr),
                ElseBranch::None => unreachable!(),
            };
            if else_val.ty.is_known()
                && result_ty.is_known()
                && !result_ty.compatible(&else_val.ty)
            {
                self.error(format!(
                    "if 表达式分支类型不一致：{} vs {}",
                    result_ty.display(),
                    else_val.ty.display()
                ));
            }
            if matches!(result_ty, Type::Unit) {
                result_ty = else_val.ty.clone();
            }
            if !matches!(else_val.ty, Type::Unit) {
                let t = result_temp
                    .clone()
                    .unwrap_or_else(|| self.ir.new_temp());
                self.ir.emit("=", else_val.place, PLACEHOLDER, &t);
                result_temp = Some(t);
            }
        }
        self.ir.emit_label(&label_end);
        let place = result_temp.unwrap_or_else(|| PLACEHOLDER.to_string());
        ExprValue::new(result_ty, place)
    }

    fn gen_loop_expr(&mut self, body: &Block) -> ExprValue {
        let label_start = self.ir.new_label();
        let label_end = self.ir.new_label();
        let result_place = self.ir.new_temp();
        self.ir.emit_label(&label_start);
        if let Some(ctx) = self.func_ctx.as_mut() {
            ctx.loop_depth += 1;
            ctx.loop_exprs.push(LoopExprCtx {
                result_place: result_place.clone(),
                break_type: None,
            });
            ctx.loop_labels.push(LoopLabels {
                start: label_start.clone(),
                end: label_end.clone(),
            });
        }
        self.gen_block_stmt(body);
        let break_type = if let Some(ctx) = self.func_ctx.as_mut() {
            ctx.loop_depth -= 1;
            ctx.loop_labels.pop();
            ctx.loop_exprs.pop().and_then(|loop_ctx| loop_ctx.break_type)
        } else {
            None
        };
        // label_end 仅作为 break 的跳转目标存在（loop 语义本身无 fall-through 退出）。
        // 配合 gen_break 把 end 写入 BREAK 四元式第四元，end 通过 BREAK 可达。
        self.ir.emit_goto(&label_start);
        self.ir.emit_label(&label_end);
        ExprValue::new(break_type.unwrap_or(Type::Unit), result_place)
    }
}

impl Default for Analyzer {
    fn default() -> Self {
        Self::new()
    }
}

fn root_identifier(expr: &Expr) -> Option<&str> {
    match expr {
        Expr::Identifier { name } => Some(name),
        Expr::Index { base, .. } | Expr::Field { base, .. } => root_identifier(base),
        _ => None,
    }
}

/// 将 BinaryOp 转为四元式 op 字符串及 (是否比较)。
fn bin_op_info(op: BinaryOp) -> (&'static str, bool) {
    match op {
        BinaryOp::Add => ("+", false),
        BinaryOp::Sub => ("-", false),
        BinaryOp::Mul => ("*", false),
        BinaryOp::Div => ("/", false),
        BinaryOp::Lt => ("<", true),
        BinaryOp::Le => ("<=", true),
        BinaryOp::Gt => (">", true),
        BinaryOp::Ge => (">=", true),
        BinaryOp::Eq => ("==", true),
        BinaryOp::Ne => ("!=", true),
        BinaryOp::Range => ("..", false),
    }
}

// 保留 TypeNode 引入用作未来扩展（如解析数组长度等）。
#[allow(dead_code)]
fn _force_type_node_use(_n: &TypeNode) {}
