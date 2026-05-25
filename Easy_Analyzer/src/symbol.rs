//! 作用域符号表 & 函数签名表。
//!
//! 变量表采用作用域栈：进入语句块/函数 push，离开 pop。
//! 同一作用域内允许 shadowing（同名重新声明覆盖旧绑定）。

use std::collections::HashMap;

use crate::types::Type;

/// 变量符号。
#[derive(Debug, Clone)]
pub struct VarSymbol {
    pub name: String,
    /// 类型；声明无类型且未初始化时为 [`Type::Unknown`]。
    pub ty: Type,
    /// 是否声明为 `mut`。
    pub mutable: bool,
    /// 是否已被赋值（初始化）。
    pub initialized: bool,
}

impl VarSymbol {
    pub fn new(name: String, ty: Type, mutable: bool, initialized: bool) -> Self {
        Self {
            name,
            ty,
            mutable,
            initialized,
        }
    }
}

/// 函数签名。
#[derive(Debug, Clone)]
pub struct FunctionSig {
    pub name: String,
    pub params: Vec<(String, Type, bool)>, // (name, type, mutable)
    pub return_type: Type, // Unit 表示无返回值
}

/// 作用域栈：栈顶是最内层作用域。
#[derive(Debug, Default)]
pub struct SymbolTable {
    scopes: Vec<HashMap<String, VarSymbol>>,
    functions: HashMap<String, FunctionSig>,
}

impl SymbolTable {
    pub fn new() -> Self {
        Self {
            scopes: vec![HashMap::new()],
            functions: HashMap::new(),
        }
    }

    pub fn push_scope(&mut self) {
        self.scopes.push(HashMap::new());
    }

    pub fn pop_scope(&mut self) {
        if self.scopes.len() > 1 {
            self.scopes.pop();
        }
    }

    /// 在当前作用域声明（允许 shadowing 覆盖）。
    pub fn declare(&mut self, sym: VarSymbol) {
        if let Some(scope) = self.scopes.last_mut() {
            scope.insert(sym.name.clone(), sym);
        }
    }

    /// 由内向外查找。
    pub fn lookup(&self, name: &str) -> Option<&VarSymbol> {
        for scope in self.scopes.iter().rev() {
            if let Some(s) = scope.get(name) {
                return Some(s);
            }
        }
        None
    }

    /// 由内向外查找，获取可变引用以便修改 initialized/type 等。
    pub fn lookup_mut(&mut self, name: &str) -> Option<&mut VarSymbol> {
        for scope in self.scopes.iter_mut().rev() {
            if scope.contains_key(name) {
                return scope.get_mut(name);
            }
        }
        None
    }

    /// 注册函数签名（重名后者覆盖前者，符合 shadowing 风格）。
    pub fn declare_function(&mut self, sig: FunctionSig) {
        self.functions.insert(sig.name.clone(), sig);
    }

    pub fn lookup_function(&self, name: &str) -> Option<&FunctionSig> {
        self.functions.get(name)
    }

    pub fn current_scope_uninferred_names(&self) -> Vec<String> {
        self.scopes
            .last()
            .map(|scope| {
                scope
                    .values()
                    .filter(|sym| matches!(sym.ty, Type::Unknown))
                    .map(|sym| sym.name.clone())
                    .collect()
            })
            .unwrap_or_default()
    }
}
