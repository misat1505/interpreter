use crate::{
    common::{
        errors::{ErrorSeverity, IError, SemanticCheckerError},
        span::Span,
        types::Type,
        visitor::Visitor,
    },
    frontend::ast::{Accessor, Expression, Node},
    semantic::{
        semantic_checker::{checker::HoverInfo, functions::FunctionCallType, SemanticChecker},
        type_alu::TypeALU,
    },
};

impl<'a> SemanticChecker<'a> {
    pub(in crate::semantic::semantic_checker) fn check_index_assignment(
        &mut self,
        identifier: &'a Node<String>,
        accessors: &'a Vec<Node<Accessor>>,
        value: &'a Node<Expression>,
        span: Span,
    ) {
        let mut current_type = match self.stack.get_variable(identifier.value.as_str(), identifier.span) {
            Ok(t) => t.clone(),
            Err(err) => {
                self.errors
                    .push(Box::new(SemanticCheckerError::at(ErrorSeverity::HIGH, err.message(), identifier.span)));
                return;
            }
        };
        self.hovers.push(HoverInfo {
            contents: format!("```raptor\n{} {}\n```", current_type, identifier.value),
            span: identifier.span,
        });
        for accessor in accessors {
            match &accessor.value {
                Accessor::Index(index_expr) => {
                    let _ = self.visit_expression(index_expr);
                    let idx_type = match self.read_last_result(index_expr.span) {
                        Ok(t) => t,
                        Err(_) => return,
                    };
                    if idx_type != Type::I64 {
                        self.errors.push(Box::new(SemanticCheckerError::type_mismatch(
                            ErrorSeverity::HIGH,
                            String::from("Array index must be of type `i64`."),
                            &Type::I64,
                            &idx_type,
                            index_expr.span,
                        )));
                        return;
                    }
                    current_type = match current_type {
                        Type::Vector(inner) => *inner,
                        Type::Str => Type::Char,
                        other => {
                            self.errors.push(Box::new(SemanticCheckerError::at(
                                ErrorSeverity::HIGH,
                                format!("Cannot index into a value of type `{}`.", other),
                                index_expr.span,
                            )));
                            return;
                        }
                    };
                }
                Accessor::Field(field) => {
                    let Type::Struct {
                        identifier: struct_name,
                        fields,
                    } = &current_type
                    else {
                        self.errors.push(Box::new(SemanticCheckerError::at(
                            ErrorSeverity::HIGH,
                            format!("Cannot access field `{}` on a value of type `{}`.", field.value, current_type),
                            field.span,
                        )));
                        return;
                    };
                    let Some(field_type) = fields.get(&field.value).cloned() else {
                        self.errors.push(Box::new(SemanticCheckerError::at(
                            ErrorSeverity::HIGH,
                            format!("Struct `{}` has no field named `{}`.", struct_name, field.value),
                            field.span,
                        )));
                        return;
                    };
                    current_type = match self.resolve_type_fully_checked(&field_type, field.span) {
                        Ok(t) => t,
                        Err(_) => return,
                    };
                    self.hovers.push(HoverInfo {
                        contents: format!("```raptor\n{} {}\n```", current_type, field.value),
                        span: field.span,
                    });
                }
            }
        }
        let _ = self.visit_expression(value);
        let actual_type = match self.read_last_result(value.span) {
            Ok(t) => t,
            Err(_) => return,
        };
        let compatible = match (&current_type, &actual_type) {
            (Type::Vector(_), Type::Vector(inner)) if **inner == Type::Void => true,
            _ => actual_type == current_type,
        };
        if !compatible {
            self.errors.push(Box::new(SemanticCheckerError::type_mismatch(
                ErrorSeverity::HIGH,
                format!("Cannot assign a value of type `{}` to a target of type `{}`.", actual_type, current_type),
                &current_type,
                &actual_type,
                span,
            )));
        }
    }

    pub(in crate::semantic::semantic_checker) fn check_expression(&mut self, expression: &'a Node<Expression>) -> Result<(), Box<dyn IError>> {
        if let Expression::FunctionCall { .. } = &expression.value {
            self.check_function_call(FunctionCallType::Expression(expression));
            return Ok(());
        }
        match &expression.value {
            Expression::Alternative(lhs, rhs) => self.evaluate_binary_op(lhs, rhs, TypeALU::alternative)?,
            Expression::Concatenation(lhs, rhs) => self.evaluate_binary_op(lhs, rhs, TypeALU::concatenation)?,
            Expression::Greater(lhs, rhs) => self.evaluate_binary_op(lhs, rhs, TypeALU::greater)?,
            Expression::GreaterEqual(lhs, rhs) => self.evaluate_binary_op(lhs, rhs, TypeALU::greater_or_equal)?,
            Expression::Less(lhs, rhs) => self.evaluate_binary_op(lhs, rhs, TypeALU::less)?,
            Expression::LessEqual(lhs, rhs) => self.evaluate_binary_op(lhs, rhs, TypeALU::less_or_equal)?,
            Expression::Equal(lhs, rhs) => self.evaluate_binary_op(lhs, rhs, TypeALU::equal)?,
            Expression::NotEqual(lhs, rhs) => self.evaluate_binary_op(lhs, rhs, TypeALU::not_equal)?,
            Expression::Addition(lhs, rhs) => self.evaluate_binary_op(lhs, rhs, TypeALU::add)?,
            Expression::Subtraction(lhs, rhs) => self.evaluate_binary_op(lhs, rhs, TypeALU::subtract)?,
            Expression::Multiplication(lhs, rhs) => self.evaluate_binary_op(lhs, rhs, TypeALU::multiplication)?,
            Expression::Division(lhs, rhs) => self.evaluate_binary_op(lhs, rhs, TypeALU::division)?,
            Expression::Modulo(lhs, rhs) => self.evaluate_binary_op(lhs, rhs, TypeALU::modulo)?,
            Expression::BooleanNegation(value) => self.evaluate_unary_op(value, TypeALU::boolean_negate)?,
            Expression::ArithmeticNegation(value) => self.evaluate_unary_op(value, TypeALU::arithmetic_negate)?,
            Expression::Casting { value, to_type } => {
                self.visit_expression(value)?;
                let from_type = self.read_last_result(value.span);
                match from_type {
                    Ok(t) => match TypeALU::cast_to_type(t, &to_type.value, Span::new(value.span.start(), to_type.span.end())) {
                        Ok(result_type) => {
                            self.last_result = Some(result_type);
                        }
                        Err(err) => {
                            self.errors
                                .push(Box::new(SemanticCheckerError::at(ErrorSeverity::HIGH, err.message(), expression.span)));
                            self.last_result = None;
                        }
                    },
                    Err(_) => {
                        self.last_result = None;
                    }
                }
            }
            Expression::Literal(literal) => self.visit_literal(literal)?,
            Expression::Variable(variable) => self.visit_variable(variable, expression.span)?,
            Expression::FunctionCall { .. } => {
                unreachable!("Function call is handled separately.")
            }
            Expression::Vector(vector) => self.visit_vector_literal(vector)?,
            Expression::Index { collection, index } => {
                self.visit_expression(collection)?;
                let collection_type = self.read_last_result(collection.span);
                self.visit_expression(index)?;
                let index_type = self.read_last_result(index.span);
                match (collection_type, index_type) {
                    (Ok(Type::Vector(inner)), Ok(Type::I64)) => {
                        self.last_result = self.resolve_type_fully_checked(&inner, expression.span).ok();
                    }
                    (Ok(Type::Str), Ok(Type::I64)) => {
                        self.last_result = Some(Type::Char);
                    }
                    (Ok(other), Ok(Type::I64)) => {
                        self.errors.push(Box::new(SemanticCheckerError::at(
                            ErrorSeverity::HIGH,
                            format!("Cannot index into a value of type `{}`.", other),
                            expression.span,
                        )));
                        self.last_result = None;
                    }
                    (Ok(_), Ok(other)) => {
                        self.errors.push(Box::new(SemanticCheckerError::type_mismatch(
                            ErrorSeverity::HIGH,
                            String::from("Array index must be of type `i64`."),
                            &Type::I64,
                            &other,
                            index.span,
                        )));
                        self.last_result = None;
                    }
                    _ => {
                        self.last_result = None;
                    }
                }
            }
            Expression::StructLiteral(sl) => self.visit_struct_literal(sl)?,
            Expression::FieldAccess { instance, field } => {
                self.visit_expression(instance)?;
                let Ok(instance_type) = self.read_last_result(instance.span) else {
                    self.last_result = None;
                    return Ok(());
                };
                let Type::Struct { identifier, fields } = &instance_type else {
                    self.errors.push(Box::new(SemanticCheckerError::at(
                        ErrorSeverity::HIGH,
                        format!("Cannot access field `{}` on a value of type `{}`.", field.value, instance_type),
                        expression.span,
                    )));
                    self.last_result = None;
                    return Ok(());
                };
                let Some(field_type) = fields.get(&field.value).cloned() else {
                    self.errors.push(Box::new(SemanticCheckerError::at(
                        ErrorSeverity::HIGH,
                        format!("Struct `{}` has no field named `{}`.", identifier, field.value),
                        field.span,
                    )));
                    self.last_result = None;
                    return Ok(());
                };
                let Some(field_type) = self.resolve_type_fully_checked(&field_type, field.span).ok() else {
                    self.last_result = None;
                    return Ok(());
                };
                self.hovers.push(HoverInfo {
                    contents: format!("```raptor\n{} {}\n```", field_type, field.value),
                    span: field.span,
                });
                self.last_result = Some(field_type);
            }
        }
        Ok(())
    }
}
