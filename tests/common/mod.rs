use inkwell::context::Context;
use raptor_lib::{backend::llvm::OverflowPolicy, common::errors::ErrorSeverity, import_resolver::ImportResolver};
use raptor_lib::{
    backend::{interpreter::interpreter::Interpreter, llvm::compiler::Compiler},
    common::errors::IError,
    frontend::{
        ast::Program,
        lexer::{
            lazy_stream_reader::LazyStreamReader,
            lexer::{Lexer, LexerOptions},
        },
        parser::{IParser, Parser},
    },
    semantic::semantic_checker::SemanticChecker,
};
use std::{
    io::{BufReader, Read, Write},
    process::Command,
    sync::atomic::{AtomicU64, Ordering},
};

static TEST_COUNTER: AtomicU64 = AtomicU64::new(0);
const LLVM_VERSION: &str = "18";

fn on_warning(_err: Box<dyn IError>) {}

fn setup_program_impl(text: BufReader<&[u8]>, skip_typecheck: bool) -> Program {
    let mut text = text;
    let mut content = String::new();
    text.read_to_string(&mut content).unwrap();
    let owned_text: &'static str = Box::leak(content.into_boxed_str());
    let code = BufReader::new(owned_text.as_bytes());
    let reader = LazyStreamReader::new(code, None);
    let lexer_options = LexerOptions {
        max_comment_length: 500,
        max_identifier_length: 100,
    };
    let lexer = Lexer::new(reader, lexer_options.clone(), on_warning).unwrap();
    let mut parser = Parser::new(lexer);
    let program = parser.parse().unwrap();
    let mut import_resolver = ImportResolver::new(lexer_options, on_warning);
    let import_resolved_program = import_resolver.resolve("", program).unwrap();
    if !skip_typecheck {
        let mut checker = SemanticChecker::new(&import_resolved_program).unwrap();
        checker.check();
        let real_errors: Vec<_> = checker
            .errors
            .iter()
            .filter(|e| matches!(e.get_severity(), ErrorSeverity::HIGH))
            .collect();
        assert_eq!(real_errors.len(), 0, "semantic checker found unexpected errors: {:?}", real_errors);
    }
    import_resolved_program
}

pub fn setup_program(text: BufReader<&[u8]>) -> Program {
    setup_program_impl(text, false)
}

pub fn setup_program_skip_typecheck(text: BufReader<&[u8]>) -> Program {
    setup_program_impl(text, true)
}

pub fn create_interpreter<'a>(program: &'a Program) -> Interpreter<'a> {
    Interpreter::new(program)
}

fn write_temp_source(content: &str) -> std::path::PathBuf {
    let id = TEST_COUNTER.fetch_add(1, Ordering::SeqCst);
    let path = std::env::temp_dir().join(format!("raptor_test_src_{}_{}.rp", std::process::id(), id));
    let mut f = std::fs::File::create(&path).expect("failed to write source temp file");
    f.write_all(content.as_bytes()).unwrap();
    path
}

pub fn capture_interpreter_output_subprocess(text: BufReader<&[u8]>, skip_typecheck: bool) -> String {
    let mut text = text;
    let mut content = String::new();
    text.read_to_string(&mut content).unwrap();
    let src_path = write_temp_source(&content);
    let exe = std::env::var("CARGO_BIN_EXE_raptor").expect("CARGO_BIN_EXE_raptor not set");
    let mut args = vec![src_path.to_str().unwrap().to_string()];
    if skip_typecheck {
        args.push("--unsafe".to_string());
    }
    let output = Command::new(exe)
        .args(&args)
        .output()
        .unwrap_or_else(|err| panic!("failed to invoke raptor binary: {}", err));
    let _ = std::fs::remove_file(&src_path);
    assert!(
        output.status.success(),
        "raptor interpreter run failed:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).expect("interpreter produced non-UTF8 stdout")
}

fn run_command(program: &str, args: &[&str], step_description: &str) {
    let output = Command::new(program)
        .args(args)
        .output()
        .unwrap_or_else(|err| panic!("Failed to invoke '{}': {}. Is it installed and on PATH?", program, err));
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        panic!("{} failed:\n{}", step_description, stderr);
    }
}

fn compile_to_exe(program: &Program, overflow_policy: OverflowPolicy) -> std::path::PathBuf {
    let context = Context::create();
    let mut compiler = Compiler::new(program, &context, overflow_policy);
    compiler.compile().expect("compilation failed");

    let id = TEST_COUNTER.fetch_add(1, Ordering::SeqCst);
    let dir = std::env::temp_dir();
    let ir_path = dir.join(format!("raptor_test_{}_{}.ll", std::process::id(), id));
    let obj_path = dir.join(format!("raptor_test_{}_{}.o", std::process::id(), id));
    let exe_path = dir.join(format!("raptor_test_{}_{}", std::process::id(), id));

    compiler.write_ir_to_file(ir_path.to_str().unwrap()).expect("failed to write IR");

    let llc = format!("llc-{}", LLVM_VERSION);
    run_command(
        &llc,
        &[ir_path.to_str().unwrap(), "-filetype=obj", "-o", obj_path.to_str().unwrap()],
        "llc",
    );

    let clang = format!("clang-{}", LLVM_VERSION);
    run_command(
        &clang,
        &[obj_path.to_str().unwrap(), "-o", exe_path.to_str().unwrap(), "-no-pie"],
        "clang",
    );

    // Sprzątamy pliki pośrednie od razu
    let _ = std::fs::remove_file(&ir_path);
    let _ = std::fs::remove_file(&obj_path);

    exe_path
}

pub fn capture_compiled_output_with_policy(program: &Program, overflow_policy: OverflowPolicy) -> (String, String, i32) {
    let exe_path = compile_to_exe(program, overflow_policy);

    let output = Command::new("valgrind")
        .args([
            "--leak-check=full",
            "--errors-for-leak-kinds=definite,indirect,possible",
            "--error-exitcode=1",
            "--quiet",
            exe_path.to_str().unwrap(),
        ])
        .output()
        .expect("failed to run compiled binary under valgrind (is valgrind installed?)");

    let stdout = String::from_utf8(output.stdout).expect("compiled binary produced non-UTF8 stdout");
    let stderr = String::from_utf8(output.stderr).expect("compiled binary produced non-UTF8 stderr");
    let exit_code = output.status.code().unwrap_or(1);

    let _ = std::fs::remove_file(&exe_path);

    if exit_code != 0 {
        panic!(
            "valgrind detected memory errors / leaks or program exited non-zero (exit code {}):\n{}",
            exit_code, stderr
        );
    }

    (stdout, stderr, exit_code)
}

pub fn capture_compiled_output_with_policy_no_valgrind(program: &Program, overflow_policy: OverflowPolicy) -> (String, String, i32) {
    let exe_path = compile_to_exe(program, overflow_policy);

    let output = Command::new(&exe_path).output().expect("failed to run compiled binary");

    let stdout = String::from_utf8(output.stdout).expect("compiled binary produced non-UTF8 stdout");
    let stderr = String::from_utf8(output.stderr).expect("compiled binary produced non-UTF8 stderr");
    let exit_code = output.status.code().unwrap_or(1);

    let _ = std::fs::remove_file(&exe_path);

    (stdout, stderr, exit_code)
}

pub fn capture_compiled_output(program: &Program) -> String {
    let (stdout, _, exit_code) = capture_compiled_output_with_policy(program, OverflowPolicy::Ignore);
    assert_eq!(exit_code, 0, "compiled binary exited with a non-zero status");
    stdout
}

pub fn assert_same_output(text: BufReader<&[u8]>, expected: &str) {
    let mut text = text;
    let mut content = String::new();
    text.read_to_string(&mut content).unwrap();
    let program = setup_program(BufReader::new(content.as_bytes()));
    let interpreter_output = capture_interpreter_output_subprocess(BufReader::new(content.as_bytes()), false);
    let compiler_output = capture_compiled_output(&program);
    assert_eq!(interpreter_output, expected, "interpreter output mismatch");
    assert_eq!(compiler_output, expected, "compiler output mismatch");
    assert_eq!(interpreter_output, compiler_output, "interpreter and compiler outputs disagree");
}
