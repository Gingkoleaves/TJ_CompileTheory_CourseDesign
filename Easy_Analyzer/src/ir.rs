//! 四元式中间代码。
//!
//! 约定操作码：
//! - `=`：赋值 `(=, src, _, dst)`
//! - `[]=`：数组下标写回 `([]=, value, index, array)`
//! - `.=`：元组字段写回 `(.=, value, field, tuple)`
//! - `INDEX`：数组下标读 `(INDEX, array, index, t)`
//! - `FIELD`：元组字段读 `(FIELD, tuple, field, t)`
//! - `+ - * /`：算术 `(+, a, b, t)`
//! - `< <= > >= == !=`：比较 `(cmp, a, b, t)`，结果为 0/1
//! - `NEG`：取负 `(NEG, a, _, t)`
//! - `FUNC` / `END_FUNC`：函数边界 `(FUNC, name, _, _)`
//! - `PARAM_DECL`：形参声明 `(PARAM_DECL, name, type, _)`
//! - `PARAM`：传参 `(PARAM, value, _, _)`
//! - `CALL`：调用 `(CALL, name, argc, result)`；void 时 result 为 `_`
//! - `RETURN`：返回 `(RETURN, value_or_underscore, _, _)`
//! - `LABEL`：标签 `(LABEL, name, _, _)`
//! - `GOTO`：无条件跳转 `(GOTO, _, _, name)`
//! - `IF_FALSE`：条件假跳 `(IF_FALSE, cond, _, name)`
//!
//! 占位符均为字符串 `"_"`。临时变量 `tN`，标签 `LN`。

use serde::Serialize;

pub const PLACEHOLDER: &str = "_";

#[derive(Debug, Clone, Serialize)]
pub struct Quadruple {
    pub op: String,
    pub arg1: String,
    pub arg2: String,
    pub result: String,
}

impl Quadruple {
    pub fn new(
        op: impl Into<String>,
        arg1: impl Into<String>,
        arg2: impl Into<String>,
        result: impl Into<String>,
    ) -> Self {
        Self {
            op: op.into(),
            arg1: arg1.into(),
            arg2: arg2.into(),
            result: result.into(),
        }
    }
}

/// 临时变量 / 标签生成器。
#[derive(Debug, Default)]
pub struct IrBuilder {
    pub quads: Vec<Quadruple>,
    next_temp: usize,
    next_label: usize,
}

impl IrBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn new_temp(&mut self) -> String {
        self.next_temp += 1;
        format!("t{}", self.next_temp)
    }

    pub fn new_label(&mut self) -> String {
        self.next_label += 1;
        format!("L{}", self.next_label)
    }

    pub fn emit(
        &mut self,
        op: impl Into<String>,
        arg1: impl Into<String>,
        arg2: impl Into<String>,
        result: impl Into<String>,
    ) {
        self.quads.push(Quadruple::new(op, arg1, arg2, result));
    }

    pub fn emit_label(&mut self, label: &str) {
        self.emit("LABEL", label, PLACEHOLDER, PLACEHOLDER);
    }

    pub fn emit_goto(&mut self, label: &str) {
        self.emit("GOTO", PLACEHOLDER, PLACEHOLDER, label);
    }

    pub fn emit_if_false(&mut self, cond: &str, label: &str) {
        self.emit("IF_FALSE", cond, PLACEHOLDER, label);
    }
}
