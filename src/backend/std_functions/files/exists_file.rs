use std::{cell::RefCell, path::Path, rc::Rc, vec};

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

pub fn exists_file() -> StdFunction {
    let params = vec![Type::Str];
    let execute = |params: &Vec<Rc<RefCell<Value>>>, span: Span| -> Result<Option<Value>, StdFunctionError> {
        let fn_name = "exists_file";
        let expected_types = vec![Type::Str];
        let mut actual_types: Vec<Type> = vec![];

        if let Some(filepath) = params.first() {
            actual_types.push(filepath.borrow().to_type());
            let filepath = filepath.borrow();

            match &*filepath {
                Value::String(path) => {
                    let exists = Path::new(path).exists();
                    Ok(Some(Value::Bool(exists)))
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
                String::from("'exists_file' expects exactly one argument."),
                span,
            )) as Box<dyn IError>
        })?;

        compiler.visit_expression(&arg.value.value)?;
        let path_value = compiler.read_last_value()?;

        let path_ptr = match path_value {
            LlvmValue::Str(ptr) => {
                let i8_type = compiler.context().i8_type();
                let i8_ptr_type = compiler.context().ptr_type(AddressSpace::default());

                let data_field = unsafe {
                    compiler
                        .builder()
                        .build_gep(i8_type, ptr, &[compiler.context().i64_type().const_int(8, false)], "str.data.field")
                }
                .map_err(err)?;

                let data = compiler
                    .builder()
                    .build_load(i8_ptr_type, data_field, "str.data")
                    .map_err(err)?
                    .into_pointer_value();

                data
            }

            other => {
                return Err(Box::new(CompilerError::at(
                    ErrorSeverity::HIGH,
                    format!("'exists_file' expects a string, got '{}'.", other.to_type()),
                    span,
                )));
            }
        };

        let i32_type = compiler.context().i32_type();
        let access_fn = compiler.libc().access_fn;

        let result = compiler
            .builder()
            .build_call(access_fn, &[path_ptr.into(), i32_type.const_int(0, false).into()], "access.call")
            .map_err(err)?
            .try_as_basic_value()
            .basic()
            .expect("access should return a value")
            .into_int_value();

        let exists = compiler
            .builder()
            .build_int_compare(inkwell::IntPredicate::EQ, result, i32_type.const_int(0, false), "access.exists")
            .map_err(err)?;

        if Compiler::expr_needs_release_in_function_call(&arg.value.value.value) {
            compiler.release_value(&path_value, arg.value.value.span)?;
        }

        compiler.set_last_value(LlvmValue::Bool(exists));

        Ok(())
    };

    StdFunction {
        params,
        execute,
        passed_by: vec![PassedBy::Value],
        return_type: Type::Bool,
        type_check: None,
        compile,
    }
}
