//! Easy_Analyzer：类 Rust 语言的语义分析器与中间代码（四元式）生成器。
//!
//! 公共入口 [`analyze`] 接收 `easy_parser::Program`，返回
//! [`AnalysisResult`]，包含语义错误列表与四元式列表。

use serde::Serialize;

pub mod ir;
pub mod semantic;
pub mod symbol;
pub mod types;

pub use ir::Quadruple;
pub use semantic::SemanticError;

#[derive(Debug, Clone, Serialize)]
pub struct AnalysisResult {
    #[serde(rename = "semanticErrors")]
    pub semantic_errors: Vec<SemanticError>,
    pub quadruples: Vec<Quadruple>,
}

pub fn analyze(program: &easy_parser::Program) -> AnalysisResult {
    let mut analyzer = semantic::Analyzer::new();
    analyzer.analyze_program(program);
    let (errors, quadruples) = analyzer.finish();
    AnalysisResult {
        semantic_errors: errors,
        quadruples,
    }
}
