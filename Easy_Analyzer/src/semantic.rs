//! 语义分析器（阶段 4 充实）。

use serde::Serialize;

use crate::ir::Quadruple;

#[derive(Debug, Clone, Serialize)]
pub struct SemanticError {
    pub message: String,
}

impl SemanticError {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

pub struct Analyzer {
    errors: Vec<SemanticError>,
    quadruples: Vec<Quadruple>,
}

impl Analyzer {
    pub fn new() -> Self {
        Self {
            errors: Vec::new(),
            quadruples: Vec::new(),
        }
    }

    pub fn analyze_program(&mut self, _program: &easy_parser::Program) {
        // 阶段 4 起逐步填充
    }

    pub fn finish(self) -> (Vec<SemanticError>, Vec<Quadruple>) {
        (self.errors, self.quadruples)
    }
}

impl Default for Analyzer {
    fn default() -> Self {
        Self::new()
    }
}
