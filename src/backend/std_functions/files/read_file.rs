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

pub fn read_file() -> StdFunction {
    let params = vec![Type::Str];
    let execute = |params: &Vec<Rc<RefCell<Value>>>, span: Span| -> Result<Option<Value>, StdFunctionError> {
        let fn_name = "read_file";
        let expected_types = vec![Type::Str];
        let mut actual_types: Vec<Type> = vec![];

        if let Some(filepath) = params.first() {
            actual_types.push(filepath.borrow().to_type());
            let filepath = filepath.borrow();

            match &*filepath {
                Value::String(path) => match fs::read_to_string(path) {
                    Ok(content) => Ok(Some(Value::String(content))),
                    Err(_) => Err(StdFunctionError::new(ErrorSeverity::HIGH, String::from("Failed to read file."), span)),
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
                String::from("'read_file' expects exactly one argument."),
                span,
            )) as Box<dyn IError>
        })?;

        compiler.visit_expression(&arg.value.value)?;
        let path_value = compiler.read_last_value()?;

        let context = compiler.context();
        let i8_type = context.i8_type();
        let i8_ptr_type = context.ptr_type(AddressSpace::default());
        let i64_type = context.i64_type();
        let i32_type = context.i32_type();

        let path_ptr = match &path_value {
            LlvmValue::Str(ptr) => {
                let data_field = unsafe {
                    compiler
                        .builder()
                        .build_gep(i8_type, *ptr, &[i64_type.const_int(8, false)], "path.data.field")
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
                    format!("'read_file' expects a string, got '{}'.", other.to_type()),
                    span,
                )));
            }
        };

        let mode = compiler
            .builder()
            .build_global_string_ptr("rb", "mode.r")
            .map_err(err)?
            .as_pointer_value();

        let fopen_fn = compiler.libc().fopen_fn;

        let file = compiler
            .builder()
            .build_call(fopen_fn, &[path_ptr.into(), mode.into()], "fopen.call")
            .map_err(err)?
            .try_as_basic_value()
            .basic()
            .expect("fopen should return a value")
            .into_pointer_value();

        if Compiler::expr_needs_release_in_function_call(&arg.value.value.value) {
            compiler.release_value(&path_value, arg.value.value.span)?;
        }

        let fseek_fn = compiler.libc().fseek_fn;
        let ftell_fn = compiler.libc().ftell_fn;

        compiler
            .builder()
            .build_call(
                fseek_fn,
                &[file.into(), i64_type.const_int(0, false).into(), i32_type.const_int(2, false).into()],
                "fseek.end",
            )
            .map_err(err)?;

        let size = compiler
            .builder()
            .build_call(ftell_fn, &[file.into()], "ftell.call")
            .map_err(err)?
            .try_as_basic_value()
            .basic()
            .expect("ftell should return a value")
            .into_int_value();

        compiler
            .builder()
            .build_call(
                fseek_fn,
                &[file.into(), i64_type.const_int(0, false).into(), i32_type.const_int(0, false).into()],
                "fseek.start",
            )
            .map_err(err)?;

        let size_plus_nul = compiler
            .builder()
            .build_int_add(size, i64_type.const_int(1, false), "size.nul")
            .map_err(err)?;

        let malloc_fn = compiler.libc().malloc_fn;

        let buf = compiler
            .builder()
            .build_call(malloc_fn, &[size_plus_nul.into()], "read.buf")
            .map_err(err)?
            .try_as_basic_value()
            .basic()
            .expect("malloc should return a value")
            .into_pointer_value();

        let fread_fn = compiler.libc().fread_fn;

        compiler
            .builder()
            .build_call(
                fread_fn,
                &[buf.into(), i64_type.const_int(1, false).into(), size.into(), file.into()],
                "fread.call",
            )
            .map_err(err)?;

        let fclose_fn = compiler.libc().fclose_fn;

        compiler.builder().build_call(fclose_fn, &[file.into()], "fclose.call").map_err(err)?;

        let end_ptr = unsafe { compiler.builder().build_gep(i8_type, buf, &[size], "read.end").map_err(err)? };

        compiler.builder().build_store(end_ptr, i8_type.const_int(0, false)).map_err(err)?;

        let header_size = i64_type.const_int(16, false);

        let header = compiler
            .builder()
            .build_call(malloc_fn, &[header_size.into()], "str.header.alloc")
            .map_err(err)?
            .try_as_basic_value()
            .basic()
            .expect("malloc should return a value")
            .into_pointer_value();

        let refcount_field = unsafe {
            compiler
                .builder()
                .build_gep(i8_type, header, &[i64_type.const_int(0, false)], "str.refcount.field")
                .map_err(err)?
        };

        let refcount_ptr = refcount_field;

        compiler.builder().build_store(refcount_ptr, i64_type.const_int(1, false)).map_err(err)?;

        let data_field = unsafe {
            compiler
                .builder()
                .build_gep(i8_type, header, &[i64_type.const_int(8, false)], "str.data.field")
                .map_err(err)?
        };

        compiler.builder().build_store(data_field, buf).map_err(err)?;

        compiler.set_last_value(LlvmValue::Str(header));

        Ok(())
    };

    StdFunction {
        params,
        execute,
        passed_by: vec![PassedBy::Value],
        return_type: Type::Str,
        type_check: None,
        compile,
    }
}
