//! A small lexer library for the course project.
//!
//! It tokenizes a Rust-like input language and reports lexical errors with
//! line and column information.

use std::fmt;

/// Line and column information for a token or lexical error.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Position {
    pub line: usize,
    pub column: usize,
}

/// A token produced by the lexer, including its kind, source text, and position.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Token {
    pub kind: TokenKind,
    pub lexeme: String,
    pub position: Position,
}

/// All token categories recognized by the lexer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TokenKind {
    Keyword(Keyword),
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

/// The reserved keywords required by the assignment.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Keyword {
    I32,
    Let,
    If,
    Else,
    While,
    Return,
    Mut,
    Fn,
    For,
    In,
    Loop,
    Break,
    Continue,
}

/// A lexical error with its message and source position.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LexError {
    pub message: String,
    pub position: Position,
}

/// The complete lexer result for one input string.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct LexResult {
    pub tokens: Vec<Token>,
    pub errors: Vec<LexError>,
}

impl fmt::Display for TokenKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TokenKind::Keyword(keyword) => write!(f, "keyword({keyword})"),
            TokenKind::Identifier => write!(f, "identifier"),
            TokenKind::Number => write!(f, "number"),
            TokenKind::Assign => write!(f, "assign"),
            TokenKind::Plus => write!(f, "plus"),
            TokenKind::Minus => write!(f, "minus"),
            TokenKind::Star => write!(f, "star"),
            TokenKind::Slash => write!(f, "slash"),
            TokenKind::EqualEqual => write!(f, "equal_equal"),
            TokenKind::Greater => write!(f, "greater"),
            TokenKind::GreaterEqual => write!(f, "greater_equal"),
            TokenKind::Less => write!(f, "less"),
            TokenKind::LessEqual => write!(f, "less_equal"),
            TokenKind::NotEqual => write!(f, "not_equal"),
            TokenKind::Ampersand => write!(f, "ampersand"),
            TokenKind::LParen => write!(f, "l_paren"),
            TokenKind::RParen => write!(f, "r_paren"),
            TokenKind::LBrace => write!(f, "l_brace"),
            TokenKind::RBrace => write!(f, "r_brace"),
            TokenKind::LBracket => write!(f, "l_bracket"),
            TokenKind::RBracket => write!(f, "r_bracket"),
            TokenKind::Semicolon => write!(f, "semicolon"),
            TokenKind::Colon => write!(f, "colon"),
            TokenKind::Comma => write!(f, "comma"),
            TokenKind::Arrow => write!(f, "arrow"),
            TokenKind::Dot => write!(f, "dot"),
            TokenKind::DotDot => write!(f, "dot_dot"),
            TokenKind::EndMarker => write!(f, "end_marker"),
        }
    }
}

impl fmt::Display for Keyword {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let text = match self {
            Keyword::I32 => "i32",
            Keyword::Let => "let",
            Keyword::If => "if",
            Keyword::Else => "else",
            Keyword::While => "while",
            Keyword::Return => "return",
            Keyword::Mut => "mut",
            Keyword::Fn => "fn",
            Keyword::For => "for",
            Keyword::In => "in",
            Keyword::Loop => "loop",
            Keyword::Break => "break",
            Keyword::Continue => "continue",
        };
        write!(f, "{text}")
    }
}

/// Tokenizes the input source and returns all tokens together with
/// any lexical errors found during the scan.
///
/// The lexer keeps scanning after an error whenever possible so callers can
/// inspect a fuller error list from a single run.
pub fn lex(input: &str) -> LexResult {
    let mut lexer = Lexer::new(input);
    lexer.lex_all();
    LexResult {
        tokens: lexer.tokens,
        errors: lexer.errors,
    }
}

struct Lexer<'a> {
    input: &'a str,
    index: usize,
    line: usize,
    column: usize,
    tokens: Vec<Token>,
    errors: Vec<LexError>,
}

impl<'a> Lexer<'a> {
    fn new(input: &'a str) -> Self {
        Self {
            input,
            index: 0,
            line: 1,
            column: 1,
            tokens: Vec::new(),
            errors: Vec::new(),
        }
    }

    fn lex_all(&mut self) {
        while self.index < self.input.len() {
            self.skip_ignored();
            if self.index >= self.input.len() {
                break;
            }

            let start = self.position();
            let start_index = self.index;
            let Some(byte) = self.peek() else {
                break;
            };

            if is_identifier_start(byte) {
                self.advance();
                while let Some(next) = self.peek() {
                    if is_identifier_continue(next) {
                        self.advance();
                    } else {
                        break;
                    }
                }
                let lexeme = &self.input[start_index..self.index];
                let kind = match keyword_from(lexeme) {
                    Some(keyword) => TokenKind::Keyword(keyword),
                    None => TokenKind::Identifier,
                };
                self.push_token(kind, lexeme, start);
                continue;
            }

            if byte.is_ascii_digit() {
                self.advance();
                while let Some(next) = self.peek() {
                    if next.is_ascii_digit() {
                        self.advance();
                    } else {
                        break;
                    }
                }
                let lexeme = &self.input[start_index..self.index];
                self.push_token(TokenKind::Number, lexeme, start);
                continue;
            }

            if let Some((kind, width)) = self.match_compound_token() {
                for _ in 0..width {
                    self.advance();
                }
                let lexeme = &self.input[start_index..self.index];
                self.push_token(kind, lexeme, start);
                continue;
            }

            let kind = match byte {
                b'=' => Some(TokenKind::Assign),
                b'+' => Some(TokenKind::Plus),
                b'-' => Some(TokenKind::Minus),
                b'*' => Some(TokenKind::Star),
                b'/' => Some(TokenKind::Slash),
                b'>' => Some(TokenKind::Greater),
                b'<' => Some(TokenKind::Less),
                b'&' => Some(TokenKind::Ampersand),
                b'(' => Some(TokenKind::LParen),
                b')' => Some(TokenKind::RParen),
                b'{' => Some(TokenKind::LBrace),
                b'}' => Some(TokenKind::RBrace),
                b'[' => Some(TokenKind::LBracket),
                b']' => Some(TokenKind::RBracket),
                b';' => Some(TokenKind::Semicolon),
                b':' => Some(TokenKind::Colon),
                b',' => Some(TokenKind::Comma),
                b'.' => Some(TokenKind::Dot),
                b'#' => Some(TokenKind::EndMarker),
                _ => None,
            };

            if let Some(kind) = kind {
                self.advance();
                let lexeme = &self.input[start_index..self.index];
                self.push_token(kind, lexeme, start);
            } else {
                let ch = byte as char;
                self.errors.push(LexError {
                    message: format!("unexpected character `{ch}`"),
                    position: start,
                });
                self.advance();
            }
        }
    }

    fn skip_ignored(&mut self) {
        loop {
            let Some(byte) = self.peek() else {
                return;
            };

            if byte.is_ascii_whitespace() {
                self.advance();
                continue;
            }

            if byte == b'/' && self.peek_next() == Some(b'/') {
                self.advance();
                self.advance();
                while let Some(next) = self.peek() {
                    self.advance();
                    if next == b'\n' {
                        break;
                    }
                }
                continue;
            }

            if byte == b'/' && self.peek_next() == Some(b'*') {
                let start = self.position();
                self.advance();
                self.advance();
                let mut terminated = false;
                while let Some(next) = self.peek() {
                    if next == b'*' && self.peek_next() == Some(b'/') {
                        self.advance();
                        self.advance();
                        terminated = true;
                        break;
                    }
                    self.advance();
                }
                if !terminated {
                    self.errors.push(LexError {
                        message: "unterminated block comment".to_string(),
                        position: start,
                    });
                    return;
                }
                continue;
            }

            return;
        }
    }

    fn match_compound_token(&self) -> Option<(TokenKind, usize)> {
        match (self.peek(), self.peek_next()) {
            (Some(b'-'), Some(b'>')) => Some((TokenKind::Arrow, 2)),
            (Some(b'='), Some(b'=')) => Some((TokenKind::EqualEqual, 2)),
            (Some(b'>'), Some(b'=')) => Some((TokenKind::GreaterEqual, 2)),
            (Some(b'<'), Some(b'=')) => Some((TokenKind::LessEqual, 2)),
            (Some(b'!'), Some(b'=')) => Some((TokenKind::NotEqual, 2)),
            (Some(b'.'), Some(b'.')) => Some((TokenKind::DotDot, 2)),
            _ => None,
        }
    }

    fn push_token(&mut self, kind: TokenKind, lexeme: &str, position: Position) {
        self.tokens.push(Token {
            kind,
            lexeme: lexeme.to_string(),
            position,
        });
    }

    fn peek(&self) -> Option<u8> {
        self.input.as_bytes().get(self.index).copied()
    }

    fn peek_next(&self) -> Option<u8> {
        self.input.as_bytes().get(self.index + 1).copied()
    }

    fn advance(&mut self) -> Option<u8> {
        let byte = self.peek()?;
        self.index += 1;
        if byte == b'\n' {
            self.line += 1;
            self.column = 1;
        } else {
            self.column += 1;
        }
        Some(byte)
    }

    fn position(&self) -> Position {
        Position {
            line: self.line,
            column: self.column,
        }
    }
}

fn is_identifier_start(byte: u8) -> bool {
    byte.is_ascii_alphabetic() || byte == b'_'
}

fn is_identifier_continue(byte: u8) -> bool {
    is_identifier_start(byte) || byte.is_ascii_digit()
}

fn keyword_from(lexeme: &str) -> Option<Keyword> {
    match lexeme {
        "i32" => Some(Keyword::I32),
        "let" => Some(Keyword::Let),
        "if" => Some(Keyword::If),
        "else" => Some(Keyword::Else),
        "while" => Some(Keyword::While),
        "return" => Some(Keyword::Return),
        "mut" => Some(Keyword::Mut),
        "fn" => Some(Keyword::Fn),
        "for" => Some(Keyword::For),
        "in" => Some(Keyword::In),
        "loop" => Some(Keyword::Loop),
        "break" => Some(Keyword::Break),
        "continue" => Some(Keyword::Continue),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keyword_suffix_stays_identifier() {
        let result = lex("if123");
        assert!(result.errors.is_empty());
        assert_eq!(result.tokens.len(), 1);
        assert_eq!(result.tokens[0].kind, TokenKind::Identifier);
        assert_eq!(result.tokens[0].lexeme, "if123");
    }

    #[test]
    fn keyword_operator_number_are_split() {
        let result = lex("if=123");
        assert!(result.errors.is_empty());
        assert_eq!(
            result
                .tokens
                .iter()
                .map(|token| &token.kind)
                .collect::<Vec<_>>(),
            vec![
                &TokenKind::Keyword(Keyword::If),
                &TokenKind::Assign,
                &TokenKind::Number,
            ]
        );
    }

    #[test]
    fn comments_are_skipped() {
        let result = lex("let /* block */ mut // line\n a");
        assert!(result.errors.is_empty());
        assert_eq!(
            result
                .tokens
                .iter()
                .map(|token| token.lexeme.as_str())
                .collect::<Vec<_>>(),
            vec!["let", "mut", "a"]
        );
    }

    #[test]
    fn longest_match_tokens_win() {
        let result = lex("-> == >= <= != .. .");
        assert!(result.errors.is_empty());
        assert_eq!(
            result
                .tokens
                .iter()
                .map(|token| &token.kind)
                .collect::<Vec<_>>(),
            vec![
                &TokenKind::Arrow,
                &TokenKind::EqualEqual,
                &TokenKind::GreaterEqual,
                &TokenKind::LessEqual,
                &TokenKind::NotEqual,
                &TokenKind::DotDot,
                &TokenKind::Dot,
            ]
        );
    }

    #[test]
    fn unterminated_block_comment_reports_error() {
        let result = lex("/* oops");
        assert!(result.tokens.is_empty());
        assert_eq!(result.errors.len(), 1);
        assert!(result.errors[0].message.contains("unterminated"));
    }
}
