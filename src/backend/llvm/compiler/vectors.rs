use inkwell::values::PointerValue;
use inkwell::{AddressSpace, IntPredicate};

use super::Compiler;
use crate::common::visitor::Visitor;
use crate::frontend::ast::Accessor;
use crate::{
    backend::llvm::llvm_alu::llvm_value::{LlvmValue, VEC_CAPACITY, VEC_DATA, VEC_LENGTH, VEC_REFCOUNT},
    common::{
        errors::{CompilerError, ErrorSeverity, IError},
        span::Span,
        types::Type,
    },
    frontend::ast::{Expression, Node},
};

impl<'a, 'ctx> Compiler<'a, 'ctx> {
    /// Allocates a fresh, empty `VecHeader { refcount: 1, data: null, length: 0, capacity: 0 }`.
    pub(in crate::backend::llvm::compiler) fn build_empty_vector(
        &mut self,
        inner_type: &Type,
        span: Span,
    ) -> Result<PointerValue<'ctx>, Box<dyn IError>> {
        let err = Self::builder_err(span);
        let struct_type = LlvmValue::vector_struct_type(self.context);

        let struct_size = struct_type.size_of().expect("VecHeader must have a known size");
        let struct_ptr = self
            .builder
            .build_call(self.libc.malloc_fn, &[struct_size.into()], "vector.header.malloc")
            .map_err(&err)?
            .try_as_basic_value()
            .basic()
            .expect("malloc should return a value")
            .into_pointer_value();

        let ptr_type = self.context.ptr_type(AddressSpace::default());
        let i64_type = self.context.i64_type();

        let refcount_field = self
            .builder
            .build_struct_gep(struct_type, struct_ptr, VEC_REFCOUNT, "vector.refcount")
            .map_err(&err)?;
        self.builder.build_store(refcount_field, i64_type.const_int(1, false)).map_err(&err)?;

        let data_field = self
            .builder
            .build_struct_gep(struct_type, struct_ptr, VEC_DATA, "vector.data")
            .map_err(&err)?;
        self.builder.build_store(data_field, ptr_type.const_null()).map_err(&err)?;

        let length_field = self
            .builder
            .build_struct_gep(struct_type, struct_ptr, VEC_LENGTH, "vector.length")
            .map_err(&err)?;
        self.builder.build_store(length_field, i64_type.const_int(0, false)).map_err(&err)?;

        let capacity_field = self
            .builder
            .build_struct_gep(struct_type, struct_ptr, VEC_CAPACITY, "vector.capacity")
            .map_err(&err)?;
        self.builder.build_store(capacity_field, i64_type.const_int(0, false)).map_err(&err)?;

        let _ = inner_type;

        Ok(struct_ptr)
    }

    pub(in crate::backend::llvm::compiler) fn build_vector_from_elements(
        &mut self,
        inner_type: &Type,
        elements: &'a [Box<Node<Expression>>],
        precomputed_first: Option<LlvmValue<'ctx>>,
        span: Span,
    ) -> Result<PointerValue<'ctx>, Box<dyn IError>> {
        let err = Self::builder_err(span);

        let element_llvm_type = LlvmValue::type_to_basic_type_enum(inner_type, self.context).ok_or_else(|| {
            Box::new(CompilerError::at(
                ErrorSeverity::HIGH,
                format!("Compiling vectors of type '{}' is not yet supported. 4", inner_type),
                span,
            )) as Box<dyn IError>
        })?;

        let count = elements.len() as u64;
        let i64_type = self.context.i64_type();

        let element_size = LlvmValue::element_byte_size(inner_type, i64_type, span)?;

        let total_size = self
            .builder
            .build_int_mul(element_size, i64_type.const_int(count, false), "vector.bytes")
            .map_err(&err)?;

        let data_ptr = self
            .builder
            .build_call(self.libc.malloc_fn, &[total_size.into()], "vector.malloc")
            .map_err(&err)?
            .try_as_basic_value()
            .basic()
            .expect("malloc should return a value")
            .into_pointer_value();

        for (index, element) in elements.iter().enumerate() {
            let element_value = if index == 0 {
                if let Some(value) = precomputed_first.clone() {
                    value
                } else {
                    self.evaluate_vector_element(inner_type, element)?
                }
            } else {
                self.evaluate_vector_element(inner_type, element)?
            };

            if !inner_type.is_compatible(&element_value.to_type()) {
                return Err(Box::new(CompilerError::at(
                    ErrorSeverity::HIGH,
                    format!(
                        "Vector element type mismatch: expected '{}', got '{}'.",
                        inner_type,
                        element_value.to_type()
                    ),
                    element.span,
                )));
            }

            // The element is being stored into a brand new, independently
            // owned vector slot. Strings are always deep-copied. Vector /
            // Struct elements need an explicit retain only if the source
            // expression was a bare variable read (a "borrow") - anything
            // else (fresh literal, field/element read, function call, ...)
            // already evaluates to an owned +1 reference.
            let element_value = match &element_value {
                LlvmValue::Str(str_ptr) => {
                    let copied = self.build_string_copy(*str_ptr, element.span)?;
                    if Self::expr_needs_release(&element.as_ref().value) {
                        self.release_value(&element_value, span)?;
                    }
                    LlvmValue::Str(copied)
                }

                LlvmValue::Vector(_, _) | LlvmValue::Struct(_, _) => {
                    if Self::expr_needs_retain(&element.value) {
                        self.retain_value(&element_value, element.span)?;
                    }
                    element_value
                }

                _ => element_value,
            };

            let element_ptr = unsafe {
                self.builder
                    .build_gep(element_llvm_type, data_ptr, &[i64_type.const_int(index as u64, false)], "vector.elem")
                    .map_err(|err| Box::new(CompilerError::at(ErrorSeverity::HIGH, err.to_string(), element.span)) as Box<dyn IError>)?
            };

            self.builder
                .build_store(element_ptr, element_value.as_basic_value_enum())
                .map_err(|err| Box::new(CompilerError::at(ErrorSeverity::HIGH, err.to_string(), element.span)) as Box<dyn IError>)?;
        }

        let struct_type = LlvmValue::vector_struct_type(self.context);
        let struct_size = struct_type.size_of().expect("VecHeader must have a known size");
        let struct_ptr = self
            .builder
            .build_call(self.libc.malloc_fn, &[struct_size.into()], "vector.header.malloc")
            .map_err(&err)?
            .try_as_basic_value()
            .basic()
            .expect("malloc should return a value")
            .into_pointer_value();

        let refcount_field = self
            .builder
            .build_struct_gep(struct_type, struct_ptr, VEC_REFCOUNT, "vector.refcount.field")
            .map_err(&err)?;
        self.builder.build_store(refcount_field, i64_type.const_int(1, false)).map_err(&err)?;

        let data_field = self
            .builder
            .build_struct_gep(struct_type, struct_ptr, VEC_DATA, "vector.data.field")
            .map_err(&err)?;
        self.builder.build_store(data_field, data_ptr).map_err(&err)?;

        let length_field = self
            .builder
            .build_struct_gep(struct_type, struct_ptr, VEC_LENGTH, "vector.length.field")
            .map_err(&err)?;
        self.builder.build_store(length_field, i64_type.const_int(count, false)).map_err(&err)?;

        let capacity_field = self
            .builder
            .build_struct_gep(struct_type, struct_ptr, VEC_CAPACITY, "vector.capacity.field")
            .map_err(&err)?;
        self.builder.build_store(capacity_field, i64_type.const_int(count, false)).map_err(&err)?;

        Ok(struct_ptr)
    }

    pub(in crate::backend::llvm::compiler) fn evaluate_vector_element(
        &mut self,
        inner_type: &Type,
        element: &'a Node<Expression>,
    ) -> Result<LlvmValue<'ctx>, Box<dyn IError>> {
        match (inner_type, &element.value) {
            (Type::Vector(nested_inner), Expression::Vector(nested_elements)) => {
                let nested_ptr = if nested_elements.is_empty() {
                    self.build_empty_vector(nested_inner, element.span)?
                } else {
                    self.build_vector_from_elements(nested_inner, nested_elements, None, element.span)?
                };
                Ok(LlvmValue::Vector(nested_ptr, nested_inner.clone()))
            }
            _ => {
                self.visit_expression(element)?;
                self.read_last_value()
            }
        }
    }

    pub(in crate::backend::llvm::compiler) fn build_vector_expression(
        &mut self,
        elements: &'a [Box<Node<Expression>>],
        span: Span,
    ) -> Result<(PointerValue<'ctx>, Type), Box<dyn IError>> {
        if elements.is_empty() {
            return Err(Box::new(CompilerError::at(
                ErrorSeverity::HIGH,
                String::from("Cannot infer the element type of an empty vector literal in this context; declare it with an explicit type instead."),
                span,
            )) as Box<dyn IError>);
        }

        self.visit_expression(&elements[0])?;
        let first_value = self.read_last_value()?;
        let inner_type = first_value.to_type();

        let vector_ptr = self.build_vector_from_elements(&inner_type, elements, Some(first_value), span)?;

        Ok((vector_ptr, inner_type))
    }

    /// Index into a `VecHeader` or `StrHeader` and locate the element's
    /// storage location, resolving multi-level accessors (`a[0].b[1]`).
    /// Does **not** release/retain anything - callers decide based on
    /// whether they're reading (retain the loaded value) or overwriting
    /// (release the old value first).
    pub(in crate::backend::llvm::compiler) fn resolve_indexed_element(
        &mut self,
        mut current_ptr: PointerValue<'ctx>,
        current_type: &Type,
        accessors: &'a [Node<Accessor>],
        span: Span,
    ) -> Result<(PointerValue<'ctx>, Type), Box<dyn IError>> {
        if accessors.is_empty() {
            return Err(Box::new(CompilerError::at(
                ErrorSeverity::HIGH,
                String::from("Accessing an element requires at least one accessor."),
                span,
            )) as Box<dyn IError>);
        }

        let err = Self::builder_err(span);
        let struct_type = LlvmValue::vector_struct_type(self.context);
        let ptr_type = self.context.ptr_type(AddressSpace::default());
        let i8_type = self.context.i8_type();

        let mut current_element_type = current_type.clone();

        for (i, accessor) in accessors.iter().enumerate() {
            let is_last = i == accessors.len() - 1;

            match &accessor.value {
                Accessor::Index(index_expr) => match &current_element_type {
                    Type::Vector(inner) => {
                        let inner_type = (**inner).clone();

                        let data_field = self
                            .builder
                            .build_struct_gep(struct_type, current_ptr, VEC_DATA, "idx.data")
                            .map_err(&err)?;
                        let length_field = self
                            .builder
                            .build_struct_gep(struct_type, current_ptr, VEC_LENGTH, "idx.length")
                            .map_err(&err)?;

                        let data = self
                            .builder
                            .build_load(ptr_type, data_field, "idx.data.val")
                            .map_err(&err)?
                            .into_pointer_value();

                        let i64_type = self.context.i64_type();

                        let length = self
                            .builder
                            .build_load(i64_type, length_field, "idx.length.val")
                            .map_err(&err)?
                            .into_int_value();

                        self.visit_expression(index_expr)?;

                        let index_value = self.read_last_value()?.into_i64_value(index_expr.span)?;

                        self.emit_bounds_check(&self.builder, &self.libc, self.context, index_value, length, index_expr.span)?;

                        let element_llvm_type = LlvmValue::type_to_basic_type_enum(&inner_type, self.context).ok_or_else(|| {
                            Box::new(CompilerError::at(
                                ErrorSeverity::HIGH,
                                format!("Compiling vectors of type '{}' is not yet supported. 5", inner_type),
                                index_expr.span,
                            )) as Box<dyn IError>
                        })?;

                        let element_ptr = unsafe {
                            self.builder
                                .build_gep(element_llvm_type, data, &[index_value], "idx.elem")
                                .map_err(&err)?
                        };

                        if is_last {
                            return Ok((element_ptr, inner_type));
                        }

                        current_ptr = self
                            .builder
                            .build_load(ptr_type, element_ptr, "idx.next")
                            .map_err(&err)?
                            .into_pointer_value();

                        current_element_type = inner_type;
                    }

                    Type::Str => {
                        self.visit_expression(index_expr)?;

                        let index_value = self.read_last_value()?.into_i64_value(index_expr.span)?;

                        let data = self.str_data_ptr(current_ptr, index_expr.span)?;

                        let length = self
                            .builder
                            .build_call(self.libc.strlen_fn, &[data.into()], "idx.str.len")
                            .map_err(&err)?
                            .try_as_basic_value()
                            .basic()
                            .expect("strlen should return a value")
                            .into_int_value();

                        self.emit_bounds_check(&self.builder, &self.libc, self.context, index_value, length, index_expr.span)?;

                        let element_ptr = unsafe { self.builder.build_gep(i8_type, data, &[index_value], "idx.str.elem").map_err(&err)? };

                        if is_last {
                            return Ok((element_ptr, Type::Char));
                        }

                        return Err(Box::new(CompilerError::at(
                            ErrorSeverity::HIGH,
                            String::from("Cannot index into type 'Char'."),
                            index_expr.span,
                        )) as Box<dyn IError>);
                    }

                    other => {
                        return Err(Box::new(CompilerError::at(
                            ErrorSeverity::HIGH,
                            format!("Cannot index into type '{}'.", other),
                            index_expr.span,
                        )) as Box<dyn IError>);
                    }
                },

                Accessor::Field(field_name_node) => {
                    let field_name = &field_name_node.value;

                    let Type::Struct { identifier, fields } = &current_element_type else {
                        return Err(Box::new(CompilerError::at(
                            ErrorSeverity::HIGH,
                            format!("Cannot access field '{}' on type '{}'.", field_name, current_element_type),
                            field_name_node.span,
                        )) as Box<dyn IError>);
                    };

                    let (struct_type, field_indices) = self.struct_llvm_type(identifier, field_name_node.span)?;

                    let field_index = *field_indices.get(field_name.as_str()).ok_or_else(|| {
                        Box::new(CompilerError::at(
                            ErrorSeverity::HIGH,
                            format!("Struct '{}' has no field '{}'.", identifier, field_name),
                            field_name_node.span,
                        )) as Box<dyn IError>
                    })?;

                    let declared_field_type = fields.get(field_name.as_str()).ok_or_else(|| {
                        Box::new(CompilerError::at(
                            ErrorSeverity::HIGH,
                            format!("Struct '{}' has no field '{}'.", identifier, field_name),
                            field_name_node.span,
                        )) as Box<dyn IError>
                    })?;

                    let field_type = self.resolve_type(declared_field_type);

                    let field_ptr = self
                        .builder
                        .build_struct_gep(struct_type, current_ptr, field_index, "field.access")
                        .map_err(&err)?;

                    if is_last {
                        return Ok((field_ptr, field_type));
                    }

                    match &field_type {
                        Type::Vector(_) | Type::Struct { .. } | Type::Str => {
                            let field_llvm_type = LlvmValue::type_to_basic_type_enum(&field_type, self.context)
                                .expect("Vector, Struct and Str always map to a pointer type");

                            current_ptr = self
                                .builder
                                .build_load(field_llvm_type, field_ptr, "field.next")
                                .map_err(&err)?
                                .into_pointer_value();

                            current_element_type = field_type;
                        }

                        other => {
                            return Err(Box::new(CompilerError::at(
                                ErrorSeverity::HIGH,
                                format!("Cannot access further into type '{}'.", other),
                                field_name_node.span,
                            )) as Box<dyn IError>);
                        }
                    }
                }
            }
        }

        Err(Box::new(CompilerError::at(
            ErrorSeverity::HIGH,
            String::from("Accessing an element requires at least one accessor."),
            span,
        )) as Box<dyn IError>)
    }

    pub(in crate::backend::llvm::compiler) fn build_default_value(
        &mut self,
        ptr: PointerValue<'ctx>,
        var_type: &Type,
        span: Span,
    ) -> Result<(), Box<dyn IError>> {
        let err = Self::builder_err(span);

        match var_type {
            Type::I64 => {
                let zero = self.context.i64_type().const_int(0, true);
                self.builder.build_store(ptr, zero).map_err(&err)?;
            }

            Type::F64 => {
                let zero = self.context.f64_type().const_float(0.0);
                self.builder.build_store(ptr, zero).map_err(&err)?;
            }

            Type::Bool => {
                let zero = self.context.bool_type().const_int(0, false);
                self.builder.build_store(ptr, zero).map_err(&err)?;
            }

            Type::Str => {
                let empty = self.build_empty_heap_string(span)?;
                self.builder.build_store(ptr, empty).map_err(&err)?;
            }

            Type::Vector(inner) => {
                let vector_ptr = self.build_empty_vector(inner, span)?;
                self.builder.build_store(ptr, vector_ptr).map_err(&err)?;
            }

            other => {
                return Err(Box::new(CompilerError::at(
                    ErrorSeverity::HIGH,
                    format!("Compiling default values of type '{}' is not yet supported.", other),
                    span,
                )) as Box<dyn IError>)
            }
        }

        Ok(())
    }

    /// Allocates a fresh, empty (but valid, refcounted) `StrHeader`.
    fn build_empty_heap_string(&mut self, span: Span) -> Result<PointerValue<'ctx>, Box<dyn IError>> {
        let err = Self::builder_err(span);

        let one = self.context.i64_type().const_int(1, false);

        let data_ptr = self
            .builder
            .build_call(self.libc.malloc_fn, &[one.into()], "str.default.malloc")
            .map_err(&err)?
            .try_as_basic_value()
            .basic()
            .expect("malloc should return a value")
            .into_pointer_value();

        let zero_byte = self.context.i8_type().const_int(0, false);

        self.builder.build_store(data_ptr, zero_byte).map_err(&err)?;

        self.build_str_header(data_ptr, span)
    }

    #[allow(dead_code)]
    pub(in crate::backend::llvm::compiler) fn build_shallow_copy_vector(
        &mut self,
        vector_ptr: PointerValue<'ctx>,
        inner_type: &Type,
        span: Span,
    ) -> Result<PointerValue<'ctx>, Box<dyn IError>> {
        let err = Self::builder_err(span);
        let function = self.current_function();

        let vector_type = LlvmValue::vector_struct_type(self.context);
        let ptr_type = self.context.ptr_type(AddressSpace::default());
        let i64_type = self.context.i64_type();

        // ---------------------------------------------------------------------
        // Load source header fields.
        // ---------------------------------------------------------------------

        let src_data_field = self
            .builder
            .build_struct_gep(vector_type, vector_ptr, VEC_DATA, "copy.src.data")
            .map_err(&err)?;

        let src_length_field = self
            .builder
            .build_struct_gep(vector_type, vector_ptr, VEC_LENGTH, "copy.src.length")
            .map_err(&err)?;

        let src_capacity_field = self
            .builder
            .build_struct_gep(vector_type, vector_ptr, VEC_CAPACITY, "copy.src.capacity")
            .map_err(&err)?;

        let old_data = self
            .builder
            .build_load(ptr_type, src_data_field, "copy.data.old")
            .map_err(&err)?
            .into_pointer_value();

        let old_length = self
            .builder
            .build_load(i64_type, src_length_field, "copy.length.old")
            .map_err(&err)?
            .into_int_value();

        let old_capacity = self
            .builder
            .build_load(i64_type, src_capacity_field, "copy.capacity.old")
            .map_err(&err)?
            .into_int_value();

        // ---------------------------------------------------------------------
        // Allocate/copy data array.
        // ---------------------------------------------------------------------

        let element_size = LlvmValue::element_byte_size(inner_type, i64_type, span)?;

        let bytes = self.builder.build_int_mul(old_length, element_size, "copy.bytes").map_err(&err)?;

        let new_data_alloca = self.builder.build_alloca(ptr_type, "copy.data.slot").map_err(&err)?;

        self.builder.build_store(new_data_alloca, ptr_type.const_null()).map_err(&err)?;

        let is_empty = self
            .builder
            .build_int_compare(IntPredicate::EQ, old_length, i64_type.const_zero(), "copy.is_empty")
            .map_err(&err)?;

        let copy_block = self.context.append_basic_block(function, "copy.data");

        let retain_block = self.context.append_basic_block(function, "copy.retain.elements");

        let merge_block = self.context.append_basic_block(function, "copy.merge");

        self.builder.build_conditional_branch(is_empty, merge_block, copy_block).map_err(&err)?;

        // ---------------------------------------------------------------------
        // memcpy source array -> new array
        // ---------------------------------------------------------------------

        self.builder.position_at_end(copy_block);

        let new_data = self
            .builder
            .build_call(self.libc.malloc_fn, &[bytes.into()], "copy.data.malloc")
            .map_err(&err)?
            .try_as_basic_value()
            .basic()
            .expect("malloc should return a value")
            .into_pointer_value();

        self.builder
            .build_call(self.libc.memcpy_fn, &[new_data.into(), old_data.into(), bytes.into()], "copy.memcpy")
            .map_err(&err)?;

        self.builder.build_store(new_data_alloca, new_data).map_err(&err)?;

        self.builder.build_unconditional_branch(retain_block).map_err(&err)?;

        // ---------------------------------------------------------------------
        // IMPORTANT:
        //
        // memcpy copied the POINTER VALUES, not ownership.
        //
        // Therefore every heap-owned element needs +1 refcount.
        // ---------------------------------------------------------------------

        self.builder.position_at_end(retain_block);

        let index_alloca = self.builder.build_alloca(i64_type, "copy.retain.index").map_err(&err)?;

        self.builder.build_store(index_alloca, i64_type.const_zero()).map_err(&err)?;

        let retain_cond = self.context.append_basic_block(function, "copy.retain.cond");

        let retain_body = self.context.append_basic_block(function, "copy.retain.body");

        let retain_done = self.context.append_basic_block(function, "copy.retain.done");

        self.builder.build_unconditional_branch(retain_cond).map_err(&err)?;

        // condition
        self.builder.position_at_end(retain_cond);

        let index = self
            .builder
            .build_load(i64_type, index_alloca, "copy.retain.index.load")
            .map_err(&err)?
            .into_int_value();

        let has_next = self
            .builder
            .build_int_compare(IntPredicate::ULT, index, old_length, "copy.retain.has_next")
            .map_err(&err)?;

        self.builder.build_conditional_branch(has_next, retain_body, retain_done).map_err(&err)?;

        // body
        self.builder.position_at_end(retain_body);

        let new_data_ptr = self
            .builder
            .build_load(ptr_type, new_data_alloca, "copy.retain.data")
            .map_err(&err)?
            .into_pointer_value();

        let element_ptr = unsafe {
            self.builder
                .build_gep(ptr_type, new_data_ptr, &[index], "copy.retain.element.ptr")
                .map_err(&err)?
        };

        let element_llvm_type =
            LlvmValue::type_to_basic_type_enum(inner_type, self.context).expect("vector element type should be representable by LLVM");

        let element_raw = self
            .builder
            .build_load(element_llvm_type, element_ptr, "copy.retain.element")
            .map_err(&err)?;

        if matches!(inner_type, Type::Str | Type::Vector(_) | Type::Struct { .. }) {
            let element_value = LlvmValue::from_basic_value_enum(element_raw, inner_type);

            self.retain_value(&element_value, span)?;
        }

        let next_index = self
            .builder
            .build_int_add(index, i64_type.const_int(1, false), "copy.retain.next")
            .map_err(&err)?;

        self.builder.build_store(index_alloca, next_index).map_err(&err)?;

        self.builder.build_unconditional_branch(retain_cond).map_err(&err)?;

        // done
        self.builder.position_at_end(retain_done);

        self.builder.build_unconditional_branch(merge_block).map_err(&err)?;

        // ---------------------------------------------------------------------
        // Build new vector header.
        // ---------------------------------------------------------------------

        self.builder.position_at_end(merge_block);

        let final_data = self
            .builder
            .build_load(ptr_type, new_data_alloca, "copy.data.final")
            .map_err(&err)?
            .into_pointer_value();

        let struct_size = vector_type.size_of().expect("vector type should be sized");

        let new_struct_ptr = self
            .builder
            .build_call(self.libc.malloc_fn, &[struct_size.into()], "copy.header.malloc")
            .map_err(&err)?
            .try_as_basic_value()
            .basic()
            .expect("malloc should return a value")
            .into_pointer_value();

        // refcount = 1
        let refcount_field = self
            .builder
            .build_struct_gep(vector_type, new_struct_ptr, VEC_REFCOUNT, "copy.dst.refcount")
            .map_err(&err)?;

        self.builder.build_store(refcount_field, i64_type.const_int(1, false)).map_err(&err)?;

        // data
        let new_data_field = self
            .builder
            .build_struct_gep(vector_type, new_struct_ptr, VEC_DATA, "copy.dst.data")
            .map_err(&err)?;

        self.builder.build_store(new_data_field, final_data).map_err(&err)?;

        // length
        let new_length_field = self
            .builder
            .build_struct_gep(vector_type, new_struct_ptr, VEC_LENGTH, "copy.dst.length")
            .map_err(&err)?;

        self.builder.build_store(new_length_field, old_length).map_err(&err)?;

        // capacity
        let new_capacity_field = self
            .builder
            .build_struct_gep(vector_type, new_struct_ptr, VEC_CAPACITY, "copy.dst.capacity")
            .map_err(&err)?;

        self.builder.build_store(new_capacity_field, old_capacity).map_err(&err)?;

        Ok(new_struct_ptr)
    }
}
