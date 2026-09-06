use std::{cell::RefCell, io::Read, rc::Rc, vec};

use crate::{
    backend::{
        interpreter::Value,
        llvm::LlvmValue,
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

pub fn tcp_read() -> StdFunction {
    let params = vec![Type::I64];

    let execute = |params: &Vec<Rc<RefCell<Value>>>, span: Span| -> Result<Option<Value>, StdFunctionError> {
        let fn_name = "tcp_read";
        let expected_types = vec![Type::I64];
        let mut actual_types: Vec<Type> = vec![];

        if let Some(handle) = params.first() {
            actual_types.push(handle.borrow().to_type());

            let handle = handle.borrow();

            match &*handle {
                Value::I64(stream_handle) => {
                    let mut streams = STREAMS.lock().unwrap();
                    let stream = streams
                        .as_mut()
                        .and_then(|m| m.get_mut(stream_handle))
                        .ok_or_else(|| StdFunctionError::new(ErrorSeverity::HIGH, format!("Invalid stream handle {}", stream_handle), span))?;

                    let mut buffer = [0u8; 4096];
                    let n = stream
                        .read(&mut buffer)
                        .map_err(|e| StdFunctionError::new(ErrorSeverity::HIGH, format!("Read failed: {}", e), span))?;

                    Ok(Some(Value::String(String::from_utf8_lossy(&buffer[..n]).to_string())))
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
                String::from("'tcp_read' expects exactly one argument."),
                span,
            )) as Box<dyn IError>
        })?;

        compiler.visit_expression(&arg.value.value)?;
        let fd = compiler.read_last_value()?.into_i64_value(span)?;

        let context = compiler.context();
        let i32_type = context.i32_type();
        let i64_type = context.i64_type();
        let i8_type = context.i8_type();

        let buf_size = 4096u64;

        let fd_i32 = compiler.builder().build_int_truncate(fd, i32_type, "fd.i32").map_err(err)?;

        let malloc_fn = compiler.libc().malloc_fn;

        let buf = compiler
            .builder()
            .build_call(malloc_fn, &[i64_type.const_int(buf_size, false).into()], "recv.buf")
            .map_err(err)?
            .try_as_basic_value()
            .basic()
            .expect("malloc should return a value")
            .into_pointer_value();

        let recv_fn = compiler.libc().recv_fn;

        let n = compiler
            .builder()
            .build_call(
                recv_fn,
                &[
                    fd_i32.into(),
                    buf.into(),
                    i64_type.const_int(buf_size - 1, false).into(),
                    i32_type.const_int(0, false).into(),
                ],
                "recv.call",
            )
            .map_err(err)?
            .try_as_basic_value()
            .basic()
            .expect("recv should return a value")
            .into_int_value();

        let end_ptr = unsafe { compiler.builder().build_gep(i8_type, buf, &[n], "recv.end").map_err(err)? };

        compiler.builder().build_store(end_ptr, i8_type.const_int(0, false)).map_err(err)?;

        let header = compiler
            .builder()
            .build_call(malloc_fn, &[i64_type.const_int(16, false).into()], "recv.header")
            .map_err(err)?
            .try_as_basic_value()
            .basic()
            .expect("malloc should return a value")
            .into_pointer_value();

        let refcount_field = unsafe {
            compiler
                .builder()
                .build_gep(i8_type, header, &[i64_type.const_zero()], "recv.refcount.field")
                .map_err(err)?
        };

        compiler
            .builder()
            .build_store(refcount_field, i64_type.const_int(1, false))
            .map_err(err)?;

        let data_field = unsafe {
            compiler
                .builder()
                .build_gep(i8_type, header, &[i64_type.const_int(8, false)], "recv.data.field")
                .map_err(err)?
        };

        compiler.builder().build_store(data_field, buf).map_err(err)?;

        compiler.set_last_value(LlvmValue::Str(header));

        Ok(())
    };

    StdFunction {
        params,
        passed_by: vec![PassedBy::Value],
        execute,
        return_type: Type::Str,
        type_check: None,
        compile,
    }
}
