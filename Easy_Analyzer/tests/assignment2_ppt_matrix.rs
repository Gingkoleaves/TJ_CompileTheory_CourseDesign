use easy_analyzer::{analyze, AnalysisResult};
use easy_lexer::lex;
use easy_parser::parse_program_ast;

fn analyze_src(src: &str) -> Result<AnalysisResult, String> {
    let lexed = lex(src);
    if !lexed.errors.is_empty() {
        return Err(format!("lexer errors: {:?}", lexed.errors));
    }
    let program = parse_program_ast(&lexed.tokens).map_err(|e| format!("parse error: {e}"))?;
    Ok(analyze(&program))
}

fn assert_ok(src: &str) -> AnalysisResult {
    let result = analyze_src(src).expect("source should lex and parse");
    assert!(
        result.semantic_errors.is_empty(),
        "expected semantic success, got {:?}",
        result.semantic_errors
    );
    result
}

fn assert_semantic_error(src: &str) {
    let result = analyze_src(src).expect("source should lex and parse");
    assert!(
        !result.semantic_errors.is_empty(),
        "expected at least one semantic error"
    );
}

fn assert_parse_error(src: &str) {
    assert!(analyze_src(src).is_err(), "expected lex or parse failure");
}

#[test]
fn ppt_required_rules_generate_core_ir() {
    let result = assert_ok(
        r#"
        fn calc(mut a:i32, b:i32) -> i32 {
            ;;;
            let mut acc:i32;
            acc = a + b * 2;
            if acc > 10 {
                acc = acc - 1;
            }
            while acc != 0 {
                acc = acc - 1;
            }
            return acc;
        }

        fn main() {
            calc(1, 2);
        }
        "#,
    );

    let ops: Vec<&str> = result.quadruples.iter().map(|q| q.op.as_str()).collect();
    for op in [
        "FUNC",
        "PARAM_DECL",
        "=",
        "*",
        "+",
        ">",
        "!=",
        "IF_FALSE",
        "LABEL",
        "GOTO",
        "CALL",
        "RETURN",
        "END_FUNC",
    ] {
        assert!(ops.contains(&op), "missing IR op {op}; ops were {ops:?}");
    }
}

#[test]
fn ppt_required_return_variable_and_call_errors_are_reported() {
    assert_semantic_error("fn f()->i32 { return; } fn main(){}");
    assert_semantic_error("fn f(){ return 1; } fn main(){}");
    assert_semantic_error("fn main(){ a = 32; }");
    assert_semantic_error("fn main(){ let mut a:i32; let b:i32 = a; }");
    assert_semantic_error("fn main(){ let mut a:i32 = 0; a = 1 == 1; }");
    assert_semantic_error("fn main(){ let mut b; }");
    assert_semantic_error("fn f(){} fn main(){ f(1); }");
    assert_semantic_error("fn f(a:i32){} fn main(){ f(1 == 1); }");
    assert_semantic_error("fn f(){} fn main(){ let x = f(); }");
}

#[test]
fn ppt_extended_rules_are_largely_supported() {
    assert_ok(
        r#"
        fn main() {
            let mut a:i32 = 1;
            let p:&mut i32 = &mut a;
            *p = 2;

            let mut arr:[i32;3] = [1, 2, 3];
            arr[1] = arr[0] + 2;

            let mut pair:(i32, i32) = (arr[0], arr[1]);
            pair.0 = pair.1;

            let mut sum:i32 = 0;
            for mut i in 0..3 {
                if i == 1 {
                    continue;
                }
                sum = sum + i;
            }

            let from_loop:i32 = loop {
                break sum;
            };

            let from_if:i32 = if from_loop > 0 { from_loop } else { 0 };
        }
        "#,
    );
}

#[test]
fn ppt_extended_rule_errors_are_reported() {
    assert_semantic_error("fn main(){ let a:i32 = 1; let p = &mut a; }");
    assert_semantic_error("fn main(){ let mut a:i32 = 1; let p = &a; let q = &mut a; }");
    assert_semantic_error("fn main(){ let a:i32 = 1; let p = &a; *p = 2; }");
    assert_semantic_error("fn main(){ let a:[i32;0] = []; }");
    assert_semantic_error("fn main(){ let a = []; }");
    assert_semantic_error("fn main(){ let a:[i32;2] = [1,2,3]; }");
    assert_semantic_error("fn main(){ let a:[i32;3] = [1,2,3]; a[0] = 4; }");
    assert_semantic_error("fn main(){ let a = [1,2,3]; let x = a[3]; }");
    assert_semantic_error("fn main(){ let a:(i32,i32,i32) = (1,2,3); let x = a.3; }");
    assert_semantic_error("fn main(){ break 1; }");
    assert_semantic_error("fn main(){ let x:i32 = loop { break; break 1; }; }");
}

#[test]
fn ppt_array_and_tuple_elements_remain_usable() {
    assert_ok(
        r#"
        fn main() {
            let mut a:[i32;3] = [1,2,3];
            let b:i32 = a[0] + a[1] + a[2];
            a[0] = b;

            let mut t:(i32,i32,i32) = (1,2,3);
            let c:i32 = t.0 + t.1 + t.2;
            t.0 = c;
        }
        "#,
    );
}

#[test]
fn optional_range_values_are_not_general_expressions() {
    assert_parse_error("fn main(){ let r = 0..3; for i in r { let x:i32 = i; } }");
}
