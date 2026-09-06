use std::{cell::RefCell, rc::Rc, thread, time, vec};

use crate::{
    backend::{
        interpreter::Value,
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

pub fn sleep_ms() -> StdFunction {
    let params = vec![Type::I64];

    let execute = |params: &Vec<Rc<RefCell<Value>>>, span: Span| -> Result<Option<Value>, StdFunctionError> {
        let fn_name = "sleep_ms";
        let expected_types = vec![Type::I64];
        let mut actual_types: Vec<Type> = vec![];

        if let Some(millis) = params.first() {
            actual_types.push(millis.borrow().to_type());

            let millis = millis.borrow();

            match &*millis {
                Value::I64(ms) => {
                    let duration = time::Duration::from_millis(*ms as u64);
                    thread::sleep(duration);
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
                String::from("'sleep_ms' expects exactly one argument."),
                span,
            )) as Box<dyn IError>
        })?;

        compiler.visit_expression(&arg.value.value)?;
        let ms_value = compiler.read_last_value()?.into_i64_value(span)?;

        let context = compiler.context();
        let i32_type = context.i32_type();
        let i64_type = context.i64_type();

        let micros_i64 = compiler
            .builder()
            .build_int_mul(ms_value, i64_type.const_int(1000, false), "sleep.micros")
            .map_err(err)?;

        let micros_i32 = compiler
            .builder()
            .build_int_truncate(micros_i64, i32_type, "sleep.micros.i32")
            .map_err(err)?;

        let usleep_fn = compiler.libc().usleep_fn;
        compiler
            .builder()
            .build_call(usleep_fn, &[micros_i32.into()], "usleep.call")
            .map_err(err)?;

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
