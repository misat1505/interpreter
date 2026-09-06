use inkwell::AddressSpace;

use super::{Compiler, ControlFrame};
use crate::common::visitor::Visitor;
use crate::frontend::ast::VariableDeclarationKind;
use crate::{
    backend::llvm::llvm_alu::llvm_value::LlvmValue,
    common::{
        errors::{CompilerError, ErrorSeverity, IError},
        span::Span,
        types::Type,
    },
    frontend::ast::{Expression, Node, Statement},
};

impl<'a, 'ctx> Compiler<'a, 'ctx> {
    pub(in crate::backend::llvm::compiler) fn compile_statement(&mut self, statement: &'a Node<Statement>) -> Result<(), Box<dyn IError>> {
        let span = statement.span;

        match &statement.value {
            Statement::Import { .. } => unreachable!("Imports have to be resolved before compilation"),
            Statement::FunctionCall { identifier, arguments } => {
                let name = identifier.value.as_str();

                if let Some(std_function) = self.program.std_functions.get(name) {
                    (std_function.compile)(self, arguments, span)?;
                } else {
                    self.build_function_call(identifier, arguments, span)?;
                }

                // Used as a bare statement: the return value (if any) is
                // discarded. If it's an owned heap value nobody stored it
                // anywhere, so it must be released here or it leaks.
                if let Some(value) = self.last_value.take() {
                    self.release_value(&value, span)?;
                }

                Ok(())
            }

            Statement::Declaration { identifier, kind } => match kind {
                VariableDeclarationKind::TYPE { var_type, value } => {
                    let llvm_type = LlvmValue::type_to_basic_type_enum(&var_type.value, self.context).ok_or_else(|| {
                        Box::new(CompilerError::at(
                            ErrorSeverity::HIGH,
                            format!("Compiling declarations of type '{}' is not yet supported.", var_type.value),
                            span,
                        )) as Box<dyn IError>
                    })?;

                    let ptr = self
                        .builder
                        .build_alloca(llvm_type, identifier.value.as_str())
                        .map_err(|err| Box::new(CompilerError::at(ErrorSeverity::HIGH, err.to_string(), span)) as Box<dyn IError>)?;

                    match value {
                        Some(val_expr) => match (&var_type.value, &val_expr.value) {
                            (Type::Vector(inner), Expression::Vector(elements)) if elements.is_empty() => {
                                let vector_ptr = self.build_empty_vector(inner, span)?;

                                self.builder
                                    .build_store(ptr, vector_ptr)
                                    .map_err(|err| Box::new(CompilerError::at(ErrorSeverity::HIGH, err.to_string(), span)) as Box<dyn IError>)?;
                            }

                            (Type::Vector(inner), Expression::Vector(elements)) => {
                                let vector_ptr = if elements.is_empty() {
                                    self.build_empty_vector(inner, span)?
                                } else {
                                    self.build_vector_from_elements(inner, elements, None, span)?
                                };

                                self.builder
                                    .build_store(ptr, vector_ptr)
                                    .map_err(|err| Box::new(CompilerError::at(ErrorSeverity::HIGH, err.to_string(), span)) as Box<dyn IError>)?;
                            }

                            _ => {
                                self.visit_expression(val_expr)?;

                                let init_value = self.read_last_value()?;

                                let init_value = self.finalize_owned_value_for_new_slot(init_value, &val_expr.value, span)?;

                                self.builder
                                    .build_store(ptr, init_value.as_basic_value_enum())
                                    .map_err(|err| Box::new(CompilerError::at(ErrorSeverity::HIGH, err.to_string(), span)) as Box<dyn IError>)?;
                            }
                        },

                        None => {
                            self.build_default_value(ptr, &var_type.value, span)?;
                        }
                    }

                    self.declare_scoped_variable(identifier.value.clone(), ptr, var_type.value.clone());

                    Ok(())
                }

                VariableDeclarationKind::LET { var_type, value } => {
                    let is_empty_vector = matches!(
                        &value.value,
                        Expression::Vector(elements) if elements.is_empty()
                    );

                    let (final_type, init_value) = if is_empty_vector {
                        let Some(var_type) = var_type else {
                            return Err(Box::new(CompilerError::at(
                                ErrorSeverity::HIGH,
                                format!(
                                    "Cannot infer type of empty vector. Consider adding a type annotation, e.g. `let {}: {} = [];`.",
                                    identifier.value,
                                    Type::Vector(Box::new(Type::I64))
                                ),
                                span,
                            )));
                        };

                        let resolved_var_type = self.resolve_type(&var_type.value);

                        let Type::Vector(inner) = &resolved_var_type else {
                            return Err(Box::new(CompilerError::expected_found(
                                ErrorSeverity::HIGH,
                                format!("Cannot assign value to variable '{}'.", identifier.value),
                                format!("{}", resolved_var_type),
                                "empty vector".to_string(),
                                span,
                            )));
                        };

                        let llvm_type = LlvmValue::type_to_basic_type_enum(&resolved_var_type, self.context).ok_or_else(|| {
                            Box::new(CompilerError::at(
                                ErrorSeverity::HIGH,
                                format!("Compiling declarations of type '{}' is not yet supported.", resolved_var_type),
                                span,
                            )) as Box<dyn IError>
                        })?;

                        let ptr = self
                            .builder
                            .build_alloca(llvm_type, identifier.value.as_str())
                            .map_err(|err| Box::new(CompilerError::at(ErrorSeverity::HIGH, err.to_string(), span)) as Box<dyn IError>)?;

                        let vector_ptr = self.build_empty_vector(inner, span)?;

                        self.builder
                            .build_store(ptr, vector_ptr)
                            .map_err(|err| Box::new(CompilerError::at(ErrorSeverity::HIGH, err.to_string(), span)) as Box<dyn IError>)?;

                        self.declare_scoped_variable(identifier.value.clone(), ptr, resolved_var_type);

                        return Ok(());
                    } else {
                        self.visit_expression(value)?;

                        let init_value = self.read_last_value()?;
                        let resolved_type = self.resolve_type(&init_value.to_type());

                        let final_type = match var_type {
                            Some(var_type) => {
                                let resolved_var_type = self.resolve_type(&var_type.value);
                                if !resolved_var_type.is_compatible(&resolved_type) {
                                    return Err(Box::new(CompilerError::expected_found(
                                        ErrorSeverity::HIGH,
                                        format!("Cannot assign value to variable '{}'.", identifier.value),
                                        format!("{}", resolved_var_type),
                                        format!("{}", resolved_type),
                                        span,
                                    )));
                                }

                                resolved_var_type.clone()
                            }

                            None => {
                                if resolved_type == Type::Void {
                                    return Err(Box::new(CompilerError::at(
                                        ErrorSeverity::HIGH,
                                        format!("Cannot assign `void` to variable '{}'.", identifier.value),
                                        span,
                                    )));
                                }

                                resolved_type
                            }
                        };

                        let init_value = self.finalize_owned_value_for_new_slot(init_value, &value.value, span)?;

                        (final_type, init_value)
                    };

                    let llvm_type = LlvmValue::type_to_basic_type_enum(&final_type, self.context).ok_or_else(|| {
                        Box::new(CompilerError::at(
                            ErrorSeverity::HIGH,
                            format!("Compiling declarations of type '{}' is not yet supported.", final_type),
                            span,
                        )) as Box<dyn IError>
                    })?;

                    let ptr = self
                        .builder
                        .build_alloca(llvm_type, identifier.value.as_str())
                        .map_err(|err| Box::new(CompilerError::at(ErrorSeverity::HIGH, err.to_string(), span)) as Box<dyn IError>)?;

                    self.builder
                        .build_store(ptr, init_value.as_basic_value_enum())
                        .map_err(|err| Box::new(CompilerError::at(ErrorSeverity::HIGH, err.to_string(), span)) as Box<dyn IError>)?;

                    self.declare_scoped_variable(identifier.value.clone(), ptr, final_type);

                    Ok(())
                }
            },

            Statement::Assignment {
                identifier,
                value,
                accessors,
            } => {
                let (var_ptr, var_type) = self.get_variable(identifier.value.as_str())?;

                if accessors.is_empty() {
                    self.visit_expression(value)?;

                    let new_value = self.read_last_value()?;
                    let new_value = self.finalize_owned_value_for_new_slot(new_value, &value.value, span)?;

                    // The variable is about to be overwritten: release
                    // whatever it currently owns first.
                    self.release_current_value(var_ptr, &var_type, span)?;

                    self.builder
                        .build_store(var_ptr, new_value.as_basic_value_enum())
                        .map_err(|err| Box::new(CompilerError::at(ErrorSeverity::HIGH, err.to_string(), span)) as Box<dyn IError>)?;

                    return Ok(());
                }

                let ptr_type = self.context.ptr_type(AddressSpace::default());

                let vector_ptr = self
                    .builder
                    .build_load(ptr_type, var_ptr, "assign.vec")
                    .map_err(|err| Box::new(CompilerError::at(ErrorSeverity::HIGH, err.to_string(), span)) as Box<dyn IError>)?
                    .into_pointer_value();

                let (element_ptr, element_type) = self.resolve_indexed_element(vector_ptr, &var_type, accessors, span)?;

                // The slot being overwritten currently holds a live
                // reference (owned by the containing vector/struct) -
                // release it before storing the new value.
                self.release_current_value(element_ptr, &element_type, span)?;

                self.visit_expression(value)?;

                let new_value = self.read_last_value()?;

                if new_value.to_type() != element_type {
                    return Err(Box::new(CompilerError::at(
                        ErrorSeverity::HIGH,
                        format!(
                            "Type mismatch in indexed assignment: expected '{}', got '{}'.",
                            element_type,
                            new_value.to_type()
                        ),
                        span,
                    )));
                }

                let new_value = self.finalize_owned_value_for_new_slot(new_value, &value.value, span)?;

                self.builder
                    .build_store(element_ptr, new_value.as_basic_value_enum())
                    .map_err(|err| Box::new(CompilerError::at(ErrorSeverity::HIGH, err.to_string(), span)) as Box<dyn IError>)?;

                Ok(())
            }

            Statement::ForLoop {
                declaration,
                condition,
                assignment,
                block,
            } => {
                let function = self.current_function();

                // The loop's own declaration (if any) lives in a scope that
                // spans the whole loop; break/continue must release
                // everything opened from here on, down to and including
                // the per-iteration body scope.
                self.push_scope();

                if let Some(decl) = declaration {
                    self.visit_statement(decl)?;
                }

                let scope_depth = self.scopes.len();

                let cond_block = self.context.append_basic_block(function, "for.cond");

                let body_block = self.context.append_basic_block(function, "for.body");

                let continue_block = self.context.append_basic_block(function, "for.continue");

                let after_block = self.context.append_basic_block(function, "for.after");

                self.builder
                    .build_unconditional_branch(cond_block)
                    .map_err(|err| Box::new(CompilerError::at(ErrorSeverity::HIGH, err.to_string(), span)) as Box<dyn IError>)?;

                self.builder.position_at_end(cond_block);

                self.visit_expression(condition)?;

                let cond_value = self.read_last_value()?.into_int_value(span)?;

                self.builder
                    .build_conditional_branch(cond_value, body_block, after_block)
                    .map_err(|err| Box::new(CompilerError::at(ErrorSeverity::HIGH, err.to_string(), span)) as Box<dyn IError>)?;

                self.builder.position_at_end(body_block);

                self.control_stack.push(ControlFrame::Loop {
                    continue_block,
                    break_block: after_block,
                    scope_depth,
                });

                self.visit_block(block)?;

                self.control_stack.pop();

                self.branch_if_no_terminator(continue_block, span)?;

                self.builder.position_at_end(continue_block);

                if let Some(assign) = assignment {
                    self.visit_statement(assign)?;
                }

                self.branch_if_no_terminator(cond_block, span)?;

                self.builder.position_at_end(after_block);

                self.pop_scope_and_release(span)?;

                Ok(())
            }

            Statement::Conditional {
                condition,
                if_block,
                else_block,
            } => {
                let function = self.current_function();

                let cond_block = self.context.append_basic_block(function, "if.cond");

                let true_block = self.context.append_basic_block(function, "if.true");

                let false_block = match else_block {
                    Some(_) => Some(self.context.append_basic_block(function, "if.false")),
                    None => None,
                };

                let after_block = self.context.append_basic_block(function, "if.after");

                self.builder
                    .build_unconditional_branch(cond_block)
                    .map_err(|err| Box::new(CompilerError::at(ErrorSeverity::HIGH, err.to_string(), span)) as Box<dyn IError>)?;

                self.builder.position_at_end(cond_block);

                self.visit_expression(condition)?;

                let cond_value = self.read_last_value()?.into_int_value(span)?;

                self.builder
                    .build_conditional_branch(cond_value, true_block, false_block.unwrap_or(after_block))
                    .map_err(|err| Box::new(CompilerError::at(ErrorSeverity::HIGH, err.to_string(), span)) as Box<dyn IError>)?;

                self.builder.position_at_end(true_block);

                self.visit_block(if_block)?;

                self.branch_if_no_terminator(after_block, span)?;

                if let Some(b) = false_block {
                    self.builder.position_at_end(b);

                    self.visit_block(else_block.as_ref().expect("else block should exist"))?;

                    self.branch_if_no_terminator(after_block, span)?;
                }

                self.builder.position_at_end(after_block);

                Ok(())
            }

            Statement::WhileLoop { condition, block } => {
                let function = self.current_function();

                let scope_depth = self.scopes.len();

                let cond_block = self.context.append_basic_block(function, "while.cond");

                let body_block = self.context.append_basic_block(function, "while.block");

                let after_block = self.context.append_basic_block(function, "while.after");

                self.builder
                    .build_unconditional_branch(cond_block)
                    .map_err(|err| Box::new(CompilerError::at(ErrorSeverity::HIGH, err.to_string(), span)) as Box<dyn IError>)?;

                self.builder.position_at_end(cond_block);

                self.visit_expression(condition)?;

                let cond_value = self.read_last_value()?.into_int_value(span)?;

                self.builder
                    .build_conditional_branch(cond_value, body_block, after_block)
                    .map_err(|err| Box::new(CompilerError::at(ErrorSeverity::HIGH, err.to_string(), span)) as Box<dyn IError>)?;

                self.builder.position_at_end(body_block);

                self.control_stack.push(ControlFrame::Loop {
                    continue_block: cond_block,
                    break_block: after_block,
                    scope_depth,
                });

                self.visit_block(block)?;

                self.control_stack.pop();

                self.branch_if_no_terminator(cond_block, span)?;

                self.builder.position_at_end(after_block);

                Ok(())
            }

            Statement::Return(value) => {
                match value {
                    Some(expr) => {
                        self.visit_expression(expr)?;

                        let return_value = self.read_last_value()?;

                        // Protect the returned value from the scope release
                        // below: if it's a bare variable read, it aliases a
                        // local that's about to be released, so retain it
                        // first to keep it alive across that release. Any
                        // other expression already evaluates to an owned +1
                        // reference not aliased by a local, so no extra
                        // retain is needed there.
                        if return_value.is_refcounted() && Self::expr_needs_retain(&expr.value) {
                            self.retain_value(&return_value, span)?;
                        }

                        self.release_all_scopes(span)?;

                        self.builder
                            .build_return(Some(&return_value.as_basic_value_enum()))
                            .map_err(|err| Box::new(CompilerError::at(ErrorSeverity::HIGH, err.to_string(), span)) as Box<dyn IError>)?;
                    }

                    None => {
                        self.release_all_scopes(span)?;

                        self.builder
                            .build_return(None)
                            .map_err(|err| Box::new(CompilerError::at(ErrorSeverity::HIGH, err.to_string(), span)) as Box<dyn IError>)?;
                    }
                }

                Ok(())
            }

            Statement::Break => {
                let (target, scope_depth) = self.find_break_target(span)?;

                self.release_scopes_from(scope_depth, span)?;

                self.branch_if_no_terminator(target, span)?;

                Ok(())
            }

            Statement::Continue => {
                let (target, scope_depth) = self.find_continue_target(span)?;

                self.release_scopes_from(scope_depth, span)?;

                self.branch_if_no_terminator(target, span)?;

                Ok(())
            }

            Statement::Switch { expressions, cases } => self.compile_switch(expressions, cases),
        }
    }

    /// Loads whatever `ptr` (of type `ty`) currently holds and releases it,
    /// if it's an owned heap value. Used right before overwriting a
    /// variable or an indexed slot.
    fn release_current_value(&mut self, ptr: inkwell::values::PointerValue<'ctx>, ty: &Type, span: Span) -> Result<(), Box<dyn IError>> {
        if !matches!(ty, Type::Str | Type::Vector(_) | Type::Struct { .. }) {
            return Ok(());
        }

        let err = Self::builder_err(span);

        let llvm_type = LlvmValue::type_to_basic_type_enum(ty, self.context).ok_or_else(|| {
            Box::new(CompilerError::at(
                ErrorSeverity::HIGH,
                format!("Compiling values of type '{}' is not yet supported.", ty),
                span,
            )) as Box<dyn IError>
        })?;

        let current_raw = self.builder.build_load(llvm_type, ptr, "release.current").map_err(&err)?;
        let current_value = LlvmValue::from_basic_value_enum(current_raw, ty);

        self.release_value(&current_value, span)
    }

    /// Prepares a freshly-evaluated value to be stored into a brand new
    /// owning slot (a variable, a struct field, a vector element, ...):
    /// strings are always deep-copied, and Vector/Struct values are
    /// retained only if `source_expr` was a bare variable read (see
    /// `expr_needs_retain`).
    pub(in crate::backend) fn finalize_owned_value_for_new_slot(
        &mut self,
        value: LlvmValue<'ctx>,
        source_expr: &Expression,
        span: Span,
    ) -> Result<LlvmValue<'ctx>, Box<dyn IError>> {
        match value {
            LlvmValue::Str(ptr) => {
                let copied = self.build_string_copy(ptr, span)?;
                if Self::expr_needs_release(source_expr) {
                    self.release_value(&value, span)?;
                }
                Ok(LlvmValue::Str(copied))
            }

            LlvmValue::Vector(_, _) | LlvmValue::Struct(_, _) => {
                if Self::expr_needs_retain(source_expr) {
                    self.retain_value(&value, span)?;
                }
                Ok(value)
            }

            other => Ok(other),
        }
    }
}
