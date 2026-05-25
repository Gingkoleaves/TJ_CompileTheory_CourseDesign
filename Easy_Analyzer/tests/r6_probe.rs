//! 第六轮探针：聚焦 Lexer / Parser / 跨 crate panic / 边界
//!
//! 探针目的：验证候选 BUG 是否真实可复现。
//! 不作为修复测试。

use easy_lexer::lex;
use easy_parser::parse_program_ast;

fn analyze_src(src: &str) -> easy_analyzer::AnalysisResult {
    let lex_result = lex(src);
    assert!(lex_result.errors.is_empty(), "lex errs: {:?}", lex_result.errors);
    let prog = parse_program_ast(&lex_result.tokens).expect("parse fail");
    easy_analyzer::analyze(&prog)
}

// ── L-1: 列号在多字节 UTF-8 字符后偏移（按字节而非字符计） ──
#[test]
fn r6_l1_column_after_utf8_byte_drift() {
    // "中" 在 UTF-8 是 3 字节；按理 column 应该 +1，实际看是否 +1
    let src = "中a";
    let r = lex(src);
    // "中" 触发未识别错误 3 次（3 个字节），column 1/2/3
    // "a" 应该在 column 4（按字节）或 column 2（按字符）
    let positions: Vec<_> = r.tokens.iter().map(|t| (t.position.line, t.position.column)).collect();
    let err_positions: Vec<_> = r.errors.iter().map(|e| (e.position.line, e.position.column)).collect();
    eprintln!("tokens at {:?}", positions);
    eprintln!("errors at {:?}", err_positions);
    // 暴露 BUG：行/列号按字节，'a' 的列号不是 2
    // 仅打印观察，不强断言
}

// ── L-2: CRLF 行尾时 column 不正确重置 ──
#[test]
fn r6_l2_crlf_line_column() {
    let src = "a\r\nb";  // CR(0x0D) 不在 advance 的 \n 分支，但跟着 \n 才换行
    let r = lex(src);
    let positions: Vec<_> = r.tokens.iter().map(|t| (t.lexeme.clone(), t.position.line, t.position.column)).collect();
    eprintln!("CRLF tokens: {:?}", positions);
    // 期望：a 在 1:1，b 在 2:1
    // 实际：advance 在 \r 时只 column+=1，碰到 \n 时 line+=1 column=1
    // 所以 b 实际在 2:1，因为下一字符 column 已被 \n 重置。OK 看起来对。
}

// ── L-3: 空文件 / 仅注释 ──
#[test]
fn r6_l3_empty_and_comment_only() {
    let r1 = lex("");
    assert!(r1.tokens.is_empty() && r1.errors.is_empty());
    let r2 = lex("// just a comment");
    assert!(r2.tokens.is_empty() && r2.errors.is_empty());
    let r3 = lex("/* unterminated");
    assert!(r3.errors.len() == 1);
}

// ── L-4: 嵌套块注释 ──
#[test]
fn r6_l4_nested_block_comments() {
    let r = lex("/* outer /* inner */ still outer */ fn");
    let kinds: Vec<_> = r.tokens.iter().map(|t| t.lexeme.clone()).collect();
    eprintln!("nested comment tokens: {:?}", kinds);
    assert!(r.errors.is_empty(), "should support nesting");
    assert_eq!(kinds, vec!["fn"]);
}

// ── L-5: 数字之后紧跟标识符（无空格）──
#[test]
fn r6_l5_number_then_identifier_no_space() {
    let r = lex("123abc");
    let toks: Vec<_> = r.tokens.iter().map(|t| (t.kind.to_string(), t.lexeme.clone())).collect();
    eprintln!("123abc → {:?}", toks);
    // 期望两 token: number(123), identifier(abc)；rustc 会拒（数字后缀），本课程没这要求
}

// ── L-6: 极长数字字面量是否影响性能/触发问题 ──
#[test]
fn r6_l6_very_long_number() {
    let s = "1".repeat(10_000);
    let src = format!("fn main(){{ let a:i32={}; }}", s);
    let r = lex(&src);
    assert!(r.errors.is_empty());
    // 不 panic 即 OK
}

// ── P-1: x.0.1 嵌套字段访问 ──
#[test]
fn r6_p1_nested_tuple_field() {
    let src = r#"
        fn main(){
            let t:((i32,i32),i32) = ((1,2), 3);
            let a:i32 = t.0.0;
        }
    "#;
    let r = lex(src);
    assert!(r.errors.is_empty());
    match parse_program_ast(&r.tokens) {
        Ok(p) => eprintln!("p1 parsed: {} fns", p.functions.len()),
        Err(e) => eprintln!("p1 parse error: {}", e),
    }
}

// ── P-2: 数组类型长度为负？lexer 不接受负号，所以试 [i32;-1]，应报 parse error ──
#[test]
fn r6_p2_array_negative_length() {
    let src = "fn main(){ let a:[i32;-1] = []; }";
    let r = lex(src);
    assert!(r.errors.is_empty());
    let res = parse_program_ast(&r.tokens);
    eprintln!("array neg length parse: {:?}", res.is_ok());
    // 期望：parse error（minus 后期望 array length 是 number）
}

// ── P-3: ()()  空元组调用 ──
#[test]
fn r6_p3_calling_unit() {
    let src = "fn main(){ let a:i32 = ()(); }";
    let r = lex(src);
    let res = parse_program_ast(&r.tokens);
    eprintln!("call on unit: {:?}", res.is_ok());
    // 期望：parse error（is_callable 不允许 Tuple）或语义错
}

// ── P-4: if 后省略 then-block 直接 else ──
#[test]
fn r6_p4_else_without_then_block() {
    let src = "fn main(){ if 1 else {} }";
    let r = lex(src);
    let res = parse_program_ast(&r.tokens);
    eprintln!("if without then: {:?}", res.err());
}

// ── P-5: 空块作 if 条件（block expression as cond）──
#[test]
fn r6_p5_block_as_condition() {
    let src = "fn main(){ if {} { return; } }";
    let r = lex(src);
    let res = parse_program_ast(&r.tokens);
    eprintln!("block as cond: {:?}", res.is_ok());
}

// ── P-6: 多个连续函数定义，无空行 ──
#[test]
fn r6_p6_consecutive_functions() {
    let src = "fn a(){}fn b(){}fn c(){}";
    let r = lex(src);
    assert!(r.errors.is_empty());
    let prog = parse_program_ast(&r.tokens).expect("parse");
    assert_eq!(prog.functions.len(), 3);
}

// ── P-7: # 之后再写函数 ──
#[test]
fn r6_p7_endmarker_followed_by_fn() {
    let src = "fn a(){} # fn b(){}";
    let r = lex(src);
    let res = parse_program_ast(&r.tokens);
    eprintln!("endmarker mid: {:?}", res.is_err());
    // 期望：parse error "unexpected tokens after #"
}

// ── P-8: 函数返回 -> () 显式 unit 返回类型 ──
#[test]
fn r6_p8_explicit_unit_return_type() {
    let src = "fn main() -> () { }";
    let r = lex(src);
    let res = parse_program_ast(&r.tokens);
    match res {
        Ok(_) => {
            let outcome = analyze_src(src);
            eprintln!("explicit unit return: {} errs", outcome.semantic_errors.len());
        }
        Err(e) => eprintln!("parse err: {}", e),
    }
}

// ── P-9: 元组单元素 (x,) 这种语法 ──
#[test]
fn r6_p9_singleton_tuple() {
    let src = "fn main(){ let a:(i32,) = (1,); }";
    let r = lex(src);
    let res = parse_program_ast(&r.tokens);
    eprintln!("singleton tuple parse: {:?}", res.is_ok());
}

// ── P-10: chained else if / else if / else 深层 ──
#[test]
fn r6_p10_deep_else_if() {
    let src = "fn main(){ if 1>0 {} else if 2>0 {} else if 3>0 {} else {} }";
    let r = lex(src);
    parse_program_ast(&r.tokens).expect("deep else if");
}

// ── P-11: 不带 fn 的顶层语句直接喂 ──
#[test]
fn r6_p11_no_fn_decl() {
    let r = lex("let x = 1;");
    let res = parse_program_ast(&r.tokens);
    eprintln!("no fn decl: {:?}", res.is_err());
    // 期望 parse error
}

// ── P-12: tuple field with number lexeme too large ──
#[test]
fn r6_p12_huge_tuple_field() {
    let src = "fn main(){ let t:(i32,i32) = (1,2); let a:i32 = t.99999999999999999999; }";
    let r = lex(src);
    let res = parse_program_ast(&r.tokens);
    match res {
        Ok(prog) => {
            let outcome = easy_analyzer::analyze(&prog);
            let msgs: Vec<_> = outcome.semantic_errors.iter().map(|e| e.message.clone()).collect();
            eprintln!("huge tuple field semantic: {:?}", msgs);
        }
        Err(e) => eprintln!("parse err: {}", e),
    }
}

// ── P-13: 函数参数列表后尾随逗号 fn f(a:i32,) {} ──
#[test]
fn r6_p13_trailing_comma_in_params() {
    let src = "fn f(a:i32,) {} fn main(){}";
    let r = lex(src);
    let res = parse_program_ast(&r.tokens);
    eprintln!("trailing comma in params: {:?}", res.is_ok());
}

// ── P-14: 极深嵌套括号触发栈溢出？──
#[test]
fn r6_p14_deep_paren_nesting() {
    let n = 5_000;
    let src = format!("fn main(){{ let a:i32 = {}1{}; }}", "(".repeat(n), ")".repeat(n));
    let r = lex(&src);
    // Parser 递归调用，深度 5000 可能 stack overflow
    // 仅作为观察，标记为 ignore 防止误失败
}
