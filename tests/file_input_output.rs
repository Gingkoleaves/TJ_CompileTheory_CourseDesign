//! Integration test for file-based parser input and output.
//!
//! It reuses the lexer project's fixed input and expected token output, then
//! checks that the parser binary appends a successful syntax-analysis result.

use std::fs;
use std::path::Path;
use std::process::Command;

#[test]
fn reads_lexer_input_file_and_produces_parser_output() {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let lexer_data_dir = manifest_dir.join("../My_Lexer/tests/data");
    let parser_data_dir = manifest_dir.join("tests/data");
    let input_path = lexer_data_dir.join("input.rs");
    let lexer_expected_path = lexer_data_dir.join("expected_output.txt");
    let output_path = parser_data_dir.join("output.txt");

    let output = Command::new(env!("CARGO_BIN_EXE_My_Parser"))
        .arg(&input_path)
        .output()
        .expect("failed to run parser binary");

    assert!(
        output.status.success(),
        "parser exited with status {:?}, stderr: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr)
    );

    fs::write(&output_path, &output.stdout).expect("failed to write parser output file");

    let actual = fs::read_to_string(&output_path).expect("failed to read parser output file");
    let expected = format!(
        "{}syntax analysis succeeded\n",
        fs::read_to_string(&lexer_expected_path).expect("failed to read lexer expected output")
    );

    assert_eq!(actual, expected);
}
