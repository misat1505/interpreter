use inkwell::AddressSpace;

use crate::{
    backend::llvm::{compiler::Compiler, LlvmValue},
    common::{
        errors::{CompilerError, ErrorSeverity, IError},
        span::Span,
        visitor::Visitor,
    },
    frontend::ast::{Argument, Node},
};

pub fn compile_write_or_append<'a, 'ctx>(
    compiler: &mut Compiler<'a, 'ctx>,
    arguments: &'a [Box<Node<Argument>>],
    mode_str: &str,
    span: Span,
) -> Result<(), Box<dyn IError>> {
    let err = |e: inkwell::builder::BuilderError| Box::new(CompilerError::at(ErrorSeverity::HIGH, e.to_string(), span)) as Box<dyn IError>;

    let path_arg = arguments.first().ok_or_else(|| {
        Box::new(CompilerError::at(
            ErrorSeverity::HIGH,
            String::from("Expected a file path argument."),
            span,
        )) as Box<dyn IError>
    })?;

    let content_arg = arguments
        .get(1)
        .ok_or_else(|| Box::new(CompilerError::at(ErrorSeverity::HIGH, String::from("Expected a content argument."), span)) as Box<dyn IError>)?;

    compiler.visit_expression(&path_arg.value.value)?;
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
                format!("Expected a string path, got '{}'.", other.to_type()),
                span,
            )));
        }
    };

    compiler.visit_expression(&content_arg.value.value)?;
    let content_value = compiler.read_last_value()?;

    let content_ptr = match &content_value {
        LlvmValue::Str(ptr) => {
            let i8_type = compiler.context().i8_type();
            let i8_ptr_type = compiler.context().ptr_type(AddressSpace::default());

            let data_field = unsafe {
                compiler
                    .builder()
                    .build_gep(i8_type, *ptr, &[compiler.context().i64_type().const_int(8, false)], "content.data.field")
            }
            .map_err(err)?;

            compiler
                .builder()
                .build_load(i8_ptr_type, data_field, "content.data")
                .map_err(err)?
                .into_pointer_value()
        }
        other => {
            return Err(Box::new(CompilerError::at(
                ErrorSeverity::HIGH,
                format!("Expected string content, got '{}'.", other.to_type()),
                span,
            )));
        }
    };

    let mode = compiler
        .builder()
        .build_global_string_ptr(mode_str, "mode")
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

    if Compiler::expr_needs_release_in_function_call(&path_arg.value.value.value) {
        compiler.release_value(&path_value, path_arg.value.value.span)?;
    }

    let strlen_fn = compiler.libc().strlen_fn;
    let len = compiler
        .builder()
        .build_call(strlen_fn, &[content_ptr.into()], "content.len")
        .map_err(err)?
        .try_as_basic_value()
        .basic()
        .expect("strlen should return a value")
        .into_int_value();

    let i64_type = compiler.context().i64_type();
    let fwrite_fn = compiler.libc().fwrite_fn;

    compiler
        .builder()
        .build_call(
            fwrite_fn,
            &[content_ptr.into(), i64_type.const_int(1, false).into(), len.into(), file.into()],
            "fwrite.call",
        )
        .map_err(err)?;

    let fclose_fn = compiler.libc().fclose_fn;
    compiler.builder().build_call(fclose_fn, &[file.into()], "fclose.call").map_err(err)?;

    if Compiler::expr_needs_release_in_function_call(&content_arg.value.value.value) {
        compiler.release_value(&content_value, content_arg.value.value.span)?;
    }

    Ok(())
}
