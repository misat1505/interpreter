use std::{cell::RefCell, collections::HashMap, rc::Rc, vec};

use inkwell::AddressSpace;

use crate::{
    backend::{
        interpreter::Value,
        llvm::LlvmValue,
        std_functions::{
            network::state::{next_handle, LISTENERS, STREAMS},
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

pub fn tcp_accept() -> StdFunction {
    let params = vec![Type::I64];

    let execute = |params: &Vec<Rc<RefCell<Value>>>, span: Span| -> Result<Option<Value>, StdFunctionError> {
        let fn_name = "tcp_accept";
        let expected_types = vec![Type::I64];
        let mut actual_types: Vec<Type> = vec![];

        if let Some(handle) = params.first() {
            actual_types.push(handle.borrow().to_type());

            let handle = handle.borrow();

            match &*handle {
                Value::I64(listener_handle) => {
                    let listeners = LISTENERS.lock().unwrap();
                    let listener = listeners
                        .as_ref()
                        .and_then(|m| m.get(listener_handle))
                        .ok_or_else(|| StdFunctionError::new(ErrorSeverity::HIGH, format!("Invalid listener handle {}", listener_handle), span))?;

                    let (stream, _addr) = listener
                        .accept()
                        .map_err(|e| StdFunctionError::new(ErrorSeverity::HIGH, format!("Accept failed: {}", e), span))?;

                    drop(listeners);

                    let new_handle = next_handle();
                    STREAMS.lock().unwrap().get_or_insert_with(HashMap::new).insert(new_handle, stream);

                    Ok(Some(Value::I64(new_handle)))
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
                String::from("'tcp_accept' expects exactly one argument."),
                span,
            )) as Box<dyn IError>
        })?;

        compiler.visit_expression(&arg.value.value)?;
        let listener_fd = compiler.read_last_value()?.into_i64_value(span)?;

        let context = compiler.context();
        let i32_type = context.i32_type();
        let ptr_type = context.ptr_type(AddressSpace::default());

        let listener_fd_i32 = compiler.builder().build_int_truncate(listener_fd, i32_type, "fd.i32").map_err(err)?;

        let accept_fn = compiler.libc().accept_fn;
        let client_fd = compiler
            .builder()
            .build_call(
                accept_fn,
                &[listener_fd_i32.into(), ptr_type.const_null().into(), ptr_type.const_null().into()],
                "accept.call",
            )
            .map_err(err)?
            .try_as_basic_value()
            .basic()
            .expect("accept should return a value")
            .into_int_value();

        let client_fd_i64 = compiler
            .builder()
            .build_int_z_extend(client_fd, context.i64_type(), "fd.i64")
            .map_err(err)?;

        compiler.set_last_value(LlvmValue::I64(client_fd_i64));
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
