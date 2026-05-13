use std::fmt;

use serde::Serialize;

use easy_lexer::{Keyword, Position, Token, TokenKind};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseError {
    pub message: String,
    pub position: Position,
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "syntax error at {}:{}: {}",
            self.position.line, self.position.column, self.message
        )
    }
}

impl std::error::Error for ParseError {}

#[allow(dead_code)]
pub fn parse_tokens(tokens: &[Token]) -> Result<(), ParseError> {
    Parser::new(tokens).parse_program().map(|_| ())
}

pub fn parse_program_ast(tokens: &[Token]) -> Result<Program, ParseError> {
    Parser::new(tokens).parse_program()
}

#[derive(Debug, Clone, Serialize)]
pub struct Program {
    pub kind: &'static str,
    pub functions: Vec<FunctionDecl>,
}

impl Program {
    fn new(functions: Vec<FunctionDecl>) -> Self {
        Self {
            kind: "Program",
            functions,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct FunctionDecl {
    pub kind: &'static str,
    pub name: String,
    pub params: Vec<Binding>,
    #[serde(rename = "returnType")]
    pub return_type: Option<TypeNode>,
    pub body: Block,
}

impl FunctionDecl {
    fn new(name: String, params: Vec<Binding>, return_type: Option<TypeNode>, body: Block) -> Self {
        Self {
            kind: "FunctionDecl",
            name,
            params,
            return_type,
            body,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct Binding {
    pub kind: &'static str,
    pub mutable: bool,
    pub name: String,
    pub ty: Option<TypeNode>,
}

impl Binding {
    fn new(name: String, mutable: bool, ty: Option<TypeNode>) -> Self {
        Self {
            kind: "Binding",
            mutable,
            name,
            ty,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct Block {
    pub kind: &'static str,
    pub statements: Vec<Statement>,
    pub tail: Option<Box<Expr>>,
}

impl Block {
    fn new(statements: Vec<Statement>, tail: Option<Box<Expr>>) -> Self {
        Self {
            kind: "Block",
            statements,
            tail,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "kind")]
pub enum Statement {
    Empty,
    Let {
        binding: Binding,
        init: Option<Box<Expr>>,
    },
    Assign {
        target: Box<Expr>,
        value: Box<Expr>,
    },
    Expr {
        expr: Box<Expr>,
    },
    Return {
        value: Option<Box<Expr>>,
    },
    While {
        condition: Box<Expr>,
        body: Block,
    },
    For {
        binding: Binding,
        iterable: Box<Expr>,
        body: Block,
    },
    Break {
        value: Option<Box<Expr>>,
    },
    Continue,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "kind")]
pub enum Expr {
    Identifier {
        name: String,
    },
    Number {
        value: String,
    },
    Unary {
        op: UnaryOp,
        expr: Box<Expr>,
    },
    Binary {
        left: Box<Expr>,
        op: BinaryOp,
        right: Box<Expr>,
    },
    Call {
        callee: Box<Expr>,
        args: Vec<Expr>,
    },
    Index {
        base: Box<Expr>,
        index: Box<Expr>,
    },
    Field {
        base: Box<Expr>,
        field: String,
    },
    Array {
        elements: Vec<Expr>,
    },
    Tuple {
        elements: Vec<Expr>,
    },
    Block {
        block: Block,
    },
    If {
        condition: Box<Expr>,
        then_branch: Block,
        else_branch: ElseBranch,
    },
    Loop {
        body: Block,
    },
}

impl Expr {
    fn is_assignable(&self) -> bool {
        match self {
            Self::Identifier { .. } => true,
            Self::Unary {
                op: UnaryOp::Deref, ..
            } => true,
            Self::Index { .. } => true,
            Self::Field { .. } => true,
            _ => false,
        }
    }

    fn can_stand_without_semicolon(&self) -> bool {
        matches!(
            self,
            Self::Block { .. } | Self::If { .. } | Self::Loop { .. }
        )
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "kind")]
pub enum ElseBranch {
    None,
    Block { block: Block },
    ElseIf { expr: Box<Expr> },
}

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, Serialize)]
pub enum UnaryOp {
    Ref,
    RefMut,
    Deref,
    Neg,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, Serialize)]
pub enum BinaryOp {
    Add,
    Sub,
    Mul,
    Div,
    Lt,
    Le,
    Gt,
    Ge,
    Eq,
    Ne,
    Range,
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "kind")]
pub enum TypeNode {
    Named {
        name: String,
    },
    Reference {
        mutable: bool,
        ty: Box<TypeNode>,
    },
    Array {
        element: Box<TypeNode>,
        length: usize,
    },
    Tuple {
        elements: Vec<TypeNode>,
    },
    Unit,
}

struct Parser<'a> {
    tokens: &'a [Token],
    current: usize,
}

impl<'a> Parser<'a> {
    fn new(tokens: &'a [Token]) -> Self {
        Self { tokens, current: 0 }
    }

    fn parse_program(mut self) -> Result<Program, ParseError> {
        let mut functions = Vec::new();

        while !self.is_at_end() && !self.check_simple(SimpleTokenKind::EndMarker) {
            functions.push(self.parse_function_decl()?);
        }

        if self.match_simple(SimpleTokenKind::EndMarker) && !self.is_at_end() {
            return Err(self.error_here("unexpected tokens after `#`"));
        }

        if !self.is_at_end() {
            return Err(self.error_here("unexpected trailing tokens"));
        }

        Ok(Program::new(functions))
    }

    fn parse_function_decl(&mut self) -> Result<FunctionDecl, ParseError> {
        self.expect_keyword(Keyword::Fn)?;
        let name = self.expect_identifier("expected function name after `fn`")?;
        self.expect_simple(SimpleTokenKind::LParen, "expected `(` after function name")?;
        let params = self.parse_parameter_list()?;
        self.expect_simple(
            SimpleTokenKind::RParen,
            "expected `)` after function parameter list",
        )?;

        let return_type = if self.match_simple(SimpleTokenKind::Arrow) {
            Some(self.parse_type()?)
        } else {
            None
        };

        let body = self.parse_block()?;
        Ok(FunctionDecl::new(name, params, return_type, body))
    }

    fn parse_parameter_list(&mut self) -> Result<Vec<Binding>, ParseError> {
        let mut params = Vec::new();

        if self.check_simple(SimpleTokenKind::RParen) {
            return Ok(params);
        }

        loop {
            let binding = self.parse_binding(true)?;
            if binding.ty.is_none() {
                return Err(self.error_here("expected `:` after parameter name"));
            }
            params.push(binding);

            if !self.match_simple(SimpleTokenKind::Comma) {
                break;
            }
        }

        Ok(params)
    }

    fn parse_type(&mut self) -> Result<TypeNode, ParseError> {
        if self.match_keyword(Keyword::I32) {
            return Ok(TypeNode::Named {
                name: "i32".to_string(),
            });
        }

        if self.match_simple(SimpleTokenKind::Ampersand) {
            let mutable = self.match_keyword(Keyword::Mut);
            return Ok(TypeNode::Reference {
                mutable,
                ty: Box::new(self.parse_type()?),
            });
        }

        if self.match_simple(SimpleTokenKind::LBracket) {
            let element = self.parse_type()?;
            self.expect_simple(
                SimpleTokenKind::Semicolon,
                "expected `;` in array type declaration",
            )?;
            let length = self
                .expect_number_text("expected array length in array type")?
                .parse::<usize>()
                .map_err(|_| self.error_here("expected numeric literal"))?;
            self.expect_simple(SimpleTokenKind::RBracket, "expected `]` after array type")?;
            return Ok(TypeNode::Array {
                element: Box::new(element),
                length,
            });
        }

        if self.match_simple(SimpleTokenKind::LParen) {
            if self.match_simple(SimpleTokenKind::RParen) {
                return Ok(TypeNode::Unit);
            }

            let mut elements = vec![self.parse_type()?];
            self.expect_simple(
                SimpleTokenKind::Comma,
                "tuple type requires `,` after the first element type",
            )?;

            if !self.check_simple(SimpleTokenKind::RParen) {
                loop {
                    elements.push(self.parse_type()?);
                    if !self.match_simple(SimpleTokenKind::Comma) {
                        break;
                    }
                    if self.check_simple(SimpleTokenKind::RParen) {
                        break;
                    }
                }
            }

            self.expect_simple(SimpleTokenKind::RParen, "expected `)` after tuple type")?;
            return Ok(TypeNode::Tuple { elements });
        }

        Err(self.error_here("expected a type"))
    }

    fn parse_block(&mut self) -> Result<Block, ParseError> {
        self.expect_simple(SimpleTokenKind::LBrace, "expected `{` to start a block")?;
        let mut statements = Vec::new();
        let mut tail = None;

        while !self.check_simple(SimpleTokenKind::RBrace) {
            if self.is_at_end() {
                return Err(self.error_here("expected `}` to close the block"));
            }

            if self.starts_forced_statement() {
                statements.push(self.parse_statement()?);
                continue;
            }

            let expr = self.parse_expression()?;

            if self.match_simple(SimpleTokenKind::Assign) {
                if !expr.is_assignable() {
                    return Err(self.error_here("left side of assignment is not assignable"));
                }
                let value = self.parse_expression()?;
                self.expect_simple(
                    SimpleTokenKind::Semicolon,
                    "expected `;` after assignment statement",
                )?;
                statements.push(Statement::Assign {
                    target: Box::new(expr),
                    value: Box::new(value),
                });
                continue;
            }

            if self.match_simple(SimpleTokenKind::Semicolon) {
                statements.push(Statement::Expr {
                    expr: Box::new(expr),
                });
                continue;
            }

            if expr.can_stand_without_semicolon() && !self.check_simple(SimpleTokenKind::RBrace) {
                statements.push(Statement::Expr {
                    expr: Box::new(expr),
                });
                continue;
            }

            tail = Some(Box::new(expr));
            break;
        }

        self.expect_simple(SimpleTokenKind::RBrace, "expected `}` to close the block")?;
        Ok(Block::new(statements, tail))
    }

    fn parse_statement(&mut self) -> Result<Statement, ParseError> {
        if self.match_simple(SimpleTokenKind::Semicolon) {
            return Ok(Statement::Empty);
        }

        if self.match_keyword(Keyword::Let) {
            let binding = self.parse_binding(false)?;
            let init = if self.match_simple(SimpleTokenKind::Assign) {
                Some(Box::new(self.parse_expression()?))
            } else {
                None
            };
            self.expect_simple(
                SimpleTokenKind::Semicolon,
                "expected `;` after variable declaration",
            )?;
            return Ok(Statement::Let { binding, init });
        }

        if self.match_keyword(Keyword::Return) {
            let value = if self.check_simple(SimpleTokenKind::Semicolon) {
                None
            } else {
                Some(Box::new(self.parse_expression()?))
            };
            self.expect_simple(SimpleTokenKind::Semicolon, "expected `;` after return")?;
            return Ok(Statement::Return { value });
        }

        if self.match_keyword(Keyword::While) {
            let condition = self.parse_expression()?;
            let body = self.parse_block()?;
            return Ok(Statement::While {
                condition: Box::new(condition),
                body,
            });
        }

        if self.match_keyword(Keyword::For) {
            let binding = self.parse_binding(false)?;
            self.expect_keyword(Keyword::In)?;
            let iterable = self.parse_iterable()?;
            let body = self.parse_block()?;
            return Ok(Statement::For {
                binding,
                iterable: Box::new(iterable),
                body,
            });
        }

        if self.match_keyword(Keyword::Break) {
            let value = if self.check_simple(SimpleTokenKind::Semicolon) {
                None
            } else {
                Some(Box::new(self.parse_expression()?))
            };
            self.expect_simple(SimpleTokenKind::Semicolon, "expected `;` after break")?;
            return Ok(Statement::Break { value });
        }

        if self.match_keyword(Keyword::Continue) {
            self.expect_simple(SimpleTokenKind::Semicolon, "expected `;` after continue")?;
            return Ok(Statement::Continue);
        }

        Err(self.error_here("expected a statement"))
    }

    fn parse_binding(&mut self, require_type: bool) -> Result<Binding, ParseError> {
        let mutable = self.match_keyword(Keyword::Mut);
        let name = self.expect_identifier("expected variable name")?;
        let ty = if self.match_simple(SimpleTokenKind::Colon) {
            Some(self.parse_type()?)
        } else {
            None
        };

        if require_type && ty.is_none() {
            return Err(self.error_here("expected `:` after parameter name"));
        }

        Ok(Binding::new(name, mutable, ty))
    }

    fn parse_iterable(&mut self) -> Result<Expr, ParseError> {
        let left = self.parse_expression()?;
        if self.match_simple(SimpleTokenKind::DotDot) {
            let right = self.parse_expression()?;
            return Ok(Expr::Binary {
                left: Box::new(left),
                op: BinaryOp::Range,
                right: Box::new(right),
            });
        }
        Ok(left)
    }

    fn parse_expression(&mut self) -> Result<Expr, ParseError> {
        let mut expr = self.parse_additive()?;

        while let Some(op) = self.match_comparison_op() {
            let right = self.parse_additive()?;
            expr = Expr::Binary {
                left: Box::new(expr),
                op,
                right: Box::new(right),
            };
        }

        Ok(expr)
    }

    fn parse_additive(&mut self) -> Result<Expr, ParseError> {
        let mut expr = self.parse_multiplicative()?;
        loop {
            let op = if self.match_simple(SimpleTokenKind::Plus) {
                Some(BinaryOp::Add)
            } else if self.match_simple(SimpleTokenKind::Minus) {
                Some(BinaryOp::Sub)
            } else {
                None
            };

            let Some(op) = op else {
                break;
            };

            let right = self.parse_multiplicative()?;
            expr = Expr::Binary {
                left: Box::new(expr),
                op,
                right: Box::new(right),
            };
        }

        Ok(expr)
    }

    fn parse_multiplicative(&mut self) -> Result<Expr, ParseError> {
        let mut expr = self.parse_unary()?;
        loop {
            let op = if self.match_simple(SimpleTokenKind::Star) {
                Some(BinaryOp::Mul)
            } else if self.match_simple(SimpleTokenKind::Slash) {
                Some(BinaryOp::Div)
            } else {
                None
            };

            let Some(op) = op else {
                break;
            };

            let right = self.parse_unary()?;
            expr = Expr::Binary {
                left: Box::new(expr),
                op,
                right: Box::new(right),
            };
        }

        Ok(expr)
    }

    fn parse_unary(&mut self) -> Result<Expr, ParseError> {
        if self.match_simple(SimpleTokenKind::Ampersand) {
            let op = if self.match_keyword(Keyword::Mut) {
                UnaryOp::RefMut
            } else {
                UnaryOp::Ref
            };
            let expr = self.parse_unary()?;
            return Ok(Expr::Unary {
                op,
                expr: Box::new(expr),
            });
        }

        if self.match_simple(SimpleTokenKind::Star) {
            let expr = self.parse_unary()?;
            return Ok(Expr::Unary {
                op: UnaryOp::Deref,
                expr: Box::new(expr),
            });
        }

        if self.match_simple(SimpleTokenKind::Minus) {
            let expr = self.parse_postfix()?;
            return Ok(Expr::Unary {
                op: UnaryOp::Neg,
                expr: Box::new(expr),
            });
        }

        self.parse_postfix()
    }

    fn parse_postfix(&mut self) -> Result<Expr, ParseError> {
        let mut expr = self.parse_primary()?;

        loop {
            if self.check_simple(SimpleTokenKind::LParen) && Self::is_callable(&expr) {
                self.advance();
                let args = self.parse_argument_list()?;
                self.expect_simple(SimpleTokenKind::RParen, "expected `)` after argument list")?;
                expr = Expr::Call {
                    callee: Box::new(expr),
                    args,
                };
                continue;
            }

            if self.match_simple(SimpleTokenKind::LBracket) {
                let index = self.parse_expression()?;
                self.expect_simple(SimpleTokenKind::RBracket, "expected `]` after index")?;
                expr = Expr::Index {
                    base: Box::new(expr),
                    index: Box::new(index),
                };
                continue;
            }

            if self.match_simple(SimpleTokenKind::Dot) {
                let field = self.expect_number_text("expected tuple field index after `.`")?;
                expr = Expr::Field {
                    base: Box::new(expr),
                    field,
                };
                continue;
            }

            break;
        }

        Ok(expr)
    }

    fn parse_primary(&mut self) -> Result<Expr, ParseError> {
        if self.match_simple(SimpleTokenKind::Number) {
            return Ok(Expr::Number {
                value: self.previous_lexeme().to_string(),
            });
        }

        if self.match_simple(SimpleTokenKind::Identifier) {
            return Ok(Expr::Identifier {
                name: self.previous_lexeme().to_string(),
            });
        }

        if self.check_simple(SimpleTokenKind::LBrace) {
            return Ok(Expr::Block {
                block: self.parse_block()?,
            });
        }

        if self.match_keyword(Keyword::If) {
            return self.parse_if_after_keyword();
        }

        if self.match_keyword(Keyword::Loop) {
            return Ok(Expr::Loop {
                body: self.parse_block()?,
            });
        }

        if self.match_simple(SimpleTokenKind::LBracket) {
            let elements = self.parse_expression_list(SimpleTokenKind::RBracket)?;
            self.expect_simple(
                SimpleTokenKind::RBracket,
                "expected `]` after array literal",
            )?;
            return Ok(Expr::Array { elements });
        }

        if self.match_simple(SimpleTokenKind::LParen) {
            if self.match_simple(SimpleTokenKind::RParen) {
                return Ok(Expr::Tuple {
                    elements: Vec::new(),
                });
            }

            let first = self.parse_expression()?;
            if self.match_simple(SimpleTokenKind::Comma) {
                let mut elements = vec![first];
                if !self.check_simple(SimpleTokenKind::RParen) {
                    loop {
                        elements.push(self.parse_expression()?);
                        if !self.match_simple(SimpleTokenKind::Comma) {
                            break;
                        }
                        if self.check_simple(SimpleTokenKind::RParen) {
                            break;
                        }
                    }
                }
                self.expect_simple(SimpleTokenKind::RParen, "expected `)` after tuple literal")?;
                return Ok(Expr::Tuple { elements });
            }

            self.expect_simple(SimpleTokenKind::RParen, "expected `)` after expression")?;
            return Ok(first);
        }

        Err(self.error_here("expected an expression"))
    }

    fn parse_if_after_keyword(&mut self) -> Result<Expr, ParseError> {
        let condition = self.parse_expression()?;
        let then_branch = self.parse_block()?;
        let else_branch = if self.match_keyword(Keyword::Else) {
            if self.match_keyword(Keyword::If) {
                ElseBranch::ElseIf {
                    expr: Box::new(self.parse_if_after_keyword()?),
                }
            } else {
                ElseBranch::Block {
                    block: self.parse_block()?,
                }
            }
        } else {
            ElseBranch::None
        };

        Ok(Expr::If {
            condition: Box::new(condition),
            then_branch,
            else_branch,
        })
    }

    fn parse_argument_list(&mut self) -> Result<Vec<Expr>, ParseError> {
        self.parse_expression_list(SimpleTokenKind::RParen)
    }

    fn parse_expression_list(
        &mut self,
        terminator: SimpleTokenKind,
    ) -> Result<Vec<Expr>, ParseError> {
        let mut items = Vec::new();
        if self.check_simple(terminator) {
            return Ok(items);
        }

        loop {
            items.push(self.parse_expression()?);
            if !self.match_simple(SimpleTokenKind::Comma) {
                break;
            }
            if self.check_simple(terminator) {
                break;
            }
        }

        Ok(items)
    }

    fn match_comparison_op(&mut self) -> Option<BinaryOp> {
        if self.match_simple(SimpleTokenKind::Less) {
            Some(BinaryOp::Lt)
        } else if self.match_simple(SimpleTokenKind::LessEqual) {
            Some(BinaryOp::Le)
        } else if self.match_simple(SimpleTokenKind::Greater) {
            Some(BinaryOp::Gt)
        } else if self.match_simple(SimpleTokenKind::GreaterEqual) {
            Some(BinaryOp::Ge)
        } else if self.match_simple(SimpleTokenKind::EqualEqual) {
            Some(BinaryOp::Eq)
        } else if self.match_simple(SimpleTokenKind::NotEqual) {
            Some(BinaryOp::Ne)
        } else {
            None
        }
    }

    fn starts_forced_statement(&self) -> bool {
        matches!(
            self.peek_kind(),
            Some(TokenKind::Semicolon)
                | Some(TokenKind::Keyword(Keyword::Let))
                | Some(TokenKind::Keyword(Keyword::Return))
                | Some(TokenKind::Keyword(Keyword::While))
                | Some(TokenKind::Keyword(Keyword::For))
                | Some(TokenKind::Keyword(Keyword::Break))
                | Some(TokenKind::Keyword(Keyword::Continue))
        )
    }

    fn is_callable(expr: &Expr) -> bool {
        matches!(
            expr,
            Expr::Identifier { .. } | Expr::Field { .. } | Expr::Index { .. }
        )
    }

    fn expect_keyword(&mut self, keyword: Keyword) -> Result<(), ParseError> {
        if self.match_keyword(keyword) {
            Ok(())
        } else {
            Err(self.error_here(&format!("expected keyword `{keyword}`")))
        }
    }

    fn match_keyword(&mut self, keyword: Keyword) -> bool {
        if matches!(self.peek_kind(), Some(TokenKind::Keyword(actual)) if *actual == keyword) {
            self.advance();
            true
        } else {
            false
        }
    }

    fn expect_identifier(&mut self, message: &str) -> Result<String, ParseError> {
        self.expect_simple_text(SimpleTokenKind::Identifier, message)
    }

    fn expect_number_text(&mut self, message: &str) -> Result<String, ParseError> {
        self.expect_simple_text(SimpleTokenKind::Number, message)
    }

    fn expect_simple(
        &mut self,
        expected: SimpleTokenKind,
        message: &str,
    ) -> Result<(), ParseError> {
        if self.match_simple(expected) {
            Ok(())
        } else {
            Err(self.error_here(message))
        }
    }

    fn expect_simple_text(
        &mut self,
        expected: SimpleTokenKind,
        message: &str,
    ) -> Result<String, ParseError> {
        if self.check_simple(expected) {
            let lexeme = self
                .tokens
                .get(self.current)
                .map(|token| token.lexeme.clone())
                .unwrap_or_default();
            self.advance();
            Ok(lexeme)
        } else {
            Err(self.error_here(message))
        }
    }

    fn match_simple(&mut self, expected: SimpleTokenKind) -> bool {
        if self.check_simple(expected) {
            self.advance();
            true
        } else {
            false
        }
    }

    fn check_simple(&self, expected: SimpleTokenKind) -> bool {
        matches_simple(self.peek_kind(), expected)
    }

    fn advance(&mut self) {
        if !self.is_at_end() {
            self.current += 1;
        }
    }

    fn previous_lexeme(&self) -> &str {
        self.tokens
            .get(self.current.saturating_sub(1))
            .map(|token| token.lexeme.as_str())
            .unwrap_or("")
    }

    fn peek_kind(&self) -> Option<&TokenKind> {
        self.tokens.get(self.current).map(|token| &token.kind)
    }

    fn is_at_end(&self) -> bool {
        self.current >= self.tokens.len()
    }

    fn error_here(&self, message: &str) -> ParseError {
        if let Some(token) = self.tokens.get(self.current) {
            ParseError {
                message: format!("{message}, found `{}`", token.lexeme),
                position: token.position,
            }
        } else if let Some(token) = self.tokens.last() {
            ParseError {
                message: format!("{message}, found end of input"),
                position: token.position,
            }
        } else {
            ParseError {
                message: format!("{message}, found empty input"),
                position: Position { line: 1, column: 1 },
            }
        }
    }
}

#[derive(Debug, Clone, Copy)]
enum SimpleTokenKind {
    Identifier,
    Number,
    Assign,
    Plus,
    Minus,
    Star,
    Slash,
    EqualEqual,
    Greater,
    GreaterEqual,
    Less,
    LessEqual,
    NotEqual,
    Ampersand,
    LParen,
    RParen,
    LBrace,
    RBrace,
    LBracket,
    RBracket,
    Semicolon,
    Colon,
    Comma,
    Arrow,
    Dot,
    DotDot,
    EndMarker,
}

fn matches_simple(kind: Option<&TokenKind>, expected: SimpleTokenKind) -> bool {
    matches!(
        (kind, expected),
        (Some(TokenKind::Identifier), SimpleTokenKind::Identifier)
            | (Some(TokenKind::Number), SimpleTokenKind::Number)
            | (Some(TokenKind::Assign), SimpleTokenKind::Assign)
            | (Some(TokenKind::Plus), SimpleTokenKind::Plus)
            | (Some(TokenKind::Minus), SimpleTokenKind::Minus)
            | (Some(TokenKind::Star), SimpleTokenKind::Star)
            | (Some(TokenKind::Slash), SimpleTokenKind::Slash)
            | (Some(TokenKind::EqualEqual), SimpleTokenKind::EqualEqual)
            | (Some(TokenKind::Greater), SimpleTokenKind::Greater)
            | (Some(TokenKind::GreaterEqual), SimpleTokenKind::GreaterEqual)
            | (Some(TokenKind::Less), SimpleTokenKind::Less)
            | (Some(TokenKind::LessEqual), SimpleTokenKind::LessEqual)
            | (Some(TokenKind::NotEqual), SimpleTokenKind::NotEqual)
            | (Some(TokenKind::Ampersand), SimpleTokenKind::Ampersand)
            | (Some(TokenKind::LParen), SimpleTokenKind::LParen)
            | (Some(TokenKind::RParen), SimpleTokenKind::RParen)
            | (Some(TokenKind::LBrace), SimpleTokenKind::LBrace)
            | (Some(TokenKind::RBrace), SimpleTokenKind::RBrace)
            | (Some(TokenKind::LBracket), SimpleTokenKind::LBracket)
            | (Some(TokenKind::RBracket), SimpleTokenKind::RBracket)
            | (Some(TokenKind::Semicolon), SimpleTokenKind::Semicolon)
            | (Some(TokenKind::Colon), SimpleTokenKind::Colon)
            | (Some(TokenKind::Comma), SimpleTokenKind::Comma)
            | (Some(TokenKind::Arrow), SimpleTokenKind::Arrow)
            | (Some(TokenKind::Dot), SimpleTokenKind::Dot)
            | (Some(TokenKind::DotDot), SimpleTokenKind::DotDot)
            | (Some(TokenKind::EndMarker), SimpleTokenKind::EndMarker)
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use easy_lexer::lex;

    fn parse_ok(source: &str) -> Program {
        let result = lex(source);
        assert!(
            result.errors.is_empty(),
            "unexpected lex errors: {:?}",
            result.errors
        );
        parse_program_ast(&result.tokens).unwrap()
    }

    #[test]
    fn parses_minimum_required_grammar() {
        parse_ok(
            r#"
            fn base(mut a:i32) -> i32 {
                ;
                let mut b:i32;
                let c=1;
                a=a+1*2;
                if a>0 {
                    foo(a, c);
                }
                while a!=0 {
                    a=a-1;
                }
                return a;
            }

            fn foo(x:i32, y:i32) {
                x+y;
            }
            "#,
        );
    }

    #[test]
    fn parses_extended_constructs() {
        let ast = parse_ok(
            r#"
            fn extra(mut a:i32) -> i32 {
                let mut arr:[i32;3]=[1,2,3];
                let mut pair:(i32,i32)=(arr[0], a);
                let r:&mut i32=&mut a;
                let v={
                    let t=pair.0;
                    if t>0 { t } else { 0 }
                };
                for mut i in 0..a {
                    if i==2 { continue; }
                }
                let b=loop {
                    break v;
                };
                return b;
            }
            "#,
        );
        serde_json::to_string_pretty(&ast).unwrap();
    }

    #[test]
    fn reports_assignment_target_errors() {
        let result = lex(r#"
            fn bad() {
                1=2;
            }
            "#);
        let error = parse_tokens(&result.tokens).unwrap_err();
        assert!(error.message.contains("not assignable"));
    }

    #[test]
    fn accepts_end_marker() {
        parse_ok(
            r#"
            fn done() {
                return;
            }
            #
            "#,
        );
    }
}
