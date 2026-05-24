//! 内部类型系统。
//!
//! `easy_parser::TypeNode` 是表层语法，本模块定义语义分析使用的
//! 规范化 `Type` 表示，并提供与 `TypeNode` 的双向转换。

use serde::Serialize;

use easy_parser::TypeNode;

/// 语义分析中的类型表示。
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "kind")]
pub enum Type {
    /// 32 位整型 `i32`。
    I32,
    /// 布尔型（比较运算结果产生；本语言没有 bool 字面量）。
    Bool,
    /// 单元类型 `()`。
    Unit,
    /// 引用类型 `&T` 或 `&mut T`。
    Ref { mutable: bool, inner: Box<Type> },
    /// 定长数组 `[T; N]`。
    Array { element: Box<Type>, length: usize },
    /// 元组 `(T1, T2, ...)`。
    Tuple { elements: Vec<Type> },
    /// 整型字面量字面值范围（`a..b`），用于 for 循环。
    Range,
    /// 函数类型占位：标识符指向函数（仅出现在函数名被当作表达式使用时）。
    Function,
    /// 类型未知（声明未给类型且未初始化前）。
    Unknown,
    /// 永不返回（return 表达式等）。
    Never,
    /// 类型错误占位，用于错误恢复，不再连锁报错。
    Error,
}

impl Type {
    /// 是否为整数类型（i32）。
    pub fn is_integer(&self) -> bool {
        matches!(self, Type::I32)
    }

    /// 是否为引用类型。
    pub fn is_reference(&self) -> bool {
        matches!(self, Type::Ref { .. })
    }

    /// 是否已知（非 Unknown / Error）。
    pub fn is_known(&self) -> bool {
        !matches!(self, Type::Unknown | Type::Error)
    }

    /// 是否兼容（用于赋值、返回、参数）。
    /// - `Error` 与任何类型兼容（避免连锁报错）。
    /// - `Never` 与任何类型兼容。
    /// - 其他情况按结构相等。
    pub fn compatible(&self, other: &Type) -> bool {
        match (self, other) {
            (Type::Error, _) | (_, Type::Error) => true,
            (Type::Never, _) | (_, Type::Never) => true,
            (Type::Ref { mutable: m1, inner: i1 }, Type::Ref { mutable: m2, inner: i2 }) => {
                m1 == m2 && i1.compatible(i2)
            }
            (
                Type::Array { element: e1, length: l1 },
                Type::Array { element: e2, length: l2 },
            ) => l1 == l2 && e1.compatible(e2),
            (Type::Tuple { elements: a }, Type::Tuple { elements: b }) => {
                a.len() == b.len() && a.iter().zip(b.iter()).all(|(x, y)| x.compatible(y))
            }
            (a, b) => a == b,
        }
    }

    /// 用于错误信息的中文显示。
    pub fn display(&self) -> String {
        match self {
            Type::I32 => "i32".to_string(),
            Type::Bool => "bool".to_string(),
            Type::Unit => "()".to_string(),
            Type::Ref { mutable, inner } => {
                if *mutable {
                    format!("&mut {}", inner.display())
                } else {
                    format!("&{}", inner.display())
                }
            }
            Type::Array { element, length } => format!("[{}; {}]", element.display(), length),
            Type::Tuple { elements } => {
                let inner = elements
                    .iter()
                    .map(|t| t.display())
                    .collect::<Vec<_>>()
                    .join(", ");
                if elements.len() == 1 {
                    format!("({},)", inner)
                } else {
                    format!("({})", inner)
                }
            }
            Type::Range => "Range<i32>".to_string(),
            // 这些占位均面向最终用户的错误信息，避免内部尖括号格式渗出。
            Type::Function => "函数".to_string(),
            Type::Unknown => "未知类型".to_string(),
            Type::Never => "!".to_string(),
            Type::Error => "类型错误".to_string(),
        }
    }
}

/// 将 `TypeNode` 转换为内部 `Type`。
/// 数组长度为 0 仍合法（语法允许），交由后续校验。
pub fn from_node(node: &TypeNode) -> Type {
    match node {
        TypeNode::Named { name } => match name.as_str() {
            "i32" => Type::I32,
            _ => Type::Error,
        },
        TypeNode::Reference { mutable, ty } => Type::Ref {
            mutable: *mutable,
            inner: Box::new(from_node(ty)),
        },
        TypeNode::Array { element, length } => Type::Array {
            element: Box::new(from_node(element)),
            length: *length,
        },
        TypeNode::Tuple { elements } => Type::Tuple {
            elements: elements.iter().map(from_node).collect(),
        },
        TypeNode::Unit => Type::Unit,
    }
}
