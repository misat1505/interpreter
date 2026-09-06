use std::{cell::RefCell, collections::HashMap, rc::Rc};

use crate::{
    backend::{
        interpreter::Value,
        llvm::compiler::Compiler,
        std_functions::{
            files::{append_file::append_file, delete_file::delete_file, exists_file::exists_file, read_file::read_file, write_file::write_file},
            io::{input::input, print::print, println::println},
            network::{
                tcp_accept::tcp_accept, tcp_close::tcp_close, tcp_connect::tcp_connect, tcp_listen::tcp_listen, tcp_read::tcp_read,
                tcp_write::tcp_write,
            },
            strings::str_len::str_len,
            time::sleep_ms::sleep_ms,
            vectors::{vector_push::vector_push, vector_size::vector_size, vector_stringify::vector_stringify},
        },
    },
    common::{
        errors::{ErrorSeverity, IError, StdFunctionError},
        span::Span,
        types::Type,
    },
    frontend::ast::{Argument, Node, PassedBy},
};

type InterpreterExecuteFn = fn(&Vec<Rc<RefCell<Value>>>, span: Span) -> Result<Option<Value>, StdFunctionError>;
type TypeChecker = fn(&[Type]) -> Result<Type, String>;
pub type LlvmCompileFn = for<'a, 'ctx> fn(&mut Compiler<'a, 'ctx>, &'a Vec<Box<Node<Argument>>>, Span) -> Result<(), Box<dyn IError>>;

#[derive(Debug, Clone)]
pub struct StdFunction {
    pub params: Vec<Type>,
    pub passed_by: Vec<PassedBy>,
    pub execute: InterpreterExecuteFn,
    pub return_type: Type,
    pub type_check: Option<TypeChecker>,
    pub compile: LlvmCompileFn,
}

impl PartialEq for StdFunction {
    fn eq(&self, other: &Self) -> bool {
        self.params == other.params
    }
}

pub fn format_types(types: &[Type]) -> String {
    types.iter().map(|t| format!("{}", t)).collect::<Vec<String>>().join(", ")
}

pub fn build_usage_error(fn_name: &str, expected_types: Vec<Type>, actual_types: Vec<Type>, span: Span) -> StdFunctionError {
    let message = format!(
        "\nInvalid usage of built-in function '{}'.\nExpected signature: {}({})\nProvided types: {}({})",
        fn_name,
        fn_name,
        format_types(&expected_types),
        fn_name,
        format_types(&actual_types)
    );
    StdFunctionError::new(ErrorSeverity::HIGH, message, span)
}

pub fn get_std_functions() -> HashMap<String, StdFunction> {
    HashMap::from([
        ("print".into(), print()),
        ("println".into(), println()),
        ("input".into(), input()),
        ("read_file".into(), read_file()),
        ("write_file".into(), write_file()),
        ("append_file".into(), append_file()),
        ("delete_file".into(), delete_file()),
        ("exists_file".into(), exists_file()),
        ("vector_stringify".into(), vector_stringify()),
        ("vector_push".into(), vector_push()),
        ("vector_size".into(), vector_size()),
        ("tcp_listen".into(), tcp_listen()),
        ("tcp_accept".into(), tcp_accept()),
        ("tcp_read".into(), tcp_read()),
        ("tcp_write".into(), tcp_write()),
        ("tcp_close".into(), tcp_close()),
        ("tcp_connect".into(), tcp_connect()),
        ("sleep_ms".into(), sleep_ms()),
        ("str_len".into(), str_len()),
    ])
}
