use std::{cell::RefCell, rc::Rc};

use crate::{
    backend::interpreter::{
        alu::value::Value,
        interpreter::{AbortState, Interpreter},
    },
    common::{
        errors::{ErrorSeverity, IError, InterpreterError},
        span::Span,
        types::Type,
        visitor::Visitor,
    },
    frontend::ast::{Accessor, Block, Expression, Node, Program, Statement, SwitchCase, SwitchExpression, VariableDeclarationKind},
};

impl<'a> Interpreter<'a> {
    pub(in crate::backend::interpreter::interpreter) fn exec_program(&mut self, program: &'a Program) -> Result<(), Box<dyn IError>> {
        for statement in &program.statements {
            self.visit_statement(statement)?;

            if let Some(abort) = &self.abort_state {
                return match abort {
                    AbortState::Break => Err(Box::new(InterpreterError::at(
                        ErrorSeverity::HIGH,
                        String::from("Break called outside 'for' or 'switch'."),
                        self.span,
                    ))),

                    AbortState::Continue => Err(Box::new(InterpreterError::at(
                        ErrorSeverity::HIGH,
                        String::from("Continue called outside 'for' or 'while'."),
                        self.span,
                    ))),

                    AbortState::Return => Err(Box::new(InterpreterError::at(
                        ErrorSeverity::HIGH,
                        String::from("Return called outside a function."),
                        self.span,
                    ))),
                };
            }
        }

        Ok(())
    }

    pub(in crate::backend::interpreter::interpreter) fn exec_statement(&mut self, statement: &'a Node<Statement>) -> Result<(), Box<dyn IError>> {
        match &statement.value {
            Statement::Import { .. } => unreachable!("Imports have to be resolved before interpretation"),
            Statement::FunctionCall { identifier, arguments } => {
                self.call_function(identifier, arguments, statement.span)?;
            }

            Statement::Declaration { identifier, kind } => match kind {
                VariableDeclarationKind::TYPE { var_type, value } => {
                    self.visit_type(var_type)?;

                    let mut computed_value = match value {
                        Some(val) => {
                            self.visit_expression(val)?;

                            self.read_last_result().map_err(|_| {
                                Box::new(InterpreterError::at(
                                    ErrorSeverity::HIGH,
                                    format!("Cannot declare variable '{}' with no value.", identifier.value),
                                    statement.span,
                                )) as Box<dyn IError>
                            })?
                        }

                        None => Value::default_value(&var_type.value, statement.span)
                            .map_err(|err| Box::new(InterpreterError::at(ErrorSeverity::HIGH, err.message(), statement.span)) as Box<dyn IError>)?,
                    };

                    match (&var_type.value, &computed_value) {
                        (Type::I8, Value::I8(_))
                        | (Type::I16, Value::I16(_))
                        | (Type::I32, Value::I32(_))
                        | (Type::I64, Value::I64(_))
                        | (Type::U8, Value::U8(_))
                        | (Type::U16, Value::U16(_))
                        | (Type::U32, Value::U32(_))
                        | (Type::U64, Value::U64(_))
                        | (Type::F64, Value::F64(_))
                        | (Type::Str, Value::String(_))
                        | (Type::Char, Value::Char(_))
                        | (Type::Bool, Value::Bool(_)) => {}

                        (Type::Vector(declared_inner), Value::Vector { values, .. }) => {
                            for value in values.borrow().iter() {
                                let actual_type = value.borrow().to_type();

                                if actual_type != *declared_inner.as_ref() {
                                    return Err(Box::new(InterpreterError::expected_found(
                                        ErrorSeverity::HIGH,
                                        format!("Cannot assign value to vector '{}'.", identifier.value),
                                        format!("{}", declared_inner.as_ref()),
                                        format!("{}", actual_type),
                                        statement.span,
                                    )));
                                }
                            }

                            if let Value::Vector { kind: _, ref values } = computed_value {
                                if values.borrow().is_empty() {
                                    computed_value = Value::Vector {
                                        kind: Box::new(var_type.value.clone()),
                                        values: values.clone(),
                                    };
                                }
                            }
                        }

                        (declared_type, computed_value) => {
                            return Err(Box::new(InterpreterError::expected_found(
                                ErrorSeverity::HIGH,
                                format!("Cannot assign value to variable '{}'.", identifier.value),
                                format!("{}", declared_type),
                                format!("{}", computed_value.to_type()),
                                statement.span,
                            )));
                        }
                    }

                    self.stack
                        .declare_variable(identifier.value.as_str(), Rc::new(RefCell::new(computed_value)), statement.span)
                        .map_err(|err| Box::new(InterpreterError::at(ErrorSeverity::HIGH, err.message(), statement.span)) as Box<dyn IError>)?;
                }

                VariableDeclarationKind::LET { var_type, value } => {
                    self.visit_expression(value)?;

                    let mut computed_value = self.read_last_result().map_err(|_| {
                        Box::new(InterpreterError::at(
                            ErrorSeverity::HIGH,
                            format!("Cannot declare variable '{}' with no value.", identifier.value),
                            statement.span,
                        )) as Box<dyn IError>
                    })?;

                    if let Value::Vector { ref kind, ref values } = computed_value {
                        if values.borrow().is_empty() {
                            match var_type {
                                Some(var_type) => {
                                    if !matches!(var_type.value, Type::Vector(_)) {
                                        return Err(Box::new(InterpreterError::expected_found(
                                            ErrorSeverity::HIGH,
                                            format!("Cannot assign value to variable '{}'.", identifier.value),
                                            format!("{}", var_type.value),
                                            "empty vector".to_string(),
                                            statement.span,
                                        )));
                                    }

                                    let resolved_var_type = self.resolve_type(&var_type.value);
                                    computed_value = Value::Vector {
                                        kind: Box::new(resolved_var_type.clone()),
                                        values: values.clone(),
                                    };
                                }

                                None => {
                                    let resolved_kind = self.resolve_type(kind);

                                    if !matches!(resolved_kind, Type::Unresolved(_)) {
                                        computed_value = Value::Vector {
                                            kind: Box::new(resolved_kind),
                                            values: values.clone(),
                                        };
                                    } else {
                                        return Err(Box::new(InterpreterError::at(
                                            ErrorSeverity::HIGH,
                                            format!(
                                                "Cannot infer type of empty vector. Consider adding a type annotation, e.g. `let {}: {} = [];`.",
                                                identifier.value,
                                                Type::Vector(Box::new(Type::I64))
                                            ),
                                            statement.span,
                                        )));
                                    }
                                }
                            }
                        }
                    }

                    let resolved_type = computed_value.to_type();

                    if let Some(var_type) = var_type {
                        let resolved_var_type = self.resolve_type(&var_type.value);
                        if resolved_var_type != resolved_type {
                            return Err(Box::new(InterpreterError::expected_found(
                                ErrorSeverity::HIGH,
                                format!("Cannot assign value to variable '{}'.", identifier.value),
                                format!("{}", var_type.value),
                                format!("{}", resolved_type),
                                statement.span,
                            )));
                        }
                    }

                    self.stack
                        .declare_variable(identifier.value.as_str(), Rc::new(RefCell::new(computed_value)), statement.span)
                        .map_err(|err| Box::new(InterpreterError::at(ErrorSeverity::HIGH, err.message(), statement.span)) as Box<dyn IError>)?;
                }
            },

            Statement::Assignment {
                identifier,
                value,
                accessors,
            } => {
                if accessors.is_empty() {
                    self.visit_expression(value)?;

                    let value = self.read_last_result().map_err(|_| {
                        Box::new(InterpreterError::at(
                            ErrorSeverity::HIGH,
                            format!("Cannot assign no value to variable '{}'.", identifier.value),
                            statement.span,
                        )) as Box<dyn IError>
                    })?;

                    self.stack
                        .assign_variable(identifier.value.as_str(), Rc::new(RefCell::new(value)), statement.span)
                        .map_err(|err| Box::new(InterpreterError::at(ErrorSeverity::HIGH, err.message(), statement.span)) as Box<dyn IError>)?;
                } else {
                    self.exec_index_assignment(identifier, accessors, value)?;
                }
            }

            Statement::Conditional {
                condition,
                if_block,
                else_block,
            } => {
                self.visit_expression(condition)?;

                let computed_condition = self.read_last_result()?;

                let boolean_value = computed_condition
                    .try_into_bool(statement.span)
                    .map_err(|_| self.condition_error(computed_condition, "if statement"))?;

                if boolean_value {
                    self.visit_block(if_block)?;
                } else if let Some(else_blk) = else_block {
                    self.visit_block(else_blk)?;
                }
            }

            Statement::WhileLoop { condition, block } => {
                self.visit_expression(condition)?;

                let mut computed_condition = self.read_last_result()?;

                let mut boolean_value = computed_condition
                    .try_into_bool(statement.span)
                    .map_err(|_| self.condition_error(computed_condition, "while statement"))?;

                while boolean_value {
                    self.visit_block(block)?;

                    if let Some(abort) = &self.abort_state {
                        match abort {
                            AbortState::Break => {
                                self.abort_state = None;
                                break;
                            }

                            AbortState::Return => {
                                break;
                            }

                            AbortState::Continue => {
                                self.abort_state = None;
                            }
                        }
                    }

                    self.visit_expression(condition)?;

                    computed_condition = self.read_last_result()?;

                    boolean_value = computed_condition
                        .try_into_bool(statement.span)
                        .map_err(|_| self.condition_error(computed_condition, "while statement"))?;
                }
            }

            Statement::ForLoop {
                declaration,
                condition,
                assignment,
                block,
            } => {
                self.stack.push_scope();

                if let Some(decl) = declaration {
                    self.visit_statement(decl)?;
                }

                self.visit_expression(condition)?;

                let mut computed_condition = self.read_last_result()?;

                let mut boolean_value = computed_condition
                    .try_into_bool(statement.span)
                    .map_err(|_| self.condition_error(computed_condition, "for statement"))?;

                while boolean_value {
                    self.visit_block(block)?;

                    if let Some(abort) = &self.abort_state {
                        match abort {
                            AbortState::Break => {
                                self.abort_state = None;
                                break;
                            }

                            AbortState::Return => {
                                break;
                            }

                            AbortState::Continue => {
                                self.abort_state = None;
                            }
                        }
                    }

                    if let Some(assign) = assignment {
                        self.visit_statement(assign)?;
                    }

                    self.visit_expression(condition)?;

                    computed_condition = self.read_last_result()?;

                    boolean_value = computed_condition
                        .try_into_bool(statement.span)
                        .map_err(|_| self.condition_error(computed_condition, "for statement"))?;
                }

                self.stack.pop_scope();
            }

            Statement::Switch { expressions, cases } => {
                self.stack.push_scope();

                for expr in expressions {
                    self.visit_switch_expression(expr)?;
                }

                for case in cases {
                    self.visit_switch_case(case)?;

                    if let Some(abort) = &self.abort_state {
                        match abort {
                            AbortState::Break => {
                                self.abort_state = None;
                                break;
                            }

                            AbortState::Return => {
                                break;
                            }

                            AbortState::Continue => {
                                break;
                            }
                        }
                    }
                }

                self.stack.pop_scope();
            }

            Statement::Return(value) => {
                let mut returned_value = None;

                if let Some(val) = value {
                    self.visit_expression(val)?;
                    returned_value = Some(self.read_last_result()?);
                }

                self.abort_state = Some(AbortState::Return);
                self.last_result = returned_value;
            }

            Statement::Break => {
                self.abort_state = Some(AbortState::Break);
            }

            Statement::Continue => {
                self.abort_state = Some(AbortState::Continue);
            }
        }

        Ok(())
    }

    pub(in crate::backend::interpreter::interpreter) fn exec_block(&mut self, block: &'a Node<Block>) -> Result<(), Box<dyn IError>> {
        self.stack.push_scope();

        for statement in &block.value.0 {
            if self.abort_state.is_some() {
                break;
            }

            self.visit_statement(statement)?;
        }

        self.stack.pop_scope();

        Ok(())
    }

    pub(in crate::backend::interpreter::interpreter) fn exec_switch_case(
        &mut self,
        switch_case: &'a Node<SwitchCase>,
    ) -> Result<(), Box<dyn IError>> {
        self.visit_expression(&switch_case.value.condition)?;

        let computed_value = self.read_last_result()?;

        let boolean_value = computed_value
            .try_into_bool(switch_case.span)
            .map_err(|_| self.condition_error(computed_value, "switch case"))?;

        if boolean_value {
            self.visit_block(&switch_case.value.block)?;
        }

        Ok(())
    }

    pub(in crate::backend::interpreter::interpreter) fn exec_switch_expression(
        &mut self,
        switch_expression: &'a Node<SwitchExpression>,
    ) -> Result<(), Box<dyn IError>> {
        if let Some(alias) = &switch_expression.value.alias {
            self.visit_expression(&switch_expression.value.expression)?;

            let computed_value = self.read_last_result()?;

            self.stack
                .declare_variable(alias.value.as_str(), Rc::new(RefCell::new(computed_value)), switch_expression.span)
                .map_err(|err| Box::new(InterpreterError::at(ErrorSeverity::HIGH, err.message(), switch_expression.span)) as Box<dyn IError>)?;
        }

        Ok(())
    }

    pub(in crate::backend::interpreter::interpreter) fn condition_error(&self, value: Value, place: &'a str) -> Box<dyn IError> {
        Box::new(InterpreterError::expected_found(
            ErrorSeverity::HIGH,
            format!("Condition in '{}' has to evaluate to a valid boolean.", place),
            format!("{}", Type::Bool),
            format!("{}", value.to_type()),
            self.span,
        ))
    }

    pub(in crate::backend::interpreter::interpreter) fn exec_index_assignment(
        &mut self,
        identifier: &'a Node<String>,
        accessors: &'a [Node<Accessor>],
        value: &'a Node<Expression>,
    ) -> Result<(), Box<dyn IError>> {
        let var_ref = self
            .stack
            .get_variable(identifier.value.as_str(), Span::new(identifier.span.start(), value.span.end()))
            .map_err(|err| Box::new(InterpreterError::at(ErrorSeverity::HIGH, err.message(), identifier.span)) as Box<dyn IError>)?
            .clone();

        let (last_accessor, earlier_accessors) = accessors.split_last().expect("parser guarantees at least one accessor in assignment");

        let mut current_cell: Rc<RefCell<Value>> = var_ref;

        // Dochodzimy do komórki zawierającej obiekt, którego ostatni
        // accessor będzie modyfikowany.
        for accessor in earlier_accessors {
            current_cell = match &accessor.value {
                Accessor::Index(index_expr) => {
                    self.visit_expression(index_expr)?;

                    let idx = self.expect_index()?;

                    let values = {
                        let borrowed = current_cell.borrow();

                        match &*borrowed {
                            Value::Vector { values, .. } => values.clone(),

                            other => {
                                return Err(Box::new(InterpreterError::expected_found(
                                    ErrorSeverity::HIGH,
                                    String::from("Cannot index into this value."),
                                    String::from("Vector"),
                                    format!("{}", other.to_type()),
                                    index_expr.span,
                                )));
                            }
                        }
                    };

                    let borrowed_values = values.borrow();

                    borrowed_values
                        .get(idx)
                        .ok_or_else(|| {
                            Box::new(InterpreterError::at(
                                ErrorSeverity::HIGH,
                                format!("Index {} out of bounds.", idx),
                                index_expr.span,
                            )) as Box<dyn IError>
                        })?
                        .clone()
                }

                Accessor::Field(field) => {
                    let borrowed = current_cell.borrow();

                    match &*borrowed {
                        Value::Struct { fields, .. } => fields
                            .borrow()
                            .get(field.value.as_str())
                            .ok_or_else(|| {
                                Box::new(InterpreterError::at(
                                    ErrorSeverity::HIGH,
                                    format!("Struct has no field `{}`.", field.value),
                                    field.span,
                                )) as Box<dyn IError>
                            })?
                            .clone(),

                        other => {
                            return Err(Box::new(InterpreterError::expected_found(
                                ErrorSeverity::HIGH,
                                String::from("Cannot access a field on this value."),
                                String::from("Struct"),
                                format!("{}", other.to_type()),
                                field.span,
                            )));
                        }
                    }
                }
            };
        }

        // Obliczamy wartość RHS dopiero po przejściu przez wcześniejsze accessors.
        self.visit_expression(value)?;
        let new_value = self.read_last_result()?;

        match &last_accessor.value {
            Accessor::Index(index_expr) => {
                self.visit_expression(index_expr)?;

                let idx = self.expect_index()?;

                let mut borrowed = current_cell.borrow_mut();

                match &mut *borrowed {
                    Value::Vector { values, .. } => {
                        let mut vec_borrowed = values.borrow_mut();

                        let target_cell = vec_borrowed.get_mut(idx).ok_or_else(|| {
                            Box::new(InterpreterError::at(
                                ErrorSeverity::HIGH,
                                format!("Index {} out of bounds.", idx),
                                index_expr.span,
                            )) as Box<dyn IError>
                        })?;

                        *target_cell = Rc::new(RefCell::new(new_value));

                        Ok(())
                    }

                    Value::String(s) => {
                        let ch = match new_value {
                            Value::Char(c) => c,

                            other => {
                                return Err(Box::new(InterpreterError::expected_found(
                                    ErrorSeverity::HIGH,
                                    String::from("Can only assign a `char` into a string index."),
                                    String::from("Char"),
                                    format!("{}", other.to_type()),
                                    value.span,
                                )));
                            }
                        };

                        let mut bytes = std::mem::take(s).into_bytes();

                        let byte = bytes.get_mut(idx).ok_or_else(|| {
                            Box::new(InterpreterError::at(
                                ErrorSeverity::HIGH,
                                format!("Index {} out of bounds.", idx),
                                index_expr.span,
                            )) as Box<dyn IError>
                        })?;

                        *byte = ch as u8;

                        *s = String::from_utf8(bytes).map_err(|_| {
                            Box::new(InterpreterError::at(
                                ErrorSeverity::HIGH,
                                String::from("String index assignment produced invalid UTF-8."),
                                index_expr.span,
                            )) as Box<dyn IError>
                        })?;

                        Ok(())
                    }

                    other => Err(Box::new(InterpreterError::expected_found(
                        ErrorSeverity::HIGH,
                        String::from("Cannot index into this value."),
                        String::from("Vector or Str"),
                        format!("{}", other.to_type()),
                        index_expr.span,
                    ))),
                }
            }

            Accessor::Field(field) => {
                let mut borrowed = current_cell.borrow_mut();

                match &mut *borrowed {
                    Value::Struct { fields, .. } => {
                        let mut fields_borrowed = fields.borrow_mut();

                        let target_cell = fields_borrowed.get_mut(field.value.as_str()).ok_or_else(|| {
                            Box::new(InterpreterError::at(
                                ErrorSeverity::HIGH,
                                format!("Struct has no field `{}`.", field.value),
                                field.span,
                            )) as Box<dyn IError>
                        })?;

                        *target_cell = Rc::new(RefCell::new(new_value));

                        Ok(())
                    }

                    other => Err(Box::new(InterpreterError::expected_found(
                        ErrorSeverity::HIGH,
                        String::from("Cannot access a field on this value."),
                        String::from("Struct"),
                        format!("{}", other.to_type()),
                        field.span,
                    ))),
                }
            }
        }
    }

    pub(in crate::backend::interpreter::interpreter) fn resolve_reference(
        &mut self,
        expression: &'a Node<Expression>,
    ) -> Result<Rc<RefCell<Value>>, Box<dyn IError>> {
        match &expression.value {
            Expression::Variable(var_name) => self
                .stack
                .get_variable(var_name.as_str(), expression.span)
                .map(Rc::clone)
                .map_err(|err| Box::new(InterpreterError::at(ErrorSeverity::HIGH, err.message(), expression.span)) as Box<dyn IError>),

            Expression::Index { collection, index } => {
                let collection_ref = self.resolve_reference(collection)?;

                self.visit_expression(index)?;

                let idx = self.expect_index()?;

                let values = match &*collection_ref.borrow() {
                    Value::Vector { values, .. } => values.clone(),

                    other => {
                        return Err(Box::new(InterpreterError::expected_found(
                            ErrorSeverity::HIGH,
                            String::from("Cannot index into this value."),
                            String::from("Vector"),
                            format!("{}", other.to_type()),
                            collection.span,
                        )));
                    }
                };

                let borrowed = values.borrow();

                let element_cell = borrowed.get(idx).ok_or_else(|| {
                    Box::new(InterpreterError::at(
                        ErrorSeverity::HIGH,
                        format!("Index {} out of bounds.", idx),
                        index.span,
                    )) as Box<dyn IError>
                })?;

                Ok(Rc::clone(element_cell))
            }

            Expression::FieldAccess { instance, field } => {
                let instance_ref = self.resolve_reference(instance)?;

                let borrowed = instance_ref.borrow();

                match &*borrowed {
                    Value::Struct { fields, .. } => {
                        let fields = fields.borrow();

                        let field_cell = fields.get(field.value.as_str()).ok_or_else(|| {
                            Box::new(InterpreterError::at(
                                ErrorSeverity::HIGH,
                                format!("Struct has no field `{}`.", field.value),
                                field.span,
                            )) as Box<dyn IError>
                        })?;

                        Ok(Rc::clone(field_cell))
                    }

                    other => Err(Box::new(InterpreterError::expected_found(
                        ErrorSeverity::HIGH,
                        String::from("Cannot access a field on this value."),
                        String::from("Struct"),
                        format!("{}", other.to_type()),
                        field.span,
                    ))),
                }
            }

            _ => Err(Box::new(InterpreterError::at(
                ErrorSeverity::HIGH,
                String::from("Cannot pass this kind of expression by reference - expected a variable, indexed value, or field."),
                expression.span,
            ))),
        }
    }
}
