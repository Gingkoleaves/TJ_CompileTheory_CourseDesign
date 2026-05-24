use easy_analyzer::analyze;
use easy_lexer::lex;
use easy_parser::parse_program_ast;

fn run(src: &str) -> easy_analyzer::AnalysisResult {
    let lex = lex(src);
    assert!(lex.errors.is_empty(), "lexer errors: {:?}", lex.errors);
    let program = parse_program_ast(&lex.tokens).expect("parse failed");
    analyze(&program)
}

fn assert_ok(src: &str) {
    let result = run(src);
    assert!(
        result.semantic_errors.is_empty(),
        "expected no semantic errors, got {:?}",
        result.semantic_errors
    );
}

fn assert_error_contains(src: &str, needles: &[&str]) {
    let result = run(src);
    assert!(
        result
            .semantic_errors
            .iter()
            .any(|e| needles.iter().all(|needle| e.message.contains(needle))),
        "expected error containing {:?}, got {:?}",
        needles,
        result.semantic_errors
    );
}

#[test]
fn mandatory_valid_program_generates_core_ir() {
    let result = run(
        r#"
        fn add(mut a:i32, b:i32) -> i32 {
            let mut acc:i32;
            acc = a + b * 2;
            if acc > 10 { acc = acc - 1; }
            while acc != 0 { acc = acc - 1; }
            return acc;
        }
        fn main(){ let answer:i32 = add(1, 2); }
        "#,
    );
    assert!(result.semantic_errors.is_empty(), "{:?}", result.semantic_errors);
    let ops: Vec<&str> = result.quadruples.iter().map(|q| q.op.as_str()).collect();
    for op in ["FUNC", "PARAM_DECL", "*", "+", ">", "IF_FALSE", "LABEL", "GOTO", "CALL", "RETURN"] {
        assert!(ops.contains(&op), "missing op {op}, got {:?}", ops);
    }
}

#[test]
fn required_return_and_variable_errors() {
    assert_error_contains("fn f()->i32{ return; }", &["return"]);
    assert_error_contains("fn f(){ return 1; }", &["return", "不能带表达式"]);
    assert_error_contains("fn main(){ a = 32; }", &["未声明"]);
    assert_error_contains("fn main(){ let mut a:i32; let mut b:i32=a; }", &["赋值前"]);
    assert_error_contains("fn main(){ let mut a:i32=0; a=1==1; }", &["类型", "不匹配"]);
    assert_error_contains("fn main(){ let mut b; }", &["无法推断"]);
}

#[test]
fn required_function_and_loop_errors() {
    assert_error_contains("fn f(){} fn main(){ f(1); }", &["形参数量"]);
    assert_error_contains("fn f(a:i32){} fn main(){ f(1==1); }", &["实参类型"]);
    assert_error_contains("fn f(){} fn main(){ let a=f(); }", &["无返回值"]);
    assert_error_contains("fn main(){ break; }", &["循环体内"]);
    assert_error_contains("fn main(){ continue; }", &["循环体内"]);
}

#[test]
fn extended_valid_program_generates_ir() {
    assert_ok(
        r#"
        fn extra(mut a:i32) -> i32 {
            let mut arr:[i32;3]=[1,2,3];
            let mut pair:(i32,i32)=(arr[0], a);
            let r:&mut i32=&mut a;
            *r = pair.1;
            for mut i in 0..a {
                if i==2 { continue; }
            }
            let b=loop { break pair.0; };
            return b;
        }
        "#,
    );
}

#[test]
fn extended_reference_errors() {
    assert_error_contains("fn main(){ let mut a:i32=1; let b=&a; let c=&mut a; }", &["可变引用"]);
    assert_error_contains("fn main(){ let mut a:i32=1; let b=&mut a; let c=&a; }", &["可变引用"]);
    assert_error_contains("fn main(){ let mut a:i32=1; let b=&mut a; let c=&mut a; }", &["可变引用"]);
    assert_error_contains("fn main(){ let a:i32=1; let b=&mut a; }", &["不可变变量"]);
    assert_error_contains("fn main(){ let a:i32=1; let b=*a; }", &["解引用"]);
    assert_error_contains("fn main(){ let a:i32=1; let b=&a; *b=2; }", &["不可变引用"]);
}

#[test]
fn deeper_composite_and_loop_errors() {
    assert_error_contains("fn main(){ let mut a:[i32;2]=[1,2]; a[0]=1==1; }", &["赋值目标类型", "不匹配"]);
    assert_error_contains("fn main(){ let mut a:(i32,i32)=(1,2); a.0=1==1; }", &["赋值目标类型", "不匹配"]);
    assert_error_contains("fn main(){ let x:i32=loop { if 1 { break 1; } else { break (); } }; }", &["break", "类型不一致"]);
}

#[test]
fn extended_array_errors() {
    assert_error_contains("fn main(){ let a:[i32;2]=[1,2,3]; }", &["[i32; 2]", "[i32; 3]", "不匹配"]);
    assert_error_contains("fn main(){ let a:[[i32;1];1]=[1]; }", &["[[i32; 1]; 1]", "[i32; 1]", "不匹配"]);
    assert_error_contains("fn main(){ let a=[1,2,3]; let b=a[a]; }", &["下标", "i32"]);
    assert_error_contains("fn main(){ let a=[1,2,3]; let b=a[3]; }", &["越界"]);
    assert_error_contains("fn main(){ let a:[i32;3]=[1,2,3]; a[0]=4; }", &["不可变"]);
}

#[test]
fn extended_tuple_errors() {
    assert_error_contains("fn main(){ let a:(i32,i32)=(1,2,3); }", &["(i32, i32)", "(i32, i32, i32)", "不匹配"]);
    assert_error_contains("fn main(){ let a:((),)=(1,); }", &["((),)", "(i32,)", "不匹配"]);
    assert_error_contains("fn main(){ let a=(1,2,3); let b=a.3; }", &["越界"]);
    assert_error_contains("fn main(){ let a:(i32,i32,i32)=(1,2,3); a.0=4; }", &["不可变"]);
}
