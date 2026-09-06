use inkwell::builder::Builder;
use inkwell::context::Context;
use inkwell::values::IntValue;
use inkwell::{AddressSpace, IntPredicate};

use super::Compiler;
use crate::common::types::Type;
use crate::common::visitor::Visitor;
use crate::{
    backend::llvm::{
        libc_functions::LibcFunctions,
        llvm_alu::{
            llvm_value::{LlvmValue, VEC_DATA, VEC_LENGTH},
            LlvmAlu,
        },
    },
    common::{
        errors::{CompilerError, ErrorSeverity, IError},
        span::Span,
    },
    frontend::ast::{Expression, Node},
};

impl<'a, 'ctx> Compiler<'a, 'ctx> {
    pub(in crate::backend::llvm::compiler) fn build_binary_op<F>(
        &mut self,
        lhs: &'a Node<Expression>,
        rhs: &'a Node<Expression>,
        op: F,
        span: Span,
    ) -> Result<(), Box<dyn IError>>
    where
        F: Fn(&LlvmAlu, &Builder<'ctx>, &LibcFunctions<'ctx>, LlvmValue<'ctx>, LlvmValue<'ctx>, Span) -> Result<LlvmValue<'ctx>, Box<dyn IError>>,
    {
        self.visit_expression(lhs)?;
        let left_value = self.read_last_value()?;
        self.visit_expression(rhs)?;
        let right_value = self.read_last_value()?;
        let value = op(&self.llvm_alu, &self.builder, &self.libc, left_value, right_value, span)?;
        self.last_value = Some(value);
        Ok(())
    }

    pub(in crate::backend::llvm::compiler) fn build_unary_op<F>(
        &mut self,
        value: &'a Node<Expression>,
        op: F,
        span: Span,
    ) -> Result<(), Box<dyn IError>>
    where
        F: Fn(&LlvmAlu, &Builder<'ctx>, &LibcFunctions<'ctx>, LlvmValue<'ctx>, Span) -> Result<LlvmValue<'ctx>, Box<dyn IError>>,
    {
        self.visit_expression(value)?;
        let computed_value = self.read_last_value()?;
        let value = op(&self.llvm_alu, &self.builder, &self.libc, computed_value, span)?;
        self.last_value = Some(value);
        Ok(())
    }

    pub(in crate::backend::llvm::compiler) fn compile_expression(&mut self, expression: &'a Node<Expression>) -> Result<(), Box<dyn IError>> {
        let span = expression.span;
        match &expression.value {
            Expression::FunctionCall { identifier, arguments } => {
                let name = identifier.value.as_str();
                if let Some(std_function) = self.program.std_functions.get(name) {
                    return (std_function.compile)(self, arguments, span);
                }
                self.build_function_call(identifier, arguments, span)
            }
            Expression::Literal(literal) => self.visit_literal(literal),
            Expression::Variable(variable) => self.visit_variable(variable, span),
            Expression::BooleanNegation(expr) => self.build_unary_op(expr, LlvmAlu::boolean_negate, span),
            Expression::ArithmeticNegation(expr) => self.build_unary_op(expr, LlvmAlu::arithmetic_negate, span),
            Expression::Addition(lhs, rhs) => {
                self.visit_expression(lhs)?;
                let left_value = self.read_last_value()?;
                self.visit_expression(rhs)?;
                let right_value = self.read_last_value()?;
                let value = self
                    .llvm_alu
                    .add(&self.builder, &self.libc, left_value.clone(), right_value.clone(), span)?;
                if let LlvmValue::Str(_) = left_value {
                    if Self::expr_needs_release(&lhs.as_ref().value) {
                        self.release_value(&left_value, lhs.as_ref().span)?;
                    }
                }
                if let LlvmValue::Str(_) = right_value {
                    if Self::expr_needs_release(&rhs.as_ref().value) {
                        self.release_value(&right_value, rhs.as_ref().span)?;
                    }
                }
                self.last_value = Some(value);
                Ok(())
            }
            Expression::Subtraction(lhs, rhs) => self.build_binary_op(lhs, rhs, LlvmAlu::subtract, span),
            Expression::Multiplication(lhs, rhs) => self.build_binary_op(lhs, rhs, LlvmAlu::multiplication, span),
            Expression::Division(lhs, rhs) => self.build_binary_op(lhs, rhs, LlvmAlu::division, span),
            Expression::Modulo(lhs, rhs) => self.build_binary_op(lhs, rhs, LlvmAlu::modulo, span),
            Expression::Concatenation(lhs, rhs) => self.build_binary_op(lhs, rhs, LlvmAlu::concatenation, span),
            Expression::Alternative(lhs, rhs) => self.build_binary_op(lhs, rhs, LlvmAlu::alternative, span),
            Expression::Greater(lhs, rhs) => self.build_binary_op(lhs, rhs, LlvmAlu::greater, span),
            Expression::GreaterEqual(lhs, rhs) => self.build_binary_op(lhs, rhs, LlvmAlu::greater_or_equal, span),
            Expression::Less(lhs, rhs) => self.build_binary_op(lhs, rhs, LlvmAlu::less, span),
            Expression::LessEqual(lhs, rhs) => self.build_binary_op(lhs, rhs, LlvmAlu::less_or_equal, span),
            Expression::Equal(lhs, rhs) => {
                self.visit_expression(lhs)?;
                let left_value = self.read_last_value()?;
                self.visit_expression(rhs)?;
                let right_value = self.read_last_value()?;
                let value = self
                    .llvm_alu
                    .equal(&self.builder, &self.libc, left_value.clone(), right_value.clone(), span)?;
                if let LlvmValue::Str(_) = left_value {
                    if Self::expr_needs_release(&lhs.as_ref().value) {
                        self.release_value(&left_value, lhs.as_ref().span)?;
                    }
                }
                if let LlvmValue::Str(_) = right_value {
                    if Self::expr_needs_release(&rhs.as_ref().value) {
                        self.release_value(&right_value, rhs.as_ref().span)?;
                    }
                }
                self.last_value = Some(value);
                Ok(())
            }
            Expression::NotEqual(lhs, rhs) => {
                self.visit_expression(lhs)?;
                let left_value = self.read_last_value()?;
                self.visit_expression(rhs)?;
                let right_value = self.read_last_value()?;
                let value = self
                    .llvm_alu
                    .not_equal(&self.builder, &self.libc, left_value.clone(), right_value.clone(), span)?;
                if let LlvmValue::Str(_) = left_value {
                    if Self::expr_needs_release(&lhs.as_ref().value) {
                        self.release_value(&left_value, lhs.as_ref().span)?;
                    }
                }
                if let LlvmValue::Str(_) = right_value {
                    if Self::expr_needs_release(&rhs.as_ref().value) {
                        self.release_value(&right_value, rhs.as_ref().span)?;
                    }
                }
                self.last_value = Some(value);
                Ok(())
            }
            Expression::Casting { value, to_type } => {
                self.visit_expression(value)?;
                let source_value = self.read_last_value()?;
                self.last_value = Some(
                    self.llvm_alu
                        .cast_to_type(&self.builder, &self.libc, source_value.clone(), &to_type.value, span)?,
                );
                if let LlvmValue::Str(_) = source_value {
                    if Self::expr_needs_release(&value.as_ref().value) {
                        self.release_value(&source_value, expression.span)?;
                    }
                }
                Ok(())
            }
            Expression::Index { collection, index } => {
                self.visit_expression(collection)?;
                let collection_value = self.read_last_value()?;
                match &collection_value {
                    LlvmValue::Vector(vector_ptr, _) => {
                        let collection_type = self.resolve_type(&collection_value.to_type());
                        let struct_type = LlvmValue::vector_struct_type(self.context);
                        let ptr_type = self.context.ptr_type(AddressSpace::default());
                        let i64_type = self.context.i64_type();
                        let inner_type = match &collection_type {
                            Type::Vector(inner) => self.resolve_type(inner),
                            other => {
                                return Err(Box::new(CompilerError::at(
                                    ErrorSeverity::HIGH,
                                    format!("Cannot index into type '{}'.", other),
                                    span,
                                )));
                            }
                        };

                        let data_field = self
                            .builder
                            .build_struct_gep(struct_type, *vector_ptr, VEC_DATA, "idx.data")
                            .map_err(|err| Box::new(CompilerError::at(ErrorSeverity::HIGH, err.to_string(), span)) as Box<dyn IError>)?;
                        let length_field = self
                            .builder
                            .build_struct_gep(struct_type, *vector_ptr, VEC_LENGTH, "idx.length")
                            .map_err(|err| Box::new(CompilerError::at(ErrorSeverity::HIGH, err.to_string(), span)) as Box<dyn IError>)?;

                        let data = self
                            .builder
                            .build_load(ptr_type, data_field, "idx.data.val")
                            .map_err(|err| Box::new(CompilerError::at(ErrorSeverity::HIGH, err.to_string(), span)) as Box<dyn IError>)?
                            .into_pointer_value();
                        let length = self
                            .builder
                            .build_load(i64_type, length_field, "idx.length.val")
                            .map_err(|err| Box::new(CompilerError::at(ErrorSeverity::HIGH, err.to_string(), span)) as Box<dyn IError>)?
                            .into_int_value();

                        self.visit_expression(index)?;
                        let index_value = self.read_last_value()?;
                        let index_int = index_value.into_i64_value(index.span)?;
                        self.emit_bounds_check(&self.builder, &self.libc, self.context, index_int, length, index.span)?;

                        let element_llvm_type = LlvmValue::type_to_basic_type_enum(&inner_type, self.context).ok_or_else(|| {
                            Box::new(CompilerError::at(
                                ErrorSeverity::HIGH,
                                format!("Compiling vectors of type '{}' is not yet supported.", inner_type),
                                index.span,
                            )) as Box<dyn IError>
                        })?;

                        let element_ptr = unsafe {
                            self.builder
                                .build_gep(element_llvm_type, data, &[index_int], "idx.elem")
                                .map_err(|err| Box::new(CompilerError::at(ErrorSeverity::HIGH, err.to_string(), span)) as Box<dyn IError>)?
                        };

                        let raw_value = self
                            .builder
                            .build_load(element_llvm_type, element_ptr, "idx.load")
                            .map_err(|err| Box::new(CompilerError::at(ErrorSeverity::HIGH, err.to_string(), span)) as Box<dyn IError>)?;
                        let element_value = LlvmValue::from_basic_value_enum(raw_value, &inner_type);

                        let element_value = match element_value {
                            LlvmValue::Str(ptr) => LlvmValue::Str(self.build_string_copy(ptr, span)?),
                            other => {
                                self.retain_value(&other, span)?;
                                other
                            }
                        };

                        if Self::expr_needs_release(&collection.value) {
                            self.release_value(&collection_value, span)?;
                        }

                        self.last_value = Some(element_value);
                        Ok(())
                    }
                    LlvmValue::Str(str_header) => {
                        self.visit_expression(index)?;
                        let index_value = self.read_last_value()?;
                        let index_int = index_value.into_i64_value(index.span)?;
                        let i8_type = self.context.i8_type();
                        let str_ptr = *str_header;
                        let data = self.str_data_ptr(str_ptr, span)?;
                        let length = self
                            .builder
                            .build_call(self.libc.strlen_fn, &[data.into()], "str.idx.len")
                            .map_err(|err| Box::new(CompilerError::at(ErrorSeverity::HIGH, err.to_string(), span)) as Box<dyn IError>)?
                            .try_as_basic_value()
                            .basic()
                            .expect("strlen should return a value")
                            .into_int_value();
                        self.emit_bounds_check(&self.builder, &self.libc, self.context, index_int, length, index.span)?;

                        let element_ptr = unsafe {
                            self.builder
                                .build_gep(i8_type, data, &[index_int], "str.idx.ptr")
                                .map_err(|err| Box::new(CompilerError::at(ErrorSeverity::HIGH, err.to_string(), span)) as Box<dyn IError>)?
                        };

                        let raw_value = self
                            .builder
                            .build_load(i8_type, element_ptr, "str.idx.load")
                            .map_err(|err| Box::new(CompilerError::at(ErrorSeverity::HIGH, err.to_string(), span)) as Box<dyn IError>)?;

                        if Self::expr_needs_release(&collection.value) {
                            self.release_value(&collection_value, span)?;
                        }

                        self.last_value = Some(LlvmValue::from_basic_value_enum(raw_value, &Type::Char));
                        Ok(())
                    }
                    other => Err(Box::new(CompilerError::at(
                        ErrorSeverity::HIGH,
                        format!("Cannot index into type '{}'.", other.to_type()),
                        span,
                    )) as Box<dyn IError>),
                }
            }
            Expression::Vector(elements) => {
                let (vector_ptr, inner_type) = self.build_vector_expression(elements, span)?;
                self.last_value = Some(LlvmValue::Vector(vector_ptr, Box::new(inner_type)));
                Ok(())
            }
            Expression::StructLiteral(node) => {
                let identifier = &node.value.identifier;
                let fields = &node.value.fields;
                let declared_type = self.program.types.get(&identifier.value).cloned().ok_or_else(|| {
                    Box::new(CompilerError::at(
                        ErrorSeverity::HIGH,
                        format!("Unknown type '{}'.", identifier.value),
                        span,
                    )) as Box<dyn IError>
                })?;
                let Type::Struct { fields: field_types, .. } = &declared_type else {
                    return Err(Box::new(CompilerError::at(
                        ErrorSeverity::HIGH,
                        format!("'{}' is not a struct type.", identifier.value),
                        span,
                    )));
                };
                let (struct_type, field_indices) = self.struct_llvm_type(&identifier.value, span)?;
                let size = struct_type.size_of().expect("struct type should be sized");
                let struct_ptr = self
                    .builder
                    .build_call(self.libc.malloc_fn, &[size.into()], "struct.malloc")
                    .map_err(|err| Box::new(CompilerError::at(ErrorSeverity::HIGH, err.to_string(), span)) as Box<dyn IError>)?
                    .try_as_basic_value()
                    .basic()
                    .expect("malloc should return a value")
                    .into_pointer_value();
                for field in fields {
                    let field_name = field.value.identifier.value.as_str();
                    let field_index = *field_indices.get(field_name).ok_or_else(|| {
                        Box::new(CompilerError::at(
                            ErrorSeverity::HIGH,
                            format!("Struct '{}' has no field '{}'.", identifier.value, field_name),
                            field.span,
                        )) as Box<dyn IError>
                    })?;
                    let is_empty_vector = matches!(
                        &field.value.value.value,
                        Expression::Vector(elements) if elements.is_empty()
                    );
                    let field_value = if is_empty_vector {
                        let expected_field_type = field_types.get(field_name).ok_or_else(|| {
                            Box::new(CompilerError::at(
                                ErrorSeverity::HIGH,
                                format!("Struct '{}' has no field '{}'.", identifier.value, field_name),
                                field.span,
                            )) as Box<dyn IError>
                        })?;
                        let resolved_field_type = self.resolve_type(expected_field_type);
                        let Type::Vector(inner) = &resolved_field_type else {
                            return Err(Box::new(CompilerError::expected_found(
                                ErrorSeverity::HIGH,
                                format!("Cannot assign value to field '{}'.", field_name),
                                format!("{}", resolved_field_type),
                                "empty vector".to_string(),
                                field.span,
                            )));
                        };
                        let vector_ptr = self.build_empty_vector(inner, field.span)?;
                        LlvmValue::Vector(vector_ptr, inner.clone())
                    } else {
                        self.visit_expression(&field.value.value)?;
                        let value = self.read_last_value()?;
                        self.finalize_owned_value_for_new_slot(value, &field.value.value.value, field.span)?
                    };
                    let field_ptr = self
                        .builder
                        .build_struct_gep(struct_type, struct_ptr, field_index, "field.init")
                        .map_err(|err| Box::new(CompilerError::at(ErrorSeverity::HIGH, err.to_string(), field.span)) as Box<dyn IError>)?;
                    self.builder
                        .build_store(field_ptr, field_value.as_basic_value_enum())
                        .map_err(|err| Box::new(CompilerError::at(ErrorSeverity::HIGH, err.to_string(), field.span)) as Box<dyn IError>)?;
                }
                let refcount_index = Self::struct_refcount_field_index(struct_type);
                let refcount_field = self
                    .builder
                    .build_struct_gep(struct_type, struct_ptr, refcount_index, "struct.refcount")
                    .map_err(|err| Box::new(CompilerError::at(ErrorSeverity::HIGH, err.to_string(), span)) as Box<dyn IError>)?;
                self.builder
                    .build_store(refcount_field, self.context.i64_type().const_int(1, false))
                    .map_err(|err| Box::new(CompilerError::at(ErrorSeverity::HIGH, err.to_string(), span)) as Box<dyn IError>)?;
                self.last_value = Some(LlvmValue::Struct(struct_ptr, Box::new(declared_type.clone())));
                Ok(())
            }
            Expression::FieldAccess { instance, field } => {
                self.visit_expression(instance)?;
                let instance_value = self.read_last_value()?;

                let (struct_ptr, struct_type_info) = match &instance_value {
                    LlvmValue::Struct(ptr, ty) => (*ptr, ty.clone()),
                    other => {
                        return Err(Box::new(CompilerError::at(
                            ErrorSeverity::HIGH,
                            format!("Cannot access field '{}' on type '{}'.", field.value, other.to_type()),
                            span,
                        )));
                    }
                };

                // Zwolnij temporary instance (po wyciągnięciu potrzebnych danych)
                if Self::expr_needs_release(&instance.value) {
                    self.release_value(&instance_value, span)?;
                }

                let Type::Struct { identifier, fields } = struct_type_info.as_ref() else {
                    return Err(Box::new(CompilerError::at(
                        ErrorSeverity::HIGH,
                        format!("Cannot access field '{}' on a non-struct type.", field.value),
                        span,
                    )));
                };
                let field_type = fields.get(&field.value).cloned().ok_or_else(|| {
                    Box::new(CompilerError::at(
                        ErrorSeverity::HIGH,
                        format!("Struct '{}' has no field '{}'.", identifier, field.value),
                        field.span,
                    )) as Box<dyn IError>
                })?;
                let resolved_field_type = self.resolve_type(&field_type);
                let (struct_type, field_indices) = self.struct_llvm_type(identifier, span)?;
                let field_index = *field_indices.get(&field.value).ok_or_else(|| {
                    Box::new(CompilerError::at(
                        ErrorSeverity::HIGH,
                        format!("Struct '{}' has no field '{}'.", identifier, field.value),
                        field.span,
                    )) as Box<dyn IError>
                })?;
                let field_ptr = self
                    .builder
                    .build_struct_gep(struct_type, struct_ptr, field_index, "field.access")
                    .map_err(|err| Box::new(CompilerError::at(ErrorSeverity::HIGH, err.to_string(), field.span)) as Box<dyn IError>)?;
                let field_llvm_type = LlvmValue::type_to_basic_type_enum(&resolved_field_type, self.context).ok_or_else(|| {
                    Box::new(CompilerError::at(
                        ErrorSeverity::HIGH,
                        format!("Compiling fields of type '{}' is not yet supported.", resolved_field_type),
                        field.span,
                    )) as Box<dyn IError>
                })?;
                let raw_value = self
                    .builder
                    .build_load(field_llvm_type, field_ptr, "field.load")
                    .map_err(|err| Box::new(CompilerError::at(ErrorSeverity::HIGH, err.to_string(), field.span)) as Box<dyn IError>)?;
                let field_value = LlvmValue::from_basic_value_enum(raw_value, &resolved_field_type);

                let field_value = match field_value {
                    LlvmValue::Str(ptr) => LlvmValue::Str(self.build_string_copy(ptr, field.span)?),
                    other => {
                        self.retain_value(&other, field.span)?;
                        other
                    }
                };
                self.last_value = Some(field_value);
                Ok(())
            }
        }
    }

    pub(in crate::backend::llvm::compiler) fn emit_bounds_check(
        &self,
        builder: &Builder<'ctx>,
        libc: &LibcFunctions<'ctx>,
        context: &'ctx Context,
        index: IntValue<'ctx>,
        length: IntValue<'ctx>,
        span: Span,
    ) -> Result<(), Box<dyn IError>> {
        let i64_type = context.i64_type();
        let zero = i64_type.const_int(0, false);
        let lt_zero = builder
            .build_int_compare(IntPredicate::SLT, index, zero, "bounds.lt_zero")
            .map_err(|err| Box::new(CompilerError::at(ErrorSeverity::HIGH, err.to_string(), span)) as Box<dyn IError>)?;
        let ge_length = builder
            .build_int_compare(IntPredicate::SGE, index, length, "bounds.ge_length")
            .map_err(|err| Box::new(CompilerError::at(ErrorSeverity::HIGH, err.to_string(), span)) as Box<dyn IError>)?;
        let out_of_bounds = builder
            .build_or(lt_zero, ge_length, "bounds.out_of_range")
            .map_err(|err| Box::new(CompilerError::at(ErrorSeverity::HIGH, err.to_string(), span)) as Box<dyn IError>)?;
        let function = builder
            .get_insert_block()
            .expect("builder should be positioned inside a function")
            .get_parent()
            .expect("basic block should belong to a function");
        let error_block = context.append_basic_block(function, "bounds.error");
        let merge_block = context.append_basic_block(function, "bounds.continue");
        builder
            .build_conditional_branch(out_of_bounds, error_block, merge_block)
            .map_err(|err| Box::new(CompilerError::at(ErrorSeverity::HIGH, err.to_string(), span)) as Box<dyn IError>)?;
        builder.position_at_end(error_block);
        let error = CompilerError::at(ErrorSeverity::HIGH, String::from("Index out of bounds."), span);
        let message = format!("{}\n", error.get_stderr_message());
        let format_str = builder
            .build_global_string_ptr(&message, "bounds.msg")
            .map_err(|err| Box::new(CompilerError::at(ErrorSeverity::HIGH, err.to_string(), span)) as Box<dyn IError>)?;
        let stderr = builder
            .build_load(context.ptr_type(AddressSpace::default()), libc.stderr.as_pointer_value(), "stderr")
            .map_err(|err| Box::new(CompilerError::at(ErrorSeverity::HIGH, err.to_string(), span)) as Box<dyn IError>)?;
        builder
            .build_call(libc.fprintf_fn, &[stderr.into(), format_str.as_pointer_value().into()], "bounds.fprintf")
            .map_err(|err| Box::new(CompilerError::at(ErrorSeverity::HIGH, err.to_string(), span)) as Box<dyn IError>)?;
        let i32_type = context.i32_type();
        builder
            .build_call(libc.exit_fn, &[i32_type.const_int(1, false).into()], "bounds.exit")
            .map_err(|err| Box::new(CompilerError::at(ErrorSeverity::HIGH, err.to_string(), span)) as Box<dyn IError>)?;
        builder
            .build_unreachable()
            .map_err(|err| Box::new(CompilerError::at(ErrorSeverity::HIGH, err.to_string(), span)) as Box<dyn IError>)?;
        builder.position_at_end(merge_block);
        Ok(())
    }
}
