use std::io::BufReader;

use raptor_lib::backend::llvm::OverflowPolicy;

use crate::common::{
    capture_compiled_output_with_policy, capture_compiled_output_with_policy_no_valgrind, create_interpreter, setup_program,
    setup_program_skip_typecheck,
};

// ---------------------------------------------------------------------
// Compiler (LLVM) — read access, Vector
// ---------------------------------------------------------------------

#[test]
fn vector_index_within_bounds_is_ok() {
    let text = BufReader::new(
        r#"
let numbers: i64[] = [420];

println(numbers[0] as str);
"#
        .as_bytes(),
    );

    let program = setup_program(text);

    let (stdout, stderr, exit_code) = capture_compiled_output_with_policy(&program, OverflowPolicy::Ignore);

    assert_eq!(exit_code, 0);
    assert_eq!(stdout, "420\n");
    assert!(stderr.is_empty());
}

#[test]
fn vector_index_equal_to_length_is_out_of_bounds() {
    let text = BufReader::new(
        r#"
let numbers: i64[] = [420];

println(numbers[1] as str);
"#
        .as_bytes(),
    );

    let program = setup_program(text);

    let (stdout, stderr, exit_code) = capture_compiled_output_with_policy_no_valgrind(&program, OverflowPolicy::Ignore);

    assert_ne!(exit_code, 0);
    assert!(stdout.is_empty(), "program should abort before println, got: {}", stdout);
    assert!(
        stderr.to_lowercase().contains("out of bounds"),
        "expected out-of-bounds message, got: {}",
        stderr
    );
}

#[test]
fn vector_index_far_beyond_length_is_out_of_bounds() {
    let text = BufReader::new(
        r#"
let numbers: i64[] = [420];

println(numbers[100] as str);
"#
        .as_bytes(),
    );

    let program = setup_program(text);

    let (stdout, stderr, exit_code) = capture_compiled_output_with_policy_no_valgrind(&program, OverflowPolicy::Ignore);

    assert_ne!(exit_code, 0);
    assert!(stdout.is_empty());
    assert!(stderr.to_lowercase().contains("out of bounds"));
}

#[test]
fn vector_negative_index_is_out_of_bounds() {
    let text = BufReader::new(
        r#"
let numbers: i64[] = [420];
let idx = 0 - 1;

println(numbers[idx] as str);
"#
        .as_bytes(),
    );

    let program = setup_program(text);

    let (stdout, stderr, exit_code) = capture_compiled_output_with_policy_no_valgrind(&program, OverflowPolicy::Ignore);

    assert_ne!(exit_code, 0);
    assert!(stdout.is_empty());
    assert!(stderr.to_lowercase().contains("out of bounds"));
}

#[test]
fn vector_index_on_empty_vector_is_always_out_of_bounds() {
    let text = BufReader::new(
        r#"
let numbers: i64[] = [];

println(numbers[0] as str);
"#
        .as_bytes(),
    );

    let program = setup_program(text);

    let (stdout, stderr, exit_code) = capture_compiled_output_with_policy_no_valgrind(&program, OverflowPolicy::Ignore);

    assert_ne!(exit_code, 0);
    assert!(stdout.is_empty());
    assert!(stderr.to_lowercase().contains("out of bounds"));
}

// ---------------------------------------------------------------------
// Compiler (LLVM) — write access (indexed assignment), Vector
// ---------------------------------------------------------------------

#[test]
fn vector_indexed_assignment_within_bounds_is_ok() {
    let text = BufReader::new(
        r#"
let numbers: i64[] = [420];

numbers[0] = 123;

println(numbers[0] as str);
"#
        .as_bytes(),
    );

    let program = setup_program(text);

    let (stdout, stderr, exit_code) = capture_compiled_output_with_policy(&program, OverflowPolicy::Ignore);

    assert_eq!(exit_code, 0);
    assert_eq!(stdout, "123\n");
    assert!(stderr.is_empty());
}

#[test]
fn vector_indexed_assignment_out_of_bounds_aborts() {
    // Matches the example: writing past the current length without growing
    // the vector first must be caught, not silently corrupt memory.
    let text = BufReader::new(
        r#"
let numbers: i64[] = [420];

println(numbers[0] as str);

numbers[1] = 123;

println(numbers[1] as str);
"#
        .as_bytes(),
    );

    let program = setup_program(text);

    let (stdout, stderr, exit_code) = capture_compiled_output_with_policy_no_valgrind(&program, OverflowPolicy::Ignore);

    assert_ne!(exit_code, 0);
    assert_eq!(stdout, "420\n", "the first println should still run before the failing assignment");
    assert!(stderr.to_lowercase().contains("out of bounds"));
}

#[test]
fn vector_indexed_assignment_after_push_is_within_bounds() {
    // Same shape as the failing example above, but after vector_push grows
    // the vector, index 1 is now valid.
    let text = BufReader::new(
        r#"
let numbers: i64[] = [420];

println(numbers[0] as str);

vector_push(&numbers, 2137);

numbers[1] = 123;

println(numbers[1] as str);
"#
        .as_bytes(),
    );

    let program = setup_program(text);

    let (stdout, stderr, exit_code) = capture_compiled_output_with_policy(&program, OverflowPolicy::Ignore);

    assert_eq!(exit_code, 0);
    assert_eq!(stdout, "420\n123\n");
    assert!(stderr.is_empty());
}

// ---------------------------------------------------------------------
// Compiler (LLVM) — read access, Str
// ---------------------------------------------------------------------

#[test]
fn string_index_within_bounds_is_ok() {
    let text = BufReader::new(
        r#"
let text = "hello";

println(text[0] as str);
"#
        .as_bytes(),
    );

    let program = setup_program(text);

    let (stdout, stderr, exit_code) = capture_compiled_output_with_policy(&program, OverflowPolicy::Ignore);

    assert_eq!(exit_code, 0);
    assert_eq!(stdout, "h\n");
    assert!(stderr.is_empty());
}

#[test]
fn string_index_equal_to_length_is_out_of_bounds() {
    let text = BufReader::new(
        r#"
let text = "hi";

println(text[2] as str);
"#
        .as_bytes(),
    );

    let program = setup_program(text);

    let (stdout, stderr, exit_code) = capture_compiled_output_with_policy_no_valgrind(&program, OverflowPolicy::Ignore);

    assert_ne!(exit_code, 0);
    assert!(stdout.is_empty());
    assert!(stderr.to_lowercase().contains("out of bounds"));
}

#[test]
fn string_negative_index_is_out_of_bounds() {
    let text = BufReader::new(
        r#"
let text = "hi";
let idx = 0 - 1;

println(text[idx] as str);
"#
        .as_bytes(),
    );

    let program = setup_program(text);

    let (stdout, stderr, exit_code) = capture_compiled_output_with_policy_no_valgrind(&program, OverflowPolicy::Ignore);

    assert_ne!(exit_code, 0);
    assert!(stdout.is_empty());
    assert!(stderr.to_lowercase().contains("out of bounds"));
}

#[test]
fn string_index_on_empty_string_is_always_out_of_bounds() {
    let text = BufReader::new(
        r#"
let text = "";

println(text[0] as str);
"#
        .as_bytes(),
    );

    let program = setup_program(text);

    let (stdout, stderr, exit_code) = capture_compiled_output_with_policy_no_valgrind(&program, OverflowPolicy::Ignore);

    assert_ne!(exit_code, 0);
    assert!(stdout.is_empty());
    assert!(stderr.to_lowercase().contains("out of bounds"));
}

// ---------------------------------------------------------------------
// Compiler (LLVM) — nested accessors (struct field + index)
// ---------------------------------------------------------------------

#[test]
fn nested_field_then_index_out_of_bounds_aborts() {
    let text = BufReader::new(
        r#"
struct Container {
    i64[] items
};

let container = Container { items: [1, 2] };

println(container.items[5] as str);
"#
        .as_bytes(),
    );

    let program = setup_program(text);

    let (stdout, stderr, exit_code) = capture_compiled_output_with_policy_no_valgrind(&program, OverflowPolicy::Ignore);

    assert_ne!(exit_code, 0);
    assert!(stdout.is_empty());
    assert!(stderr.to_lowercase().contains("out of bounds"));
}

#[test]
fn nested_field_then_indexed_assignment_out_of_bounds_aborts() {
    let text = BufReader::new(
        r#"
struct Container {
    i64[] items
};

let container = Container { items: [1, 2] };

container.items[5] = 99;

println(container.items[0] as str);
"#
        .as_bytes(),
    );

    let program = setup_program(text);

    let (stdout, stderr, exit_code) = capture_compiled_output_with_policy_no_valgrind(&program, OverflowPolicy::Ignore);

    assert_ne!(exit_code, 0);
    assert!(stdout.is_empty());
    assert!(stderr.to_lowercase().contains("out of bounds"));
}

#[test]
fn nested_field_then_indexed_assignment_within_bounds_is_ok() {
    let text = BufReader::new(
        r#"
struct Container {
    i64[] items
};

let container = Container { items: [1, 2] };

container.items[1] = 99;

println(container.items[1] as str);
"#
        .as_bytes(),
    );

    let program = setup_program(text);

    let (stdout, stderr, exit_code) = capture_compiled_output_with_policy_no_valgrind(&program, OverflowPolicy::Ignore);

    assert_eq!(exit_code, 0);
    assert_eq!(stdout, "99\n");
    assert!(stderr.is_empty());
}

// ---------------------------------------------------------------------
// Compiler (LLVM) — execution stops exactly at the failing access
// ---------------------------------------------------------------------

#[test]
fn out_of_bounds_stops_program_execution() {
    let text = BufReader::new(
        r#"
let numbers: i64[] = [420];

println(numbers[0] as str);
println(numbers[1] as str);
println(999 as str);
"#
        .as_bytes(),
    );

    let program = setup_program(text);

    let (stdout, stderr, exit_code) = capture_compiled_output_with_policy_no_valgrind(&program, OverflowPolicy::Ignore);

    assert_ne!(exit_code, 0);
    assert_eq!(stdout, "420\n", "execution must stop at the first out-of-bounds access");
    assert!(stderr.to_lowercase().contains("out of bounds"));
}

// ---------------------------------------------------------------------
// Interpreter — mirrors the compiler cases above
// ---------------------------------------------------------------------

pub fn assert_interpreter_out_of_bounds(text: BufReader<&[u8]>) {
    let program = setup_program_skip_typecheck(text);

    let mut interpreter = create_interpreter(&program);

    let result = interpreter.interpret();

    assert!(result.is_err(), "expected interpreter to fail on out-of-bounds access");

    let error = result.unwrap_err();

    assert!(
        error.get_stderr_message().to_lowercase().contains("out of bounds"),
        "expected out-of-bounds error, got: {}",
        error.get_stderr_message()
    );
}

#[test]
fn interpreter_vector_read_out_of_bounds() {
    let text = BufReader::new(
        r#"
let numbers: i64[] = [420];

println(numbers[1] as str);
"#
        .as_bytes(),
    );

    assert_interpreter_out_of_bounds(text);
}

#[test]
fn interpreter_vector_negative_index_out_of_bounds() {
    let text = BufReader::new(
        r#"
let numbers: i64[] = [420];
let idx = 0 - 1;

println(numbers[idx] as str);
"#
        .as_bytes(),
    );

    let program = setup_program_skip_typecheck(text);

    let mut interpreter = create_interpreter(&program);

    let result = interpreter.interpret();

    assert!(result.is_err(), "expected interpreter to fail on out-of-bounds access");

    let error = result.unwrap_err();

    assert!(
        error
            .get_stderr_message()
            .to_lowercase()
            .contains("array index must be a non-negative i64"),
        "expected out-of-bounds error, got: {}",
        error.get_stderr_message()
    );
}

#[test]
fn interpreter_vector_write_out_of_bounds() {
    let text = BufReader::new(
        r#"
let numbers: i64[] = [420];

println(numbers[0] as str);

numbers[1] = 123;

println(numbers[1] as str);
"#
        .as_bytes(),
    );

    assert_interpreter_out_of_bounds(text);
}

#[test]
fn interpreter_vector_write_after_push_is_within_bounds() {
    let text = BufReader::new(
        r#"
let numbers: i64[] = [420];

println(numbers[0] as str);

vector_push(&numbers, 2137);

numbers[1] = 123;

println(numbers[1] as str);
"#
        .as_bytes(),
    );

    let program = setup_program_skip_typecheck(text);
    let mut interpreter = create_interpreter(&program);

    let result = interpreter.interpret();

    assert!(
        result.is_ok(),
        "expected interpreter to succeed, got: {:?}",
        result.err().map(|e| e.get_stderr_message())
    );
}

#[test]
fn interpreter_string_read_out_of_bounds() {
    let text = BufReader::new(
        r#"
let text = "hi";

println(text[2] as str);
"#
        .as_bytes(),
    );

    assert_interpreter_out_of_bounds(text);
}

#[test]
fn interpreter_string_read_on_empty_string_out_of_bounds() {
    let text = BufReader::new(
        r#"
let text = "";

println(text[0] as str);
"#
        .as_bytes(),
    );

    assert_interpreter_out_of_bounds(text);
}

#[test]
fn interpreter_nested_field_index_out_of_bounds() {
    let text = BufReader::new(
        r#"
struct Container {
    i64[] items
};

let container = Container { items: [1, 2] };

println(container.items[5] as str);
"#
        .as_bytes(),
    );

    assert_interpreter_out_of_bounds(text);
}
