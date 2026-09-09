use crate::{
    common::{
        errors::{ErrorSeverity, IError, SemanticCheckerError},
        span::Span,
        types::Type,
        visitor::Visitor,
    },
    frontend::{
        ast::{DeclaredType, EnumDeclaration, Node, Statement, SwitchCase, SwitchExpression, VariableDeclarationKind},
        tokens::TokenCategory,
    },
    semantic::semantic_checker::{
        checker::{type_prefix, DefinitionInfo, HoverInfo},
        SemanticChecker,
    },
};

impl<'a> SemanticChecker<'a> {
    pub(in crate::semantic::semantic_checker) fn check_declaration(&mut self, statement: &'a Node<Statement>) -> Result<(), Box<dyn IError>> {
        let Statement::Declaration { identifier, kind } = &statement.value else {
            return Ok(());
        };
        match kind {
            VariableDeclarationKind::TYPE { var_type, value } => {
                let _ = self.visit_type(var_type);
                let declared_type = match self.read_last_result(var_type.span) {
                    Ok(declared_type) => declared_type,
                    Err(_) => return Ok(()),
                };
                let resolved_type = match value {
                    Some(value) => {
                        let _ = self.visit_expression(value);
                        match self.read_last_result(value.span) {
                            Ok(actual_type) => {
                                let types_compatible = declared_type == actual_type
                                    || matches!(
                                        (&declared_type, &actual_type),
                                        (Type::Vector(_), Type::Vector(inner))
                                            if **inner == Type::Void
                                    );
                                if !types_compatible {
                                    let error = SemanticCheckerError::type_mismatch(
                                        ErrorSeverity::HIGH,
                                        format!(
                                            "Cannot assign a value of type `{}` to variable `{}` of type `{}`.",
                                            actual_type, identifier.value, declared_type
                                        ),
                                        &declared_type,
                                        &actual_type,
                                        statement.span,
                                    );
                                    self.errors.push(Box::new(error));
                                }
                                Some(declared_type.clone())
                            }
                            Err(_) => None,
                        }
                    }
                    None => Some(declared_type.clone()),
                };
                if let Some(resolved_type) = resolved_type.clone() {
                    if let Err(err) = self.stack.declare_variable(identifier.value.as_str(), resolved_type, identifier.span) {
                        self.errors
                            .push(Box::new(SemanticCheckerError::at(ErrorSeverity::HIGH, err.message(), statement.span)));
                    }
                }
                self.identifier_hover(&resolved_type.unwrap_or(Type::Void), identifier);
            }
            VariableDeclarationKind::LET { var_type, value } => {
                let _ = self.visit_expression(value);
                let resolved_type = self.read_last_result(value.span).ok();
                let final_type = match var_type {
                    Some(var_type) => {
                        let _ = self.visit_type(var_type);
                        let declared_type = match self.read_last_result(var_type.span) {
                            Ok(declared_type) => self.resolve_type_fully_checked(&declared_type, var_type.span)?,
                            Err(_) => return Ok(()),
                        };
                        if let Some(resolved_type) = &resolved_type {
                            let types_compatible = declared_type == *resolved_type
                                || matches!(
                                    (&declared_type, resolved_type),
                                    (Type::Vector(_), Type::Vector(inner))
                                        if **inner == Type::Void
                                );
                            if !types_compatible {
                                let error = SemanticCheckerError::type_mismatch(
                                    ErrorSeverity::HIGH,
                                    format!(
                                        "Cannot assign a value of type `{}` to variable `{}` of type `{}`.",
                                        resolved_type, identifier.value, declared_type
                                    ),
                                    &declared_type,
                                    resolved_type,
                                    statement.span,
                                );
                                self.errors.push(Box::new(error));
                            }
                        }
                        if let Err(err) = self
                            .stack
                            .declare_variable(identifier.value.as_str(), declared_type.clone(), identifier.span)
                        {
                            self.errors
                                .push(Box::new(SemanticCheckerError::at(ErrorSeverity::HIGH, err.message(), statement.span)));
                        }
                        declared_type
                    }
                    None => match resolved_type {
                        Some(resolved_type) => {
                            if matches!(
                                &resolved_type,
                                Type::Vector(inner) if **inner == Type::Void
                            ) {
                                let error = SemanticCheckerError::at(
                                    ErrorSeverity::HIGH,
                                    format!(
                                        "Cannot infer the type of an empty vector. Add a type annotation, e.g. `let {}: {} = [];`.",
                                        identifier.value,
                                        Type::Vector(Box::new(Type::I64))
                                    ),
                                    statement.span,
                                );
                                self.errors.push(Box::new(error));
                            } else if resolved_type == Type::Void {
                                let error = SemanticCheckerError::at(
                                    ErrorSeverity::HIGH,
                                    format!("Cannot assign a value of type `{}` to variable `{}`.", resolved_type, identifier.value),
                                    statement.span,
                                );
                                self.errors.push(Box::new(error));
                            } else if let Err(err) = self
                                .stack
                                .declare_variable(identifier.value.as_str(), resolved_type.clone(), identifier.span)
                            {
                                self.errors
                                    .push(Box::new(SemanticCheckerError::at(ErrorSeverity::HIGH, err.message(), statement.span)));
                            }
                            resolved_type
                        }
                        None => Type::Void,
                    },
                };
                self.identifier_hover(&final_type, identifier);
            }
        }
        Ok(())
    }

    pub(in crate::semantic::semantic_checker) fn check_assignment(&mut self, statement: &'a Node<Statement>) -> Result<(), Box<dyn IError>> {
        let Statement::Assignment {
            accessors,
            value,
            identifier,
        } = &statement.value
        else {
            return Ok(());
        };
        if accessors.is_empty() {
            self.visit_expression(value)?;
            let value = self.read_last_result(value.span).map_err(|_| {
                let error = SemanticCheckerError::at(
                    ErrorSeverity::HIGH,
                    format!("Cannot assign to variable `{}`: no value was produced.", identifier.value),
                    statement.span,
                );
                self.errors.push(Box::new(error.clone()));
                Box::new(error) as Box<dyn IError>
            })?;
            if let Err(err) = self.stack.assign_variable(identifier.value.as_str(), value.clone(), statement.span) {
                self.errors
                    .push(Box::new(SemanticCheckerError::at(ErrorSeverity::HIGH, err.message(), statement.span)));
            }
            self.identifier_hover(&value, identifier);

            if let Ok(def_span) = self.stack.get_variable_declaration_span(identifier.value.as_str(), statement.span) {
                self.definitions.push(DefinitionInfo {
                    def_span: *def_span,
                    use_span: identifier.span,
                });
            }
        } else {
            self.check_index_assignment(identifier, accessors, value, statement.span);
        }
        Ok(())
    }

    pub(in crate::semantic::semantic_checker) fn check_conditional(&mut self, statement: &'a Node<Statement>) -> Result<(), Box<dyn IError>> {
        let Statement::Conditional {
            condition,
            else_block,
            if_block,
        } = &statement.value
        else {
            return Ok(());
        };
        let _ = self.visit_expression(condition);
        if let Ok(resolved_condition) = self.read_last_result(condition.span) {
            if resolved_condition != Type::Bool {
                let error = SemanticCheckerError::type_mismatch(
                    ErrorSeverity::HIGH,
                    String::from("If condition must be of type `bool`."),
                    &Type::Bool,
                    &resolved_condition,
                    condition.span,
                );
                self.errors.push(Box::new(error));
            }
        }
        let _ = self.visit_block(if_block);
        if let Some(else_blk) = else_block {
            let _ = self.visit_block(else_blk);
        }
        Ok(())
    }

    pub(in crate::semantic::semantic_checker) fn check_while_loop(&mut self, statement: &'a Node<Statement>) -> Result<(), Box<dyn IError>> {
        let Statement::WhileLoop { block, condition } = &statement.value else {
            return Ok(());
        };
        let _ = self.visit_expression(condition);
        if let Ok(resolved_condition) = self.read_last_result(condition.span) {
            if resolved_condition != Type::Bool {
                let error = SemanticCheckerError::type_mismatch(
                    ErrorSeverity::HIGH,
                    String::from("While condition must be of type `bool`."),
                    &Type::Bool,
                    &resolved_condition,
                    condition.span,
                );
                self.errors.push(Box::new(error));
            }
        }
        self.stack.enter_breakable();
        self.stack.enter_continuable();
        let _ = self.visit_block(block);
        self.stack.exit_breakable();
        self.stack.exit_continuable();
        Ok(())
    }

    pub(in crate::semantic::semantic_checker) fn check_for_loop(&mut self, statement: &'a Node<Statement>) -> Result<(), Box<dyn IError>> {
        let Statement::ForLoop {
            assignment,
            block,
            condition,
            declaration,
        } = &statement.value
        else {
            return Ok(());
        };
        self.stack.push_scope();
        if let Some(decl) = declaration {
            let _ = self.visit_statement(decl);
        }
        let _ = self.visit_expression(condition);
        if let Ok(resolved_condition) = self.read_last_result(condition.span) {
            if resolved_condition != Type::Bool {
                let error = SemanticCheckerError::type_mismatch(
                    ErrorSeverity::HIGH,
                    String::from("For loop condition must be of type `bool`."),
                    &Type::Bool,
                    &resolved_condition,
                    condition.span,
                );
                self.errors.push(Box::new(error));
            }
        }
        if let Some(assign) = assignment {
            let _ = self.visit_statement(assign);
        }
        self.stack.enter_breakable();
        self.stack.enter_continuable();
        let _ = self.visit_block(block);
        self.stack.exit_breakable();
        self.stack.exit_continuable();
        self.stack.pop_scope();
        Ok(())
    }

    pub(in crate::semantic::semantic_checker) fn check_switch(&mut self, statement: &'a Node<Statement>) -> Result<(), Box<dyn IError>> {
        let Statement::Switch { cases, expressions } = &statement.value else {
            return Ok(());
        };
        for expr in expressions {
            let _ = self.check_switch_expression(expr);
        }
        for case in cases {
            let _ = self.check_switch_case(case);
        }
        Ok(())
    }

    pub(in crate::semantic::semantic_checker) fn check_return(&mut self, statement: &'a Node<Statement>) -> Result<(), Box<dyn IError>> {
        let Statement::Return(value) = &statement.value else {
            return Ok(());
        };
        if self.stack.size() <= 1 {
            self.errors.push(Box::new(SemanticCheckerError::at(
                ErrorSeverity::HIGH,
                String::from("Return statement is not inside a function."),
                statement.span,
            )));
        }
        let actual_type = match value {
            Some(val) => {
                let _ = self.visit_expression(val);
                self.read_last_result(val.span).ok()
            }
            None => None,
        };
        if let Some(fn_declaration) = self.current_function_declaration.clone() {
            let fn_name = &fn_declaration.identifier.value;
            let expected = fn_declaration.return_type.value.clone();
            if expected == Type::Void && value.is_some() {
                self.errors.push(Box::new(SemanticCheckerError::at(
                    ErrorSeverity::HIGH,
                    format!("Function `{}` returns `{}` and must use `return;` without a value.", fn_name, Type::Void),
                    statement.span,
                )));
            }
            let is_ok = match (&actual_type, &expected) {
                (None, Type::Void) => true,
                (Some(t), exp) => exp.is_compatible(t),
                (None, _) => false,
            };
            if !is_ok {
                let got = actual_type.unwrap_or(Type::Void);
                self.errors.push(Box::new(SemanticCheckerError::type_mismatch(
                    ErrorSeverity::HIGH,
                    format!("Return type of function `{}` does not match the declared return type.", fn_name),
                    &expected,
                    &got,
                    statement.span,
                )));
            }
        }
        Ok(())
    }

    pub(in crate::semantic::semantic_checker) fn check_break(&mut self, statement: &'a Node<Statement>) -> Result<(), Box<dyn IError>> {
        if !self.stack.is_in_breakable() {
            self.errors.push(Box::new(SemanticCheckerError::at(
                ErrorSeverity::HIGH,
                String::from("Break statement is not inside a loop or switch case."),
                statement.span,
            )));
        }
        Ok(())
    }

    pub(in crate::semantic::semantic_checker) fn check_continue(&mut self, statement: &'a Node<Statement>) -> Result<(), Box<dyn IError>> {
        if !self.stack.is_in_continuable() {
            self.errors.push(Box::new(SemanticCheckerError::at(
                ErrorSeverity::HIGH,
                String::from("Continue statement is not inside a loop."),
                statement.span,
            )));
        }
        Ok(())
    }

    pub(in crate::semantic::semantic_checker) fn check_switch_case(&mut self, switch_case: &'a Node<SwitchCase>) -> Result<(), Box<dyn IError>> {
        let _ = self.visit_expression(&switch_case.value.condition);
        if let Ok(resolved_condition) = self.read_last_result(switch_case.value.condition.span) {
            if resolved_condition != Type::Bool {
                let error = SemanticCheckerError::type_mismatch(
                    ErrorSeverity::HIGH,
                    String::from("Switch case condition must be of type `bool`."),
                    &Type::Bool,
                    &resolved_condition,
                    switch_case.value.condition.span,
                );
                self.errors.push(Box::new(error));
            }
        }
        self.stack.enter_breakable();
        let _ = self.visit_block(&switch_case.value.block);
        self.stack.exit_breakable();
        Ok(())
    }

    pub(in crate::semantic::semantic_checker) fn check_switch_expression(
        &mut self,
        switch_expression: &'a Node<SwitchExpression>,
    ) -> Result<(), Box<dyn IError>> {
        let expression = &switch_expression.value.expression;
        let _ = self.visit_expression(expression);
        if let Ok(resolved_type) = self.read_last_result(expression.span) {
            match &switch_expression.value.alias {
                None => {}
                Some(id) => {
                    if let Err(err) = self.stack.declare_variable(id.value.as_str(), resolved_type, id.span) {
                        self.errors.push(Box::new(SemanticCheckerError::at(
                            ErrorSeverity::HIGH,
                            err.message(),
                            switch_expression.span,
                        )));
                    }
                }
            }
        }
        Ok(())
    }

    pub(in crate::semantic::semantic_checker) fn check_match_statement(
        &mut self,
        match_statement: &'a Node<Statement>,
    ) -> Result<(), Box<dyn IError>> {
        let Statement::Match {
            ref expression,
            ref match_arms,
            ref rest_arm,
        } = match_statement.value
        else {
            unreachable!();
        };

        self.visit_expression(&expression)?;
        let actual_type = self.read_last_result(expression.span)?;
        let resolved_type = self.resolve_type_fully_checked(&actual_type, expression.span)?;

        let Type::Enum {
            identifier: declared_enum_ident,
            fields: declared_fields,
        } = resolved_type
        else {
            self.errors.push(Box::new(SemanticCheckerError::at(
                ErrorSeverity::HIGH,
                format!("Cannot 'match' non-enum expression of type '{}'.", resolved_type),
                expression.span,
            )));

            return Ok(());
        };

        let mut visited_fields: Vec<String> = vec![];

        for match_arm in match_arms.iter() {
            if match_arm.value.enum_name.value != declared_enum_ident {
                self.errors.push(Box::new(SemanticCheckerError::at(
                    ErrorSeverity::HIGH,
                    format!(
                        "Cannot 'match' non-matching enum. Expected '{}', got: '{}'.",
                        declared_enum_ident, match_arm.value.enum_name.value
                    ),
                    expression.span,
                )));

                continue;
            }

            let Some(enum_definition_node) = self.program.declared_types.get(&match_arm.value.enum_name.value) else {
                unreachable!();
            };

            let DeclaredType::Enum(EnumDeclaration {
                members: ref enum_members_location,
                ..
            }) = enum_definition_node.value
            else {
                unreachable!();
            };

            self.definitions.push(DefinitionInfo {
                use_span: match_arm.value.enum_name.span,
                def_span: enum_definition_node.span,
            });

            self.hovers.push(HoverInfo {
                contents: format!("```raptor\nenum {}\n```", match_arm.value.enum_name.value),
                span: match_arm.value.enum_name.span,
            });

            if visited_fields.iter().any(|field| *field == match_arm.value.variant_name.value) {
                self.errors.push(Box::new(SemanticCheckerError::at(
                    ErrorSeverity::HIGH,
                    format!("Multiple arms for variant '{}'.", match_arm.value.variant_name.value),
                    expression.span,
                )));
            }

            let Some(declared_field) = declared_fields.get(&match_arm.value.variant_name.value) else {
                self.errors.push(Box::new(SemanticCheckerError::at(
                    ErrorSeverity::HIGH,
                    format!(
                        "Enum '{}' doesn't have variant '{}'.",
                        declared_enum_ident, match_arm.value.variant_name.value
                    ),
                    Span::new(match_arm.value.enum_name.span.start(), match_arm.value.variant_name.span.end()),
                )));

                visited_fields.push(match_arm.value.variant_name.value.clone());

                continue;
            };

            let member_definition_node = enum_members_location
                .iter()
                .find(|node| node.value.identifier.value == match_arm.value.variant_name.value)
                .expect("Variant not found should be already handled");

            self.definitions.push(DefinitionInfo {
                use_span: match_arm.value.variant_name.span,
                def_span: member_definition_node.span,
            });

            let value_str = match (declared_field, match_arm.value.variant_value.as_ref()) {
                (None, Some(val)) => {
                    self.errors.push(Box::new(SemanticCheckerError::at(
                        ErrorSeverity::HIGH,
                        format!(
                            "Enum '{}' variant '{}' doesn't contain any value.",
                            declared_enum_ident, match_arm.value.variant_name.value
                        ),
                        val.span,
                    )));

                    visited_fields.push(match_arm.value.variant_name.value.clone());

                    continue;
                }

                (Some(field), Some(var_node)) => {
                    let resolved_type = self.resolve_type_fully_checked(field, var_node.span)?;

                    format!(
                        "{}{}{}{}",
                        TokenCategory::ParenOpen,
                        type_prefix(&resolved_type),
                        resolved_type,
                        TokenCategory::ParenClose
                    )
                }
                (_, None) => "".to_owned(),
            };

            self.hovers.push(HoverInfo {
                contents: format!(
                    "```raptor\n{} {}{}{}{}\n```",
                    TokenCategory::Enum,
                    match_arm.value.enum_name.value,
                    TokenCategory::DoubleColon,
                    match_arm.value.variant_name.value,
                    value_str
                ),
                span: match_arm.value.variant_name.span,
            });

            visited_fields.push(match_arm.value.variant_name.value.clone());

            if let (Some(field), Some(var_node)) = (declared_field, match_arm.value.variant_value.as_ref()) {
                let resolved_type = self.resolve_type_fully_checked(field, var_node.span)?;

                self.stack.push_scope();

                if let Err(err) = self.stack.declare_variable(&var_node.value, resolved_type.clone(), var_node.span) {
                    self.errors.push(Box::new(err));
                }

                self.identifier_hover(&resolved_type, var_node);

                self.visit_block(&match_arm.value.block)?;

                self.unused_variables_in_last_scope_warn();
                self.stack.pop_scope();
            } else {
                self.visit_block(&match_arm.value.block)?;
            }
        }

        match rest_arm {
            Some(block) => {
                let all_variants_covered = declared_fields.keys().all(|variant| visited_fields.contains(variant));

                if all_variants_covered {
                    self.errors.push(Box::new(SemanticCheckerError::at(
                        ErrorSeverity::LOW,
                        "The 'rest' arm is unnecessary because all enum variants are already matched.".to_string(),
                        block.span,
                    )));
                }

                self.visit_block(block)?;
            }

            None => {
                let missing_variants = declared_fields
                    .keys()
                    .filter(|variant| !visited_fields.contains(variant))
                    .collect::<Vec<_>>();

                if missing_variants.len() > 0 {
                    let variants_concat = missing_variants
                        .iter()
                        .map(|variant| format!("'{}'", variant))
                        .collect::<Vec<_>>()
                        .join(", ");

                    let start_pos = match match_arms.last() {
                        None => match_statement.span.start(),
                        Some(arm) => arm.span.end(),
                    };
                    self.errors.push(Box::new(SemanticCheckerError::at(
                        ErrorSeverity::HIGH,
                        format!("Match is missing enum variants: {}.", variants_concat),
                        Span::new(start_pos, match_statement.span.end()),
                    )));
                }
            }
        };

        Ok(())
    }
}
