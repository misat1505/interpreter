use std::{cell::RefCell, rc::Rc, vec};

use inkwell::AddressSpace;

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

pub fn println() -> StdFunction {
    let params = vec![Type::Str];

    let execute = |params: &Vec<Rc<RefCell<Value>>>, span: Span| -> Result<Option<Value>, StdFunctionError> {
        let fn_name = "println";
        let expected_types = vec![Type::Str];
        let mut actual_types: Vec<Type> = vec![];

        if let Some(value) = params.first() {
            actual_types.push(value.borrow().to_type());

            let value = value.borrow();

            match &*value {
                Value::String(text) => {
                    println!("{}", text);
                    Ok(None)
                }
                _ => Err(build_usage_error(fn_name, expected_types, actual_types, span)),
            }
        } else {
            Err(build_usage_error(fn_name, expected_types, actual_types, span))
        }
    };

    let compile: LlvmCompileFn = |compiler, arguments, span| {
        let err = |err: inkwell::builder::BuilderError| Box::new(CompilerError::at(ErrorSeverity::HIGH, err.to_string(), span)) as Box<dyn IError>;

        let arg = arguments.first().ok_or_else(|| {
            Box::new(CompilerError::at(
                ErrorSeverity::HIGH,
                String::from("'println' expects exactly one argument."),
                span,
            )) as Box<dyn IError>
        })?;

        compiler.visit_expression(&arg.value.value)?;
        let text_value = compiler.read_last_value()?;

        let text_ptr = match text_value {
            LlvmValue::Str(ptr) => {
                let i8_type = compiler.context().i8_type();
                let i8_ptr_type = compiler.context().ptr_type(AddressSpace::default());

                let data_field = unsafe {
                    compiler
                        .builder()
                        .build_gep(i8_type, ptr, &[compiler.context().i64_type().const_int(8, false)], "str.data.field")
                }
                .map_err(err)?;

                compiler
                    .builder()
                    .build_load(i8_ptr_type, data_field, "str.data")
                    .map_err(err)?
                    .into_pointer_value()
            }

            other => {
                return Err(Box::new(CompilerError::at(
                    ErrorSeverity::HIGH,
                    format!("'println' expects a string, got '{}'.", other.to_type()),
                    span,
                )));
            }
        };

        let printf_fn = compiler.libc().printf_fn;

        let format_str = compiler.builder().build_global_string_ptr("%s\n", "fmt").map_err(err)?;

        compiler
            .builder()
            .build_call(printf_fn, &[format_str.as_pointer_value().into(), text_ptr.into()], "printf_call")
            .map_err(err)?;

        let null_stream = compiler.context().ptr_type(AddressSpace::default()).const_null();

        compiler
            .builder()
            .build_call(compiler.libc().fflush_fn, &[null_stream.into()], "fflush.stdout")
            .map_err(err)?;

        if Compiler::expr_needs_release_in_function_call(&arg.value.value.value) {
            compiler.release_value(&text_value, arg.value.value.span)?;
        }

        Ok(())
    };

    StdFunction {
        params,
        execute,
        passed_by: vec![PassedBy::Value],
        return_type: Type::Void,
        type_check: None,
        compile,
    }
}
