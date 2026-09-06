use std::{cell::RefCell, io::Write, rc::Rc, vec};

use crate::{
    backend::{
        interpreter::Value,
        llvm::{compiler::Compiler, LlvmValue},
        std_functions::{
            network::state::STREAMS,
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

pub fn tcp_write() -> StdFunction {
    let params = vec![Type::I64, Type::Str];

    let execute = |params: &Vec<Rc<RefCell<Value>>>, span: Span| -> Result<Option<Value>, StdFunctionError> {
        let fn_name = "tcp_write";
        let expected_types = vec![Type::I64, Type::Str];
        let mut actual_types: Vec<Type> = vec![];

        let handle_param = params.first();
        let data_param = params.get(1);

        if let (Some(handle), Some(data)) = (handle_param, data_param) {
            actual_types.push(handle.borrow().to_type());
            actual_types.push(data.borrow().to_type());

            let handle = handle.borrow();
            let data = data.borrow();

            match (&*handle, &*data) {
                (Value::I64(stream_handle), Value::String(payload)) => {
                    let mut streams = STREAMS.lock().unwrap();

                    let stream = streams
                        .as_mut()
                        .and_then(|m| m.get_mut(stream_handle))
                        .ok_or_else(|| StdFunctionError::new(ErrorSeverity::HIGH, format!("Invalid stream handle {}", stream_handle), span))?;

                    stream
                        .write_all(payload.as_bytes())
                        .map_err(|e| StdFunctionError::new(ErrorSeverity::HIGH, format!("Write failed: {}", e), span))?;

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

        let fd_arg = arguments.first().ok_or_else(|| {
            Box::new(CompilerError::at(
                ErrorSeverity::HIGH,
                String::from("'tcp_write' expects exactly two arguments."),
                span,
            )) as Box<dyn IError>
        })?;

        let data_arg = arguments.get(1).ok_or_else(|| {
            Box::new(CompilerError::at(
                ErrorSeverity::HIGH,
                String::from("'tcp_write' expects exactly two arguments."),
                span,
            )) as Box<dyn IError>
        })?;

        compiler.visit_expression(&fd_arg.value.value)?;
        let fd = compiler.read_last_value()?.into_i64_value(span)?;

        compiler.visit_expression(&data_arg.value.value)?;
        let data_value = compiler.read_last_value()?;

        let data_ptr = match data_value {
            LlvmValue::Str(ptr) => ptr,
            _ => {
                return Err(Box::new(CompilerError::at(
                    ErrorSeverity::HIGH,
                    String::from("'tcp_write' expects a string as its second argument."),
                    span,
                )))
            }
        };

        let data_field = unsafe {
            compiler
                .builder()
                .build_gep(
                    compiler.context().i8_type(),
                    data_ptr,
                    &[compiler.context().i64_type().const_int(8, false)],
                    "str.data.field",
                )
                .map_err(err)?
        };

        let text_ptr = compiler
            .builder()
            .build_load(compiler.context().ptr_type(inkwell::AddressSpace::default()), data_field, "str.data")
            .map_err(err)?
            .into_pointer_value();

        let context = compiler.context();
        let i32_type = context.i32_type();

        let fd_i32 = compiler.builder().build_int_truncate(fd, i32_type, "fd.i32").map_err(err)?;

        let strlen_fn = compiler.libc().strlen_fn;

        let len = compiler
            .builder()
            .build_call(strlen_fn, &[text_ptr.into()], "data.len")
            .map_err(err)?
            .try_as_basic_value()
            .basic()
            .expect("strlen should return a value")
            .into_int_value();

        let send_fn = compiler.libc().send_fn;

        compiler
            .builder()
            .build_call(
                send_fn,
                &[fd_i32.into(), text_ptr.into(), len.into(), i32_type.const_int(0, false).into()],
                "send.call",
            )
            .map_err(err)?;

        if Compiler::expr_needs_release_in_function_call(&data_arg.value.value.value) {
            compiler.release_value(&data_value, data_arg.value.value.span)?;
        }

        Ok(())
    };

    StdFunction {
        params,
        passed_by: vec![PassedBy::Value, PassedBy::Value],
        execute,
        return_type: Type::Void,
        type_check: None,
        compile,
    }
}
