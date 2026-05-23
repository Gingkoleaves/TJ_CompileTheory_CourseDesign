//! 四元式中间代码（阶段 5 充实）。

use serde::Serialize;

/// 四元式：(op, arg1, arg2, result)，arg/result 为空时用 `_`。
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
