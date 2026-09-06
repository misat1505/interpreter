use std::{
    cell::RefCell,
    io::{self, Write},
    rc::Rc,
    vec,
};

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

pub fn input() -> StdFunction {
    let params = vec![Type::Str];

    let execute = |params: &Vec<Rc<RefCell<Value>>>, span: Span| -> Result<Option<Value>, StdFunctionError> {
        let fn_name = "input";
        let expected_types = vec![Type::Str];
        let mut actual_types: Vec<Type> = vec![];

        if let Some(value) = params.first() {
            actual_types.push(value.borrow().to_type());
            let value = value.borrow();

            match &*value {
                Value::String(prompt) => {
                    print!("{}", prompt);
                    io::stdout().flush().unwrap();

                    let mut input = String::new();

                    match io::stdin().read_line(&mut input) {
                        Ok(_) => Ok(Some(Value::String(input.trim().to_string()))),
                        Err(_) => Err(StdFunctionError::new(ErrorSeverity::HIGH, String::from("Failed to read input."), span)),
                    }
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
                String::from("'input' expects exactly one argument."),
                span,
            )) as Box<dyn IError>
        })?;

        compiler.visit_expression(&arg.value.value)?;
        let prompt_value = compiler.read_last_value()?;

        let prompt_ptr = match &prompt_value {
            LlvmValue::Str(ptr) => {
                let i8_type = compiler.context().i8_type();
                let i8_ptr_type = compiler.context().ptr_type(AddressSpace::default());

                let data_field = unsafe {
                    compiler
                        .builder()
                        .build_gep(i8_type, *ptr, &[compiler.context().i64_type().const_int(8, false)], "prompt.data.field")
                }
                .map_err(err)?;

                compiler
                    .builder()
                    .build_load(i8_ptr_type, data_field, "prompt.data")
                    .map_err(err)?
                    .into_pointer_value()
            }
            other => {
                return Err(Box::new(CompilerError::at(
                    ErrorSeverity::HIGH,
                    format!("'input' expects a string, got '{}'.", other.to_type()),
                    span,
                )));
            }
        };

        let format_str = compiler
            .builder()
            .build_global_string_ptr("%s", "fmt.prompt")
            .map_err(err)?
            .as_pointer_value();

        let printf_fn = compiler.libc().printf_fn;

        compiler
            .builder()
            .build_call(printf_fn, &[format_str.into(), prompt_ptr.into()], "printf.prompt")
            .map_err(err)?;

        let null_stream = compiler.context().ptr_type(AddressSpace::default()).const_null();

        compiler
            .builder()
            .build_call(compiler.libc().fflush_fn, &[null_stream.into()], "fflush.stdout")
            .map_err(err)?;

        if Compiler::expr_needs_release_in_function_call(&arg.value.value.value) {
            compiler.release_value(&prompt_value, arg.value.value.span)?;
        }

        let context = compiler.context();
        let i64_type = context.i64_type();
        let i32_type = context.i32_type();
        let i8_type = context.i8_type();

        const BUF_SIZE: u64 = 4096;

        let malloc_fn = compiler.libc().malloc_fn;

        let buf = compiler
            .builder()
            .build_call(malloc_fn, &[i64_type.const_int(BUF_SIZE, false).into()], "input.buf")
            .map_err(err)?
            .try_as_basic_value()
            .basic()
            .expect("malloc should return a value")
            .into_pointer_value();

        let read_fn = compiler.libc().read_fn;

        let n = compiler
            .builder()
            .build_call(
                read_fn,
                &[
                    i32_type.const_int(0, false).into(),
                    buf.into(),
                    i64_type.const_int(BUF_SIZE - 1, false).into(),
                ],
                "read.call",
            )
            .map_err(err)?
            .try_as_basic_value()
            .basic()
            .expect("read should return a value")
            .into_int_value();

        let end_ptr = unsafe { compiler.builder().build_gep(i8_type, buf, &[n], "input.end").map_err(err)? };

        compiler.builder().build_store(end_ptr, i8_type.const_int(0, false)).map_err(err)?;

        let zero = i64_type.const_zero();
        let one = i64_type.const_int(1, false);

        let has_input = compiler
            .builder()
            .build_int_compare(inkwell::IntPredicate::UGT, n, zero, "input.has_input")
            .map_err(err)?;

        let current_function = compiler
            .builder()
            .get_insert_block()
            .and_then(|block| block.get_parent())
            .ok_or_else(|| {
                Box::new(CompilerError::at(
                    ErrorSeverity::HIGH,
                    "Cannot get current function while compiling input.".to_string(),
                    span,
                )) as Box<dyn IError>
            })?;

        let check_newline_block = context.append_basic_block(current_function, "input.check_newline");

        let done_block = context.append_basic_block(current_function, "input.done");

        compiler
            .builder()
            .build_conditional_branch(has_input, check_newline_block, done_block)
            .map_err(err)?;

        compiler.builder().position_at_end(check_newline_block);

        let last_index = compiler.builder().build_int_sub(n, one, "input.last_index").map_err(err)?;

        let last_ptr = unsafe { compiler.builder().build_gep(i8_type, buf, &[last_index], "input.last").map_err(err)? };

        let last_char = compiler
            .builder()
            .build_load(i8_type, last_ptr, "input.last_char")
            .map_err(err)?
            .into_int_value();

        let newline = i8_type.const_int(b'\n' as u64, false);

        let is_newline = compiler
            .builder()
            .build_int_compare(inkwell::IntPredicate::EQ, last_char, newline, "input.is_newline")
            .map_err(err)?;

        let remove_newline_block = context.append_basic_block(current_function, "input.remove_newline");

        compiler
            .builder()
            .build_conditional_branch(is_newline, remove_newline_block, done_block)
            .map_err(err)?;

        compiler.builder().position_at_end(remove_newline_block);

        compiler.builder().build_store(last_ptr, i8_type.const_int(0, false)).map_err(err)?;

        compiler.builder().build_unconditional_branch(done_block).map_err(err)?;

        compiler.builder().position_at_end(done_block);

        let header = compiler
            .builder()
            .build_call(malloc_fn, &[i64_type.const_int(16, false).into()], "input.header")
            .map_err(err)?
            .try_as_basic_value()
            .basic()
            .expect("malloc should return a value")
            .into_pointer_value();

        let refcount_field = unsafe {
            compiler
                .builder()
                .build_gep(i8_type, header, &[i64_type.const_zero()], "input.refcount.field")
                .map_err(err)?
        };

        compiler
            .builder()
            .build_store(refcount_field, i64_type.const_int(1, false))
            .map_err(err)?;

        let data_field = unsafe {
            compiler
                .builder()
                .build_gep(i8_type, header, &[i64_type.const_int(8, false)], "input.data.field")
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
