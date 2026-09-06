use std::{cell::RefCell, fs, rc::Rc, vec};

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

pub fn delete_file() -> StdFunction {
    let params = vec![Type::Str];

    let execute = |params: &Vec<Rc<RefCell<Value>>>, span: Span| -> Result<Option<Value>, StdFunctionError> {
        let fn_name = "delete_file";
        let expected_types = vec![Type::Str];
        let mut actual_types: Vec<Type> = vec![];

        if let Some(filepath) = params.first() {
            actual_types.push(filepath.borrow().to_type());
            let filepath = filepath.borrow();

            match &*filepath {
                Value::String(path) => match fs::remove_file(path) {
                    Ok(_) => Ok(None),
                    Err(_) => Err(StdFunctionError::new(ErrorSeverity::HIGH, String::from("Failed to delete file."), span)),
                },
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
                String::from("'delete_file' expects exactly one argument."),
                span,
            )) as Box<dyn IError>
        })?;

        compiler.visit_expression(&arg.value.value)?;
        let path_value = compiler.read_last_value()?;

        let path_ptr = match &path_value {
            LlvmValue::Str(ptr) => {
                let i8_type = compiler.context().i8_type();
                let i8_ptr_type = compiler.context().ptr_type(AddressSpace::default());

                let data_field = unsafe {
                    compiler
                        .builder()
                        .build_gep(i8_type, *ptr, &[compiler.context().i64_type().const_int(8, false)], "path.data.field")
                }
                .map_err(err)?;

                compiler
                    .builder()
                    .build_load(i8_ptr_type, data_field, "path.data")
                    .map_err(err)?
                    .into_pointer_value()
            }
            other => {
                return Err(Box::new(CompilerError::at(
                    ErrorSeverity::HIGH,
                    format!("'delete_file' expects a string, got '{}'.", other.to_type()),
                    span,
                )));
            }
        };

        let remove_fn = compiler.libc().remove_fn;

        compiler.builder().build_call(remove_fn, &[path_ptr.into()], "remove.call").map_err(err)?;

        if Compiler::expr_needs_release_in_function_call(&arg.value.value.value) {
            compiler.release_value(&path_value, arg.value.value.span)?;
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
