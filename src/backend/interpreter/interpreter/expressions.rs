use std::{cell::RefCell, collections::HashMap, rc::Rc};

use crate::{
    backend::interpreter::{
        alu::{value::Value, ALU},
        interpreter::Interpreter,
    },
    common::{
        errors::{ComputationError, ErrorSeverity, IError, InterpreterError},
        span::Span,
        types::Type,
        visitor::Visitor,
    },
    frontend::ast::{Expression, Literal, Node, StructLiteralField},
};

impl<'a> Interpreter<'a> {
    pub(in crate::backend::interpreter::interpreter) fn eval_expression(&mut self, expression: &'a Node<Expression>) -> Result<(), Box<dyn IError>> {
        match &expression.value {
            Expression::Casting { value, to_type } => {
                self.visit_expression(value)?;

                let computed_value = self.read_last_result()?;

                let value = ALU::cast_to_type(computed_value, &to_type.value, self.span)
                    .map_err(|err| Box::new(ComputationError::at(ErrorSeverity::HIGH, err.message(), self.span)) as Box<dyn IError>)?;

                self.last_result = Some(value);
            }

            Expression::BooleanNegation(value) => self.evaluate_unary_op(value, ALU::boolean_negate)?,
            Expression::ArithmeticNegation(value) => self.evaluate_unary_op(value, ALU::arithmetic_negate)?,

            Expression::Addition(lhs, rhs) => self.evaluate_binary_op(lhs, rhs, ALU::add)?,
            Expression::Subtraction(lhs, rhs) => self.evaluate_binary_op(lhs, rhs, ALU::subtract)?,
            Expression::Multiplication(lhs, rhs) => self.evaluate_binary_op(lhs, rhs, ALU::multiplication)?,
            Expression::Division(lhs, rhs) => self.evaluate_binary_op(lhs, rhs, ALU::division)?,
            Expression::Modulo(lhs, rhs) => self.evaluate_binary_op(lhs, rhs, ALU::modulo)?,

            Expression::Alternative(lhs, rhs) => self.evaluate_binary_op(lhs, rhs, ALU::alternative)?,
            Expression::Concatenation(lhs, rhs) => self.evaluate_binary_op(lhs, rhs, ALU::concatenation)?,

            Expression::Greater(lhs, rhs) => self.evaluate_binary_op(lhs, rhs, ALU::greater)?,
            Expression::GreaterEqual(lhs, rhs) => self.evaluate_binary_op(lhs, rhs, ALU::greater_or_equal)?,
            Expression::Less(lhs, rhs) => self.evaluate_binary_op(lhs, rhs, ALU::less)?,
            Expression::LessEqual(lhs, rhs) => self.evaluate_binary_op(lhs, rhs, ALU::less_or_equal)?,
            Expression::Equal(lhs, rhs) => self.evaluate_binary_op(lhs, rhs, ALU::equal)?,
            Expression::NotEqual(lhs, rhs) => self.evaluate_binary_op(lhs, rhs, ALU::not_equal)?,

            Expression::Literal(literal) => self.visit_literal(literal)?,
            Expression::Vector(vector) => self.visit_vector_literal(vector)?,
            Expression::Variable(variable) => self.visit_variable(variable, expression.span)?,
            Expression::FunctionCall { identifier, arguments } => self.call_function(identifier, arguments, expression.span)?,
            Expression::Index { collection, index } => self.eval_index(collection, index)?,
            Expression::StructLiteral(node) => self.eval_struct_literal(&node.value.identifier, &node.value.fields, expression.span)?,
            Expression::FieldAccess { instance, field } => self.eval_field_access(instance, field)?,
        }

        Ok(())
    }

    pub(in crate::backend::interpreter::interpreter) fn eval_field_access(
        &mut self,
        instance: &'a Node<Expression>,
        field: &'a Node<String>,
    ) -> Result<(), Box<dyn IError>> {
        self.visit_expression(instance)?;

        let instance_value = self.read_last_result()?;

        let Value::Struct { fields, .. } = instance_value else {
            return Err(Box::new(InterpreterError::expected_found(
                ErrorSeverity::HIGH,
                format!("Cannot access field '{}' on this value.", field.value),
                String::from("struct"),
                format!("{}", instance_value.to_type()),
                instance.span,
            )));
        };

        let borrowed_fields = fields.borrow();

        let field_cell = borrowed_fields.get(&field.value).ok_or_else(|| {
            Box::new(InterpreterError::at(
                ErrorSeverity::HIGH,
                format!("Struct has no field '{}'.", field.value),
                field.span,
            )) as Box<dyn IError>
        })?;

        self.last_result = Some(field_cell.borrow().clone());

        Ok(())
    }

    pub(in crate::backend::interpreter::interpreter) fn eval_literal(&mut self, literal: &Literal) -> Result<(), Box<dyn IError>> {
        let value = match literal {
            Literal::F64(value) => Value::F64(*value),
            Literal::I64(value) => Value::I64(*value),
            Literal::String(value) => Value::String(value.to_string()),
            Literal::Char(value) => Value::Char(*value),
            Literal::False => Value::Bool(false),
            Literal::True => Value::Bool(true),
        };

        self.last_result = Some(value);

        Ok(())
    }

    pub(in crate::backend::interpreter::interpreter) fn eval_vector_literal(
        &mut self,
        expressions: &'a [Box<Node<Expression>>],
    ) -> Result<(), Box<dyn IError>> {
        let values = Rc::new(RefCell::new(Vec::new()));

        for expression in expressions {
            self.visit_expression(expression)?;

            values.borrow_mut().push(Rc::new(RefCell::new(self.read_last_result()?)));
        }

        let kind = if let Some(first) = values.borrow().first() {
            Box::new(Type::Vector(Box::new(first.borrow().to_type())))
        } else {
            Box::new(Type::Void)
        };

        self.last_result = Some(Value::Vector { kind, values });

        Ok(())
    }

    pub(in crate::backend::interpreter::interpreter) fn eval_struct_literal(
        &mut self,
        identifier: &'a Node<String>,
        fields: &'a [Node<StructLiteralField>],
        span: Span,
    ) -> Result<(), Box<dyn IError>> {
        let declared_type = self.program.types.get(&identifier.value).cloned().ok_or_else(|| {
            Box::new(InterpreterError::at(
                ErrorSeverity::HIGH,
                format!("Unknown type '{}'.", identifier.value),
                span,
            )) as Box<dyn IError>
        })?;

        let Type::Struct { fields: fields_types, .. } = &declared_type else {
            return Err(Box::new(InterpreterError::at(
                ErrorSeverity::HIGH,
                format!("'{}' is not a struct type.", identifier.value),
                span,
            )));
        };

        let mut field_values = HashMap::new();

        for field in fields {
            let field_name = field.value.identifier.value.as_str();

            let expected_type = fields_types.get(field_name).ok_or_else(|| {
                Box::new(InterpreterError::at(
                    ErrorSeverity::HIGH,
                    format!("Struct `{}` has no field `{}`.", identifier.value, field_name),
                    field.value.identifier.span,
                )) as Box<dyn IError>
            })?;

            self.visit_expression(&field.value.value)?;
            let mut value = self.read_last_result()?;

            if let (Type::Vector(_), Value::Vector { kind, .. }) = (expected_type, &mut value) {
                if *kind.as_ref() == Type::Void {
                    **kind = expected_type.clone();
                }
            }

            field_values.insert(field.value.identifier.value.clone(), Rc::new(RefCell::new(value)));
        }

        self.last_result = Some(Value::Struct {
            identifier: identifier.value.clone(),
            fields_types: Rc::new(fields_types.clone()),
            fields: Rc::new(RefCell::new(field_values)),
        });

        Ok(())
    }

    pub(in crate::backend::interpreter::interpreter) fn eval_variable(&mut self, variable: &'a str, span: Span) -> Result<(), Box<dyn IError>> {
        let value = self
            .stack
            .get_variable(variable, span)
            .map_err(|err| Box::new(InterpreterError::at(ErrorSeverity::HIGH, err.message(), span)) as Box<dyn IError>)?;

        self.last_result = Some(value.borrow().clone());

        Ok(())
    }

    pub(in crate::backend::interpreter::interpreter) fn eval_index(
        &mut self,
        collection: &'a Node<Expression>,
        index: &'a Node<Expression>,
    ) -> Result<(), Box<dyn IError>> {
        self.visit_expression(collection)?;

        let collection_value = self.read_last_result()?;

        match collection_value {
            Value::Vector { values, .. } => {
                let idx = self.eval_index_value(index)?;

                let borrowed = values.borrow();

                let element_cell = borrowed.get(idx).ok_or_else(|| {
                    Box::new(InterpreterError::at(
                        ErrorSeverity::HIGH,
                        format!("Index {} out of bounds.", idx),
                        index.span,
                    )) as Box<dyn IError>
                })?;

                self.last_result = Some(element_cell.borrow().clone());

                Ok(())
            }

            Value::String(s) => {
                let idx = self.eval_index_value(index)?;

                let ch = s.chars().nth(idx).ok_or_else(|| {
                    Box::new(InterpreterError::at(
                        ErrorSeverity::HIGH,
                        format!("Index {} out of bounds.", idx),
                        index.span,
                    )) as Box<dyn IError>
                })?;

                self.last_result = Some(Value::Char(ch));

                Ok(())
            }

            other => Err(Box::new(InterpreterError::expected_found(
                ErrorSeverity::HIGH,
                String::from("Cannot index into this value."),
                String::from("Vector or Str"),
                format!("{}", other.to_type()),
                collection.span,
            ))),
        }
    }

    fn eval_index_value(&mut self, index: &'a Node<Expression>) -> Result<usize, Box<dyn IError>> {
        self.visit_expression(index)?;

        let index_value = self.read_last_result()?;

        match index_value {
            Value::I64(i) if i >= 0 => Ok(i as usize),

            other => Err(Box::new(InterpreterError::expected_found(
                ErrorSeverity::HIGH,
                String::from("Array index must be a non-negative i64."),
                String::from("I64"),
                format!("{}", other.to_type()),
                index.span,
            ))),
        }
    }

    pub(in crate::backend::interpreter::interpreter) fn expect_index(&mut self) -> Result<usize, Box<dyn IError>> {
        match self.read_last_result()? {
            Value::I64(i) if i >= 0 => Ok(i as usize),

            other => Err(Box::new(InterpreterError::expected_found(
                ErrorSeverity::HIGH,
                String::from("Array index must be a non-negative i64."),
                String::from("I64"),
                format!("{}", other.to_type()),
                self.span,
            ))),
        }
    }

    fn evaluate_binary_op<F>(&mut self, lhs: &'a Node<Expression>, rhs: &'a Node<Expression>, op: F) -> Result<(), Box<dyn IError>>
    where
        F: Fn(Value, Value, Span) -> Result<Value, ComputationError>,
    {
        self.visit_expression(lhs)?;
        let left_value = self.read_last_result()?;

        self.visit_expression(rhs)?;
        let right_value = self.read_last_result()?;

        let value = op(left_value, right_value, self.span)
            .map_err(|err| Box::new(ComputationError::at(ErrorSeverity::HIGH, err.message(), self.span)) as Box<dyn IError>)?;

        self.last_result = Some(value);

        Ok(())
    }

    fn evaluate_unary_op<F>(&mut self, value: &'a Node<Expression>, op: F) -> Result<(), Box<dyn IError>>
    where
        F: Fn(Value, Span) -> Result<Value, ComputationError>,
    {
        self.visit_expression(value)?;
        let computed_value = self.read_last_result()?;

        let value = op(computed_value, self.span)
            .map_err(|err| Box::new(ComputationError::at(ErrorSeverity::HIGH, err.message(), self.span)) as Box<dyn IError>)?;

        self.last_result = Some(value);

        Ok(())
    }
}
