use axum::{
    extract::Json,
    http::{header, StatusCode},
    response::{IntoResponse, Response},
    routing::{get, post},
    Router,
};
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;

#[derive(Deserialize)]
struct AnalyzeRequest {
    source: String,
}

#[derive(Serialize)]
struct AnalyzeResponse {
    tokens: Vec<TokenView>,
    ast: Option<serde_json::Value>,
    #[serde(rename = "lexerErrors")]
    lexer_errors: Vec<String>,
    #[serde(rename = "parseError")]
    parse_error: Option<String>,
}

#[derive(Serialize)]
struct TokenView {
    line: usize,
    col: usize,
    value: String,
    #[serde(rename = "type")]
    type_: String,
    type_enum: String,
}

async fn serve_index() -> Response {
        let html = r#"<!doctype html>
<html lang="en">
    <head>
        <meta charset="UTF-8" />
        <meta name="viewport" content="width=device-width, initial-scale=1.0" />
        <title>Easy Server</title>
    </head>
    <body>
        <h1>Easy Server is running</h1>
        <p>Use POST /api/analyze with JSON: {"source": "..."}</p>
    </body>
</html>"#;
    (StatusCode::OK, [(header::CONTENT_TYPE, "text/html; charset=utf-8")], html).into_response()
}

async fn analyze(Json(req): Json<AnalyzeRequest>) -> Json<AnalyzeResponse> {
    let lex_result = easy_lexer::lex(&req.source);

    let tokens: Vec<TokenView> = lex_result
        .tokens
        .iter()
        .map(|token| TokenView {
            line: token.position.line,
            col: token.position.column,
            value: token.lexeme.clone(),
            type_: classify_token(&token.kind),
            type_enum: format!("{:?}", token.kind),
        })
        .collect();

    let lexer_errors: Vec<String> = lex_result
        .errors
        .iter()
        .map(|error| {
            format!(
                "[词法错误] {}:{}: {}",
                error.position.line, error.position.column, error.message
            )
        })
        .collect();

    let (ast, parse_error) = if lex_result.errors.is_empty() {
        match easy_parser::parse_program_ast(&lex_result.tokens) {
            Ok(program) => (
                serde_json::to_value(program).ok(),
                None,
            ),
            Err(error) => (None, Some(format!("{error}"))),
        }
    } else {
        (None, None)
    };

    Json(AnalyzeResponse {
        tokens,
        ast,
        lexer_errors,
        parse_error,
    })
}

fn classify_token(kind: &easy_lexer::TokenKind) -> String {
    use easy_lexer::TokenKind;

    match kind {
        TokenKind::Keyword(_) => "关键字".to_string(),
        TokenKind::Identifier => "标识符".to_string(),
        TokenKind::Number => "数字".to_string(),
        TokenKind::Assign
        | TokenKind::Plus
        | TokenKind::Minus
        | TokenKind::Star
        | TokenKind::Slash
        | TokenKind::EqualEqual
        | TokenKind::Greater
        | TokenKind::GreaterEqual
        | TokenKind::Less
        | TokenKind::LessEqual
        | TokenKind::NotEqual
        | TokenKind::Ampersand => "算符".to_string(),
        TokenKind::Semicolon | TokenKind::Colon | TokenKind::Comma => "分隔符".to_string(),
        TokenKind::LParen
        | TokenKind::RParen
        | TokenKind::LBrace
        | TokenKind::RBrace
        | TokenKind::LBracket
        | TokenKind::RBracket => "界符".to_string(),
        TokenKind::Arrow | TokenKind::Dot | TokenKind::DotDot => "特殊符号".to_string(),
        TokenKind::EndMarker => "结束符".to_string(),
    }
}

#[tokio::main]
async fn main() {
    let app = Router::new()
        .route("/", get(serve_index))
        .route("/api/analyze", post(analyze));

    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    println!("Server running at http://{}", addr);
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}