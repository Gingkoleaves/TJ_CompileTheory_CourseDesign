//! Integration test for file-based lexer input and output.
//!
//! It feeds the binary from a fixed input file and writes the actual output
//! to a fixed project file for manual inspection.

use std::fs;
use std::path::Path;
use std::process::Command;

#[test]
fn reads_from_input_file_and_writes_output_file() {
    let data_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/data");
    let input_path = data_dir.join("input.rs");
    let expected_path = data_dir.join("expected_output.txt");
    let output_path = data_dir.join("output.txt");

    let output = Command::new(env!("CARGO_BIN_EXE_My_Lexer"))
        .arg(&input_path)
        .output()
        .expect("failed to run lexer binary");

    assert!(
        output.status.success(),
        "lexer exited with status {:?}, stderr: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr)
    );

    fs::write(&output_path, &output.stdout).expect("failed to write output file");

    let actual =
        normalize_newlines(&fs::read_to_string(&output_path).expect("failed to read output file"));
    let expected = normalize_newlines(
        &fs::read_to_string(&expected_path).expect("failed to read expected output"),
    );
    assert_eq!(actual, expected);
}

fn normalize_newlines(text: &str) -> String {
    text.replace("\r\n", "\n")
}
