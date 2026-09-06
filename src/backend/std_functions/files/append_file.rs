use std::{cell::RefCell, fs::OpenOptions, io::Write, rc::Rc, vec};

use crate::{
    backend::{
        interpreter::Value,
        std_functions::{
            files::common::compile_write_or_append,
            std_functions::{build_usage_error, LlvmCompileFn, StdFunction},
        },
    },
    common::{
        errors::{ErrorSeverity, StdFunctionError},
        span::Span,
        types::Type,
    },
    frontend::ast::PassedBy,
};

pub fn append_file() -> StdFunction {
    let params = vec![Type::Str, Type::Str];
    let execute = |params: &Vec<Rc<RefCell<Value>>>, span: Span| -> Result<Option<Value>, StdFunctionError> {
        let fn_name = "append_file";
        let expected_types = vec![Type::Str, Type::Str];
        let mut actual_types: Vec<Type> = vec![];
        if let Some(filepath) = params.first() {
            actual_types.push(filepath.borrow().to_type());
            if let Some(content) = params.get(1) {
                actual_types.push(content.borrow().to_type());
                let filepath = filepath.borrow();
                let content = content.borrow();
                match &*filepath {
                    Value::String(path) => match &*content {
                        Value::String(con) => match OpenOptions::new().append(true).create(true).open(path) {
                            Ok(mut file) => match write!(file, "{}", con) {
                                Ok(_) => Ok(None),
                                Err(_) => Err(StdFunctionError::new(
                                    ErrorSeverity::HIGH,
                                    String::from("Failed to append to file."),
                                    span,
                                )),
                            },
                            Err(_) => Err(StdFunctionError::new(
                                ErrorSeverity::HIGH,
                                String::from("Failed to append to file."),
                                span,
                            )),
                        },
                        _ => Err(build_usage_error(fn_name, expected_types, actual_types, span)),
                    },
                    _ => Err(build_usage_error(fn_name, expected_types, actual_types, span)),
                }
            } else {
                Err(build_usage_error(fn_name, expected_types, actual_types, span))
            }
        } else {
            Err(build_usage_error(fn_name, expected_types, actual_types, span))
        }
    };

    let compile: LlvmCompileFn = |compiler, arguments, span| compile_write_or_append(compiler, arguments, "ab", span);

    StdFunction {
        params,
        execute,
        passed_by: vec![PassedBy::Value, PassedBy::Value],
        return_type: Type::Void,
        type_check: None,
        compile,
    }
}
