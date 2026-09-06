use std::{cell::RefCell, rc::Rc, vec};

use crate::{
    backend::{
        interpreter::Value,
        std_functions::{
            network::state::{LISTENERS, STREAMS},
            std_functions::{build_usage_error, LlvmCompileFn, StdFunction},
        },
    },
    common::{
        errors::{CompilerError, ErrorSeverity, IError, StdFunctionError},
        span::Span,
        types::Type,
        visitor::Visitor,
    },
    frontend::ast::PassedBy,
};

pub fn tcp_close() -> StdFunction {
    let params = vec![Type::I64];

    let execute = |params: &Vec<Rc<RefCell<Value>>>, span: Span| -> Result<Option<Value>, StdFunctionError> {
        let fn_name = "tcp_close";
        let expected_types = vec![Type::I64];
        let mut actual_types: Vec<Type> = vec![];

        if let Some(handle) = params.first() {
            actual_types.push(handle.borrow().to_type());

            let handle = handle.borrow();

            match &*handle {
                Value::I64(h) => {
                    STREAMS.lock().unwrap().as_mut().map(|m| m.remove(h));
                    LISTENERS.lock().unwrap().as_mut().map(|m| m.remove(h));
                    Ok(None)
                }
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
                String::from("'tcp_close' expects exactly one argument."),
                span,
            )) as Box<dyn IError>
        })?;

        compiler.visit_expression(&arg.value.value)?;
        let fd = compiler.read_last_value()?.into_i64_value(span)?;

        let context = compiler.context();
        let i32_type = context.i32_type();

        let fd_i32 = compiler.builder().build_int_truncate(fd, i32_type, "fd.i32").map_err(err)?;

        let close_fn = compiler.libc().close_fn;
        compiler.builder().build_call(close_fn, &[fd_i32.into()], "close.call").map_err(err)?;

        Ok(())
    };

    StdFunction {
        params,
        passed_by: vec![PassedBy::Value],
        execute,
        return_type: Type::Void,
        type_check: None,
        compile,
    }
}
