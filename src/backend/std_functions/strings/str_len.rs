use std::{cell::RefCell, rc::Rc, vec};

use crate::{
    backend::{
        interpreter::Value,
        llvm::{compiler::Compiler, LlvmValue},
        std_functions::std_functions::{build_usage_error, LlvmCompileFn, StdFunction},
    },
    common::{
        errors::{CompilerError, ErrorSeverity, IError, StdFunctionError},
        span::Span,
        types::Type,
        visitor::Visitor,
    },
    frontend::ast::PassedBy,
};

pub fn str_len() -> StdFunction {
    let params = vec![Type::Str];

    let execute = |params: &Vec<Rc<RefCell<Value>>>, span: Span| -> Result<Option<Value>, StdFunctionError> {
        let fn_name = "str_len";
        let expected_types = vec![Type::Str];
        let mut actual_types: Vec<Type> = vec![];

        if let Some(text) = params.first() {
            actual_types.push(text.borrow().to_type());

            let text = text.borrow();

            match &*text {
                Value::String(s) => Ok(Some(Value::I64(s.chars().count() as i64))),
                _ => Err(build_usage_error(fn_name, expected_types, actual_types, span)),
            }
        } else {
            Err(build_usage_error(fn_name, expected_types, actual_types, span))
        }
    };

    let compile: LlvmCompileFn = |compiler, arguments, span| {
        let err = |e: inkwell::builder::BuilderError| Box::new(CompilerError::at(ErrorSeverity::HIGH, e.to_string(), span)) as Box<dyn IError>;

        let arg = arguments.first().ok_or_else(|| {
            Box::new(CompilerError::at(
                ErrorSeverity::HIGH,
                String::from("'str_len' expects exactly one argument."),
                span,
            )) as Box<dyn IError>
        })?;

        compiler.visit_expression(&arg.value.value)?;
        let str_value = compiler.read_last_value()?;

        let str_ptr = match str_value {
            LlvmValue::Str(ptr) => ptr,
            other => {
                return Err(Box::new(CompilerError::at(
                    ErrorSeverity::HIGH,
                    format!("'str_len' expects a string, got '{}'.", other.to_type()),
                    span,
                )))
            }
        };

        // Just peeking at the length - read the header's data pointer
        // directly, no need to copy or retain/release anything.
        let data = compiler.str_data_ptr(str_ptr, span)?;

        let strlen_fn = compiler.libc().strlen_fn;
        let length = compiler
            .builder()
            .build_call(strlen_fn, &[data.into()], "str.len")
            .map_err(err)?
            .try_as_basic_value()
            .basic()
            .expect("strlen should return a value")
            .into_int_value();

        if Compiler::expr_needs_release(&arg.value.value.value) {
            compiler.release_value(&str_value, span)?;
        }

        compiler.set_last_value(LlvmValue::I64(length));

        Ok(())
    };

    StdFunction {
        params,
        passed_by: vec![PassedBy::Value],
        execute,
        return_type: Type::I64,
        type_check: None,
        compile,
    }
}
