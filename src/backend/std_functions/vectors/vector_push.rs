use std::{cell::RefCell, rc::Rc, vec};

use inkwell::AddressSpace;

use crate::{
    backend::llvm::llvm_alu::llvm_value::{VEC_CAPACITY, VEC_DATA, VEC_LENGTH},
    backend::{
        interpreter::Value,
        llvm::LlvmValue,
        std_functions::std_functions::{build_usage_error, LlvmCompileFn, StdFunction},
        type_utils::type_accepts_value,
    },
    common::{
        errors::{CompilerError, ErrorSeverity, IError, StdFunctionError},
        span::Span,
        types::Type,
        visitor::Visitor,
    },
    frontend::ast::PassedBy,
};

pub fn vector_push() -> StdFunction {
    let params = vec![Type::Vector(Box::new(Type::Void)), Type::Void];

    let execute = |params: &Vec<Rc<RefCell<Value>>>, span: Span| -> Result<Option<Value>, StdFunctionError> {
        let fn_name = "vector_push";
        let expected_types = vec![Type::Vector(Box::new(Type::Void)), Type::Void];

        let mut actual_types: Vec<Type> = vec![];

        if let (Some(vector), Some(value)) = (params.first(), params.get(1)) {
            actual_types.push(vector.borrow().to_type());
            actual_types.push(value.borrow().to_type());

            let mut vector = vector.borrow_mut();

            match &mut *vector {
                Value::Vector { kind, values } => {
                    if let Type::Vector(inner) = kind.as_ref() {
                        {
                            let value_ref = value.borrow();

                            if !type_accepts_value(inner, &value_ref) {
                                return Err(build_usage_error(fn_name, expected_types, actual_types, span));
                            }
                        }

                        values.borrow_mut().push(Rc::clone(value));

                        Ok(None)
                    } else {
                        Err(build_usage_error(fn_name, expected_types, actual_types, span))
                    }
                }

                _ => Err(build_usage_error(fn_name, expected_types, actual_types, span)),
            }
        } else {
            Err(build_usage_error(fn_name, expected_types, actual_types, span))
        }
    };

    let type_check: fn(&[Type]) -> Result<Type, String> = |arg_types: &[Type]| match arg_types {
        [Type::Vector(inner), value_type] => {
            if inner.is_compatible(value_type) {
                Ok(Type::Void)
            } else {
                Err(format!("vector_push expected element of type '{}', but got '{}'.", inner, value_type))
            }
        }
        [other, _] => Err(format!("vector_push expected a vector as first argument, but got '{}'.", other)),
        _ => Err(String::from("vector_push expects exactly 2 arguments.")),
    };

    let compile: LlvmCompileFn = |compiler, arguments, span| {
        let err = |e: inkwell::builder::BuilderError| Box::new(CompilerError::at(ErrorSeverity::HIGH, e.to_string(), span)) as Box<dyn IError>;

        let err_arity = || {
            Box::new(CompilerError::at(
                ErrorSeverity::HIGH,
                String::from("'vector_push' expects exactly 2 arguments."),
                span,
            )) as Box<dyn IError>
        };

        let vector_arg = arguments.first().ok_or_else(err_arity)?;
        let value_arg = arguments.get(1).ok_or_else(err_arity)?;

        let vector_slot_ptr = compiler.resolve_reference(&vector_arg.value.value)?;

        compiler.visit_expression(&vector_arg.value.value)?;
        let vector_value = compiler.read_last_value()?;

        let vector_type = compiler.resolve_type(&vector_value.to_type());

        let inner_type = match vector_type {
            Type::Vector(inner) => compiler.resolve_type(&inner),
            other => {
                return Err(Box::new(CompilerError::at(
                    ErrorSeverity::HIGH,
                    format!("'vector_push' expects a vector, got '{}'.", other),
                    span,
                )))
            }
        };

        if crate::backend::llvm::compiler::Compiler::expr_needs_release(&vector_arg.value.value.value) {
            compiler.release_value(&vector_value, span)?;
        }

        compiler.visit_expression(&value_arg.value.value)?;
        let pushed_value = compiler.read_last_value()?;

        if !inner_type.is_compatible(&pushed_value.to_type()) {
            return Err(Box::new(CompilerError::at(
                ErrorSeverity::HIGH,
                format!(
                    "Vector element type mismatch: expected '{}', got '{}'.",
                    inner_type,
                    pushed_value.to_type()
                ),
                span,
            )));
        }

        let pushed_value = compiler.finalize_owned_value_for_new_slot(pushed_value, &value_arg.value.value.value, span)?;

        let context = compiler.context();
        let ptr_type = context.ptr_type(AddressSpace::default());
        let i64_type = context.i64_type();
        let struct_type = LlvmValue::vector_struct_type(context);

        let function = compiler
            .builder()
            .get_insert_block()
            .expect("builder should be positioned inside a function")
            .get_parent()
            .expect("basic block should belong to a function");

        let struct_ptr = compiler
            .builder()
            .build_load(ptr_type, vector_slot_ptr, "vector.ptr")
            .map_err(err)?
            .into_pointer_value();

        let data_field = compiler
            .builder()
            .build_struct_gep(struct_type, struct_ptr, VEC_DATA, "vector.data")
            .map_err(err)?;

        let length_field = compiler
            .builder()
            .build_struct_gep(struct_type, struct_ptr, VEC_LENGTH, "vector.length")
            .map_err(err)?;

        let capacity_field = compiler
            .builder()
            .build_struct_gep(struct_type, struct_ptr, VEC_CAPACITY, "vector.capacity")
            .map_err(err)?;

        let old_length = compiler
            .builder()
            .build_load(i64_type, length_field, "vector.length.old")
            .map_err(err)?
            .into_int_value();

        let old_capacity = compiler
            .builder()
            .build_load(i64_type, capacity_field, "vector.capacity.old")
            .map_err(err)?
            .into_int_value();

        let element_llvm_type = LlvmValue::type_to_basic_type_enum(&inner_type, context).ok_or_else(|| {
            Box::new(CompilerError::at(
                ErrorSeverity::HIGH,
                format!("Compiling vectors of type '{}' is not yet supported. 7", inner_type),
                span,
            )) as Box<dyn IError>
        })?;

        let element_size = LlvmValue::element_byte_size(&inner_type, i64_type, span)?;

        let needs_grow = compiler
            .builder()
            .build_int_compare(inkwell::IntPredicate::SGE, old_length, old_capacity, "needs.grow")
            .map_err(err)?;

        let grow_block = context.append_basic_block(function, "push.grow");
        let merge_block = context.append_basic_block(function, "push.merge");

        compiler
            .builder()
            .build_conditional_branch(needs_grow, grow_block, merge_block)
            .map_err(err)?;

        compiler.builder().position_at_end(grow_block);

        let old_data = compiler
            .builder()
            .build_load(ptr_type, data_field, "vector.data.old")
            .map_err(err)?
            .into_pointer_value();

        let is_zero = compiler
            .builder()
            .build_int_compare(inkwell::IntPredicate::EQ, old_capacity, i64_type.const_int(0, false), "cap.is_zero")
            .map_err(err)?;

        let doubled = compiler
            .builder()
            .build_int_mul(old_capacity, i64_type.const_int(2, false), "cap.doubled")
            .map_err(err)?;

        let new_capacity = compiler
            .builder()
            .build_select(is_zero, i64_type.const_int(1, false), doubled, "cap.new")
            .map_err(err)?
            .into_int_value();

        let new_bytes = compiler
            .builder()
            .build_int_mul(new_capacity, element_size, "vector.bytes.new")
            .map_err(err)?;

        let new_data = compiler
            .builder()
            .build_call(compiler.libc().realloc_fn, &[old_data.into(), new_bytes.into()], "vector.realloc")
            .map_err(err)?
            .try_as_basic_value()
            .basic()
            .expect("realloc should return a value")
            .into_pointer_value();

        compiler.builder().build_store(data_field, new_data).map_err(err)?;

        compiler.builder().build_store(capacity_field, new_capacity).map_err(err)?;

        compiler.builder().build_unconditional_branch(merge_block).map_err(err)?;

        compiler.builder().position_at_end(merge_block);

        let current_data = compiler
            .builder()
            .build_load(ptr_type, data_field, "vector.data.current")
            .map_err(err)?
            .into_pointer_value();

        let elem_ptr = unsafe {
            compiler
                .builder()
                .build_gep(element_llvm_type, current_data, &[old_length], "vector.push.elem")
                .map_err(err)?
        };

        compiler
            .builder()
            .build_store(elem_ptr, pushed_value.as_basic_value_enum())
            .map_err(err)?;

        let new_length = compiler
            .builder()
            .build_int_add(old_length, i64_type.const_int(1, false), "vector.length.new")
            .map_err(err)?;

        compiler.builder().build_store(length_field, new_length).map_err(err)?;

        Ok(())
    };

    StdFunction {
        params,
        passed_by: vec![PassedBy::Reference, PassedBy::Value],
        execute,
        return_type: Type::Void,
        type_check: Some(type_check),
        compile,
    }
}
