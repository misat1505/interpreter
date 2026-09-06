use std::io::BufReader;

use raptor_lib::backend::llvm::OverflowPolicy;

use crate::common::{
    capture_compiled_output_with_policy, capture_compiled_output_with_policy_no_valgrind, create_interpreter, setup_program,
    setup_program_skip_typecheck,
};

#[test]
fn integer_cast_without_overflow_is_unchanged() {
    let text = BufReader::new(
        r#"
let value = 42;
let result = value as i8;

println(result as str);
"#
        .as_bytes(),
    );

    let program = setup_program(text);

    let (stdout, stderr, exit_code) = capture_compiled_output_with_policy(&program, OverflowPolicy::Error);

    assert_eq!(exit_code, 0);
    assert_eq!(stdout, "42\n");
    assert!(stderr.is_empty());
}

#[test]
fn integer_cast_overflow_is_ignored_by_default() {
    let text = BufReader::new(
        r#"
let value = 256;
let result = value as i8;

println(result as str);
"#
        .as_bytes(),
    );

    let program = setup_program(text);

    let (stdout, stderr, exit_code) = capture_compiled_output_with_policy(&program, OverflowPolicy::Ignore);

    assert_eq!(exit_code, 0);
    assert_eq!(stdout, "0\n");
    assert!(stderr.is_empty());
}

#[test]
fn integer_cast_overflow_warns_and_continues() {
    let text = BufReader::new(
        r#"
let value = 256;
let result = value as i8;

println(result as str);
"#
        .as_bytes(),
    );

    let program = setup_program(text);

    let (stdout, stderr, exit_code) = capture_compiled_output_with_policy(&program, OverflowPolicy::Warn);

    assert_eq!(exit_code, 0);

    assert_eq!(stdout, "0\n");

    assert!(stderr.contains("warning"), "expected warning in stderr, got: {}", stderr);

    assert!(
        stderr.contains("Integer overflow in cast") || stderr.contains("Value does not fit in target type"),
        "expected overflow description in stderr, got: {}",
        stderr
    );
}

#[test]
fn integer_cast_overflow_errors_and_aborts() {
    let text = BufReader::new(
        r#"
let value = 256;
let result = value as i8;

println(result as str);
"#
        .as_bytes(),
    );

    let program = setup_program(text);

    let (stdout, stderr, exit_code) = capture_compiled_output_with_policy_no_valgrind(&program, OverflowPolicy::Error);

    assert_eq!(exit_code, 1);

    assert!(stdout.is_empty(), "program should abort before println, got: {}", stdout);

    assert!(stderr.contains("error"), "expected error in stderr, got: {}", stderr);

    assert!(
        stderr.contains("Integer overflow in cast") || stderr.contains("Value does not fit in target type"),
        "expected overflow description in stderr, got: {}",
        stderr
    );
}

#[test]
fn integer_cast_overflow_is_detected_for_all_integer_widths() {
    let cases = [
        r#"
let value = 128;
let result = value as i8;
println(result as str);
"#,
        r#"
let value = 32768;
let result = value as i16;
println(result as str);
"#,
        r#"
let value = 2147483648;
let result = value as i32;
println(result as str);
"#,
    ];

    for source in cases {
        let text = BufReader::new(source.as_bytes());
        let program = setup_program(text);

        let (_, stderr, exit_code) = capture_compiled_output_with_policy(&program, OverflowPolicy::Warn);

        assert_eq!(exit_code, 0);

        assert!(stderr.contains("warning"), "expected warning, got: {}", stderr);
    }
}

#[test]
fn negative_value_cast_to_unsigned_is_detected() {
    let text = BufReader::new(
        r#"
let value = -1;
let result = value as u8;

println(result as str);
"#
        .as_bytes(),
    );

    let program = setup_program(text);

    let (stdout, stderr, exit_code) = capture_compiled_output_with_policy(&program, OverflowPolicy::Warn);

    assert_eq!(exit_code, 0);

    assert_eq!(stdout, "255\n");

    assert!(stderr.contains("warning"), "expected warning, got: {}", stderr);
}

#[test]
fn arithmetic_negation_of_i8_min_overflows() {
    let text = BufReader::new(
        r#"
let value = -128;
let result = value as i8;
let negated = -result;

println(negated as str);
"#
        .as_bytes(),
    );

    let program = setup_program(text);

    let (stdout, stderr, exit_code) = capture_compiled_output_with_policy(&program, OverflowPolicy::Warn);

    assert_eq!(exit_code, 0);

    // LLVM integer negation wraps:
    // -(-128i8) == -128i8
    assert_eq!(stdout, "-128\n");

    assert!(stderr.contains("warning"), "expected warning, got: {}", stderr);

    assert!(
        stderr.contains("Arithmetic negation overflow"),
        "expected negation overflow diagnostic, got: {}",
        stderr
    );
}

#[test]
fn arithmetic_negation_detects_minimum_value_for_all_integer_widths() {
    let cases = [
        (
            r#"
let value = -128;
let result = value as i8;
let negated = -result;
println(negated as str);
"#,
            "-128\n",
        ),
        (
            r#"
let value = -32768;
let result = value as i16;
let negated = -result;
println(negated as str);
"#,
            "-32768\n",
        ),
        (
            r#"
let value = -2147483648;
let result = value as i32;
let negated = -result;
println(negated as str);
"#,
            "-2147483648\n",
        ),
    ];

    for (source, expected) in cases {
        let text = BufReader::new(source.as_bytes());
        let program = setup_program(text);

        let (stdout, stderr, exit_code) = capture_compiled_output_with_policy(&program, OverflowPolicy::Warn);

        assert_eq!(exit_code, 0);
        assert_eq!(stdout, expected);

        assert!(stderr.contains("warning"), "expected warning, got: {}", stderr);

        assert!(
            stderr.contains("Arithmetic negation overflow"),
            "expected negation overflow diagnostic, got: {}",
            stderr
        );
    }
}

#[test]
fn arithmetic_negation_without_overflow_does_not_warn() {
    let text = BufReader::new(
        r#"
let value = 127;
let result = value as i8;
let negated = -result;

println(negated as str);
"#
        .as_bytes(),
    );

    let program = setup_program(text);

    let (stdout, stderr, exit_code) = capture_compiled_output_with_policy(&program, OverflowPolicy::Warn);

    assert_eq!(exit_code, 0);
    assert_eq!(stdout, "-127\n");
    assert!(stderr.is_empty(), "unexpected stderr: {}", stderr);
}

#[test]
fn overflow_warning_does_not_stop_program_execution() {
    let text = BufReader::new(
        r#"
let value = 256;
let result = value as i8;

println(result as str);
println(123 as str);
"#
        .as_bytes(),
    );

    let program = setup_program(text);

    let (stdout, stderr, exit_code) = capture_compiled_output_with_policy(&program, OverflowPolicy::Warn);

    assert_eq!(exit_code, 0);

    assert_eq!(stdout, "0\n123\n");

    assert!(stderr.contains("warning"), "expected warning, got: {}", stderr);
}

#[test]
fn overflow_error_stops_program_execution() {
    let text = BufReader::new(
        r#"
let value = 256;
let result = value as i8;

println(result as str);
println(123 as str);
"#
        .as_bytes(),
    );

    let program = setup_program(text);

    let (stdout, stderr, exit_code) = capture_compiled_output_with_policy_no_valgrind(&program, OverflowPolicy::Error);

    assert_eq!(exit_code, 1);

    assert!(stdout.is_empty(), "program continued after overflow: {}", stdout);

    assert!(stderr.contains("error"), "expected error, got: {}", stderr);
}

pub fn assert_interpreter_overflow(text: BufReader<&[u8]>) {
    let program = setup_program_skip_typecheck(text);

    let mut interpreter = create_interpreter(&program);

    let result = interpreter.interpret();

    assert!(result.is_err(), "expected interpreter to fail on integer overflow");

    let error = result.unwrap_err();

    assert!(
        error.get_stderr_message().to_lowercase().contains("overflow"),
        "expected overflow error, got: {}",
        error.get_stderr_message()
    );
}

#[test]
fn interpreter_stops_on_integer_cast_overflow() {
    let text = BufReader::new(
        r#"
let value = 256;
let result = value as i8;

println(result as str);
"#
        .as_bytes(),
    );

    let program = setup_program_skip_typecheck(text);

    let mut interpreter = create_interpreter(&program);

    let result = interpreter.interpret();

    assert!(result.is_err(), "expected interpreter to fail on integer overflow");

    let error = result.unwrap_err();

    assert!(
        error.get_stderr_message().to_lowercase().contains("cannot cast"),
        "expected overflow error, got: {}",
        error.get_stderr_message()
    );
}

#[test]
fn interpreter_stops_on_integer_negation_overflow() {
    let text = BufReader::new(
        r#"
let value = -128;
let result = value as i8;
let negated = -result;

println(negated as str);
"#
        .as_bytes(),
    );

    assert_interpreter_overflow(text);
}
