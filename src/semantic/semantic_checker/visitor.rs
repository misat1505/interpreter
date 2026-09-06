use crate::{
    common::{
        errors::{ErrorSeverity, IError, SemanticCheckerError},
        span::Span,
        types::Type,
        visitor::Visitor,
    },
    frontend::ast::{Argument, Block, Expression, Literal, Node, Parameter, Program, Statement, StructLiteral, SwitchCase, SwitchExpression},
    semantic::semantic_checker::{checker::HoverInfo, functions::FunctionCallType, SemanticChecker},
};
use std::collections::HashSet;

impl<'a> Visitor<'a> for SemanticChecker<'a> {
    #![allow(unused_must_use)]
    fn visit_program(&mut self, program: &'a Program) -> Result<(), Box<dyn IError>> {
        for statement in &program.statements {
            self.visit_statement(statement);
        }
        self.unused_variables_in_last_scope_warn();
        for function in program.functions.values() {
            self.stack.push_stack_frame().map_err(Box::new);
            self.visit_type(&function.value.return_type)?;
            let raw_return_type = self.read_last_result(function.value.return_type.span)?;
            let resolved_return_type = self.resolve_type_fully_checked(&raw_return_type, function.value.return_type.span)?;
            let mut function_declaration = function.value.clone();
            let return_type_span = function_declaration.return_type.span;
            function_declaration.return_type = Node {
                value: resolved_return_type,
                span: return_type_span,
            };

            self.current_function_declaration = Some(function_declaration);
            for param in &function.value.parameters {
                let param_name = &param.value.identifier.value;
                self.visit_type(&param.value.parameter_type)?;
                let raw_t = self.read_last_result(param.value.parameter_type.span)?;
                let t = self.resolve_type_fully_checked(&raw_t, param.value.parameter_type.span)?;
                if let Err(err) = self.stack.declare_variable(param_name, t, param.value.identifier.span) {
                    self.errors
                        .push(Box::new(SemanticCheckerError::at(ErrorSeverity::HIGH, err.message(), param.span)));
                }
            }

            self.visit_block(&function.value.block);
            self.unused_variables_in_last_scope_warn();
            self.current_function_declaration = None;
            self.stack.pop_stack_frame();
        }
        Ok(())
    }

    fn visit_expression(&mut self, expression: &'a Node<Expression>) -> Result<(), Box<dyn IError>> {
        self.check_expression(expression)
    }

    fn visit_statement(&mut self, statement: &'a Node<Statement>) -> Result<(), Box<dyn IError>> {
        match statement.value {
            Statement::Import { .. } => unreachable!("Imports have to be resolved before semantic check"),
            Statement::FunctionCall { .. } => self.check_function_call(FunctionCallType::Statement(statement)),
            Statement::Declaration { .. } => self.check_declaration(statement)?,
            Statement::Assignment { .. } => self.check_assignment(statement)?,
            Statement::Conditional { .. } => self.check_conditional(statement)?,
            Statement::WhileLoop { .. } => self.check_while_loop(statement)?,
            Statement::ForLoop { .. } => self.check_for_loop(statement)?,
            Statement::Switch { .. } => self.check_switch(statement)?,
            Statement::Return { .. } => self.check_return(statement)?,
            Statement::Break => self.check_break(statement)?,
            Statement::Continue => self.check_continue(statement)?,
        }
        Ok(())
    }

    fn visit_argument(&mut self, argument: &'a Node<Argument>) -> Result<(), Box<dyn IError>> {
        self.visit_expression(&argument.value.value)?;
        Ok(())
    }

    fn visit_block(&mut self, block: &'a Node<Block>) -> Result<(), Box<dyn IError>> {
        self.stack.push_scope();
        for statement in &block.value.0 {
            self.visit_statement(statement);
        }
        self.unused_variables_in_last_scope_warn();
        self.stack.pop_scope();
        Ok(())
    }

    fn visit_parameter(&mut self, parameter: &'a Node<Parameter>) -> Result<(), Box<dyn IError>> {
        self.visit_type(&parameter.value.parameter_type)?;
        Ok(())
    }

    fn visit_switch_case(&mut self, switch_case: &'a Node<SwitchCase>) -> Result<(), Box<dyn IError>> {
        self.check_switch_case(switch_case)
    }

    fn visit_switch_expression(&mut self, switch_expression: &'a Node<SwitchExpression>) -> Result<(), Box<dyn IError>> {
        self.check_switch_expression(switch_expression)
    }

    fn visit_type(&mut self, node_type: &'a Node<Type>) -> Result<(), Box<dyn IError>> {
        let resolved_type = match &node_type.value {
            Type::Unresolved(name) => self.program.types.get(name).cloned().ok_or_else(|| {
                let err = SemanticCheckerError::at(ErrorSeverity::HIGH, format!("Unknown type `{}`.", name), node_type.span);
                self.errors.push(Box::new(err.clone()));
                Box::new(err) as Box<dyn IError>
            })?,
            other => other.clone(),
        };
        self.last_result = Some(resolved_type);
        Ok(())
    }

    fn visit_literal(&mut self, literal: &'a Literal) -> Result<(), Box<dyn IError>> {
        let t = match literal {
            Literal::F64(_) => Type::F64,
            Literal::I64(_) => Type::I64,
            Literal::String(_) => Type::Str,
            Literal::Char(_) => Type::Char,
            Literal::False | Literal::True => Type::Bool,
        };
        self.last_result = Some(t);
        Ok(())
    }

    fn visit_variable(&mut self, variable: &'a str, span: Span) -> Result<(), Box<dyn IError>> {
        let value = self.stack.get_variable(variable, span).map_err(|err| {
            let error = SemanticCheckerError::at(ErrorSeverity::HIGH, err.message(), span);
            self.errors.push(Box::new(error.clone()));
            Box::new(error) as Box<dyn IError>
        })?;
        self.hovers.push(HoverInfo {
            contents: format!("```raptor\n{} {}\n```", value, variable),
            span,
        });
        self.last_result = Some(value.clone());
        Ok(())
    }

    fn visit_vector_literal(&mut self, vector: &'a [Box<Node<Expression>>]) -> Result<(), Box<dyn IError>> {
        let mut element_type: Option<Type> = None;
        for expression in vector {
            self.visit_expression(expression)?;
            if let Ok(t) = self.read_last_result(expression.span) {
                match &element_type {
                    None => {
                        element_type = Some(t);
                    }
                    Some(expected) if *expected != t => {
                        self.errors.push(Box::new(SemanticCheckerError::type_mismatch(
                            ErrorSeverity::HIGH,
                            String::from("Vector elements have mismatched types."),
                            expected,
                            &t,
                            expression.span,
                        )));
                    }
                    _ => {}
                }
            }
        }
        let vector_type = Type::Vector(Box::new(element_type.unwrap_or(Type::Void)));
        if let (Some(first), Some(last)) = (vector.first(), vector.last()) {
            let span = Span::new(first.span.start(), last.span.end());
            self.hovers.push(HoverInfo {
                contents: format!("```raptor\n{}\n```", vector_type),
                span,
            });
        }
        self.last_result = Some(vector_type);
        Ok(())
    }
}

impl<'a> SemanticChecker<'a> {
    pub(in crate::semantic::semantic_checker) fn visit_struct_literal(&mut self, node: &'a Node<StructLiteral>) -> Result<(), Box<dyn IError>> {
        let identifier = &node.value.identifier;
        let fields = &node.value.fields;
        let type_node: &'a Node<Type> = Box::leak(Box::new(Node {
            value: Type::Unresolved(identifier.value.clone()),
            span: identifier.span,
        }));
        let _ = self.visit_type(type_node);
        let Ok(declared_type) = self.read_last_result(identifier.span) else {
            return Ok(());
        };
        let Type::Struct {
            identifier: struct_name,
            fields: expected_fields,
        } = &declared_type
        else {
            let error = SemanticCheckerError::at(
                ErrorSeverity::HIGH,
                format!("`{}` is not a struct type.", identifier.value),
                identifier.span,
            );
            self.errors.push(Box::new(error));
            return Ok(());
        };
        let mut seen_fields = HashSet::new();
        for field in fields {
            let field_name = &field.value.identifier.value;
            self.visit_expression(&field.value.value)?;
            let Ok(actual_type) = self.read_last_result(field.value.value.span) else {
                continue;
            };
            let Some(expected_type) = expected_fields.get(field_name) else {
                let error = SemanticCheckerError::at(
                    ErrorSeverity::HIGH,
                    format!("Struct `{}` has no field named `{}`.", struct_name, field_name),
                    field.value.identifier.span,
                );
                self.errors.push(Box::new(error));
                continue;
            };
            let expected_type = match self.resolve_type_fully_checked(expected_type, field.span) {
                Ok(t) => t,
                Err(_) => continue,
            };
            let actual_type = match self.resolve_type_fully_checked(&actual_type, field.value.value.span) {
                Ok(t) => t,
                Err(_) => continue,
            };
            let compatible = match (&expected_type, &actual_type) {
                (Type::Vector(expected_inner), Type::Vector(actual_inner)) if **actual_inner == Type::Void => true,
                _ => expected_type.is_compatible(&actual_type),
            };
            if !compatible {
                let error = SemanticCheckerError::type_mismatch(
                    ErrorSeverity::HIGH,
                    format!(
                        "Cannot assign a value of type `{}` to field `{}` of struct `{}`.",
                        actual_type, field_name, struct_name
                    ),
                    &expected_type,
                    &actual_type,
                    field.span,
                );
                self.errors.push(Box::new(error));
            }
            if !seen_fields.insert(field_name.clone()) {
                let error = SemanticCheckerError::at(
                    ErrorSeverity::HIGH,
                    format!("Field `{}` is specified more than once.", field_name),
                    field.value.identifier.span,
                );
                self.errors.push(Box::new(error));
            }
        }
        for expected_name in expected_fields.keys() {
            if !seen_fields.contains(expected_name) {
                let error = SemanticCheckerError::at(
                    ErrorSeverity::HIGH,
                    format!("Missing field `{}` in initializer of struct `{}`.", expected_name, struct_name),
                    node.span,
                );
                self.errors.push(Box::new(error));
            }
        }
        self.hovers.push(HoverInfo {
            contents: format!("```raptor\n{}\n```", declared_type),
            span: identifier.span,
        });
        self.last_result = Some(declared_type);
        Ok(())
    }

    pub(in crate::semantic::semantic_checker) fn resolve_type_fully_checked(&mut self, ty: &Type, span: Span) -> Result<Type, Box<dyn IError>> {
        match ty {
            Type::Unresolved(name) => self.program.types.get(name).cloned().ok_or_else(|| {
                let err = SemanticCheckerError::at(ErrorSeverity::HIGH, format!("Unknown type `{}`.", name), span);
                self.errors.push(Box::new(err.clone()));
                Box::new(err) as Box<dyn IError>
            }),
            Type::Vector(inner) => Ok(Type::Vector(Box::new(self.resolve_type_fully_checked(inner, span)?))),
            other => Ok(other.clone()),
        }
    }
}
