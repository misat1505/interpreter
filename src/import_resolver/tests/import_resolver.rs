use std::{
    collections::HashMap,
    fs,
    io::{BufReader, Write},
    path::{Path, PathBuf},
    rc::Rc,
    sync::atomic::{AtomicU64, Ordering},
    time::{SystemTime, UNIX_EPOCH},
};

use crate::{
    backend::std_functions::std_functions::get_std_functions,
    common::{span::Span, types::Type},
    frontend::{
        ast::{Block, Expression, FunctionDeclaration, Literal, Node, Program, Statement},
        lexer::{
            lazy_stream_reader::LazyStreamReader,
            lexer::{Lexer, LexerOptions},
        },
        parser::{IParser, Parser},
    },
    import_resolver::{normalize_path, ImportResolver},
};

static DIR_COUNTER: AtomicU64 = AtomicU64::new(0);

fn unique_temp_dir() -> PathBuf {
    let id = DIR_COUNTER.fetch_add(1, Ordering::SeqCst);
    let nanos = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
    let dir = std::env::temp_dir().join(format!("raptor_import_resolver_test_{}_{}", nanos, id));
    fs::create_dir_all(&dir).expect("failed to create temp dir for test fixtures");
    dir
}

fn write_file(dir: &Path, relative: &str, contents: &str) -> PathBuf {
    let path = dir.join(relative);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).expect("failed to create parent dir for test fixture");
    }
    let mut file = fs::File::create(&path).expect("failed to create test fixture file");
    file.write_all(contents.as_bytes()).expect("failed to write test fixture file");
    path
}

fn parse_source_file(path: &Path) -> Program {
    let file = fs::File::open(path).expect("failed to open test fixture file");
    let reader = BufReader::new(file);
    let filename: &'static str = Box::leak(path.to_string_lossy().into_owned().into_boxed_str());
    let stream = LazyStreamReader::new(reader, Some(filename));

    let lexer = Lexer::new(stream, test_lexer_options(), |_| {}).expect("lexer should succeed on well-formed test fixture");

    let mut parser = Parser::new(lexer);
    parser.parse().expect("parser should succeed on well-formed test fixture")
}

fn test_lexer_options() -> LexerOptions {
    LexerOptions {
        max_comment_length: 500,
        max_identifier_length: 100,
    }
}

fn dummy_node<T>(value: T) -> Node<T> {
    Node {
        value,
        span: Span::default(),
    }
}

fn program_declaring_function(name: &str) -> Program {
    let function = FunctionDeclaration {
        identifier: dummy_node(name.to_string()),
        parameters: vec![],
        return_type: dummy_node(Type::Struct {
            identifier: "Void".to_string(),
            fields: HashMap::new(),
        }),
        block: dummy_node(Block(vec![])),
    };

    Program {
        statements: vec![],
        functions: HashMap::from([(name.to_string(), Rc::new(dummy_node(function)))]),
        std_functions: HashMap::new(),
        extern_functions: HashMap::new(),
        declared_types: HashMap::new(),
        types: HashMap::new(),
    }
}

fn fresh_merged_program() -> Program {
    Program {
        statements: vec![],
        functions: HashMap::new(),
        std_functions: get_std_functions(),
        extern_functions: HashMap::new(),
        declared_types: HashMap::new(),
        types: HashMap::new(),
    }
}

fn extract_call_string_arg(stmt: &Statement) -> String {
    match stmt {
        Statement::FunctionCall { arguments, .. } => match &arguments[0].value.value.value {
            Expression::Literal(Literal::String(s)) => s.clone(),
            other => panic!("expected a string literal argument, got {:?}", other),
        },
        other => panic!("expected a FunctionCall statement, got {:?}", other),
    }
}

#[test]
fn normalize_path_collapses_parent_dir_components() {
    assert_eq!(normalize_path(Path::new("a/b/../c")), PathBuf::from("a/c"));
}

#[test]
fn normalize_path_removes_current_dir_components() {
    assert_eq!(normalize_path(Path::new("./a/./b")), PathBuf::from("a/b"));
}

#[test]
fn normalize_path_ignores_parent_dir_beyond_root() {
    assert_eq!(normalize_path(Path::new("a/../../b")), PathBuf::from("b"));
}

#[test]
fn resolve_with_no_imports_still_includes_std_functions() {
    let dir = unique_temp_dir();
    let entry_path = write_file(&dir, "entry.rp", r#"println("hello");"#);

    let entry_program = parse_source_file(&entry_path);
    let mut resolver = ImportResolver::new(test_lexer_options(), |_| {});

    let merged = resolver
        .resolve(&entry_path.to_string_lossy(), entry_program)
        .expect("resolve should succeed when there are no imports");

    assert!(merged.std_functions.contains_key("println"));
    assert_eq!(merged.statements.len(), 1);

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn resolve_merges_single_import_in_place_preserving_order() {
    let dir = unique_temp_dir();
    write_file(&dir, "child.rp", r#"println("child");"#);
    let entry_path = write_file(&dir, "entry.rp", r#"println("before");import "child.rp";println("after");"#);

    let entry_program = parse_source_file(&entry_path);
    let mut resolver = ImportResolver::new(test_lexer_options(), |_| {});

    let merged = resolver
        .resolve(&entry_path.to_string_lossy(), entry_program)
        .expect("resolve should succeed for a single valid import");

    let values: Vec<String> = merged.statements.iter().map(|s| extract_call_string_arg(&s.value)).collect();

    assert_eq!(values, vec!["before".to_string(), "child".to_string(), "after".to_string()]);
    assert!(
        merged.statements.iter().all(|s| !matches!(s.value, Statement::Import { .. })),
        "import statements must be expanded away, not left in the merged AST"
    );

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn resolve_dedups_diamond_imports() {
    let dir = unique_temp_dir();
    write_file(&dir, "common.rp", r#"println("common");"#);
    write_file(&dir, "a.rp", r#"import "common.rp";println("a");"#);
    write_file(&dir, "b.rp", r#"import "common.rp";println("b");"#);
    let entry_path = write_file(&dir, "entry.rp", r#"import "a.rp";import "b.rp";"#);

    let entry_program = parse_source_file(&entry_path);
    let mut resolver = ImportResolver::new(test_lexer_options(), |_| {});

    let merged = resolver
        .resolve(&entry_path.to_string_lossy(), entry_program)
        .expect("diamond-shaped imports should resolve without error or duplication");

    let values: Vec<String> = merged.statements.iter().map(|s| extract_call_string_arg(&s.value)).collect();

    assert_eq!(values, vec!["common".to_string(), "a".to_string(), "b".to_string()]);

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn resolve_detects_cyclic_imports() {
    let dir = unique_temp_dir();
    write_file(&dir, "b.rp", r#"import "a.rp";"#);
    let a_path = write_file(&dir, "a.rp", r#"import "b.rp";"#);

    let entry_program = parse_source_file(&a_path);
    let mut resolver = ImportResolver::new(test_lexer_options(), |_| {});

    let result = resolver.resolve(&a_path.to_string_lossy(), entry_program);

    let err = result.expect_err("a cyclic import chain must be rejected");
    assert!(err.message().contains("Cyclic import detected"));

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn resolve_reports_missing_imported_file() {
    let dir = unique_temp_dir();
    let entry_path = write_file(&dir, "entry.rp", r#"import "missing.rp";"#);

    let entry_program = parse_source_file(&entry_path);
    let mut resolver = ImportResolver::new(test_lexer_options(), |_| {});

    let result = resolver.resolve(&entry_path.to_string_lossy(), entry_program);

    let err = result.expect_err("importing a nonexistent file must fail");
    assert!(err.message().contains("not found"));

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn resolve_resolves_relative_paths_against_each_files_own_directory() {
    let dir = unique_temp_dir();
    write_file(&dir, "sibling.rp", r#"println("sibling");"#);
    write_file(&dir, "sub/child.rp", r#"import "../sibling.rp";println("child");"#);
    let entry_path = write_file(&dir, "entry.rp", r#"import "sub/child.rp";"#);

    let entry_program = parse_source_file(&entry_path);
    let mut resolver = ImportResolver::new(test_lexer_options(), |_| {});

    let merged = resolver
        .resolve(&entry_path.to_string_lossy(), entry_program)
        .expect("relative import climbing back out of a subdirectory should resolve");

    let values: Vec<String> = merged.statements.iter().map(|s| extract_call_string_arg(&s.value)).collect();

    assert_eq!(values, vec!["sibling".to_string(), "child".to_string()]);

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn merge_program_detects_function_name_collision_across_modules() {
    let mut resolver = ImportResolver::new(test_lexer_options(), |_| {});
    let mut merged = fresh_merged_program();

    resolver
        .merge_program("module_a.rp", program_declaring_function("shared_helper"), &mut merged)
        .expect("first module should merge without conflict");

    let result = resolver.merge_program("module_b.rp", program_declaring_function("shared_helper"), &mut merged);

    let err = result.expect_err("re-declaring the same function name in a second module must fail");
    assert!(err.message().contains("Redeclaration of 'shared_helper'"));
}

#[test]
fn merge_program_detects_collision_with_std_function_name() {
    let mut resolver = ImportResolver::new(test_lexer_options(), |_| {});
    let mut merged = fresh_merged_program();

    let result = resolver.merge_program("module.rp", program_declaring_function("println"), &mut merged);

    let err = result.expect_err("shadowing a built-in function name must fail");
    assert!(err.message().contains("Redeclaration of 'println'"));
}
