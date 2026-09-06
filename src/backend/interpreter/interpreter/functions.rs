use std::{cell::RefCell, collections::HashMap, rc::Rc};

use crate::{
    backend::{
        interpreter::{
            interpreter::{AbortState, Interpreter},
            Value,
        },
        std_functions::std_functions::StdFunction,
        type_utils::type_accepts_value,
    },
    common::{
        errors::{ErrorSeverity, IError, InterpreterError},
        span::Span,
        types::Type,
        visitor::Visitor,
    },
    frontend::ast::{Argument, FunctionDeclaration, Node, PassedBy},
};

impl<'a> Interpreter<'a> {
    pub(in crate::backend::interpreter::interpreter) fn execute_std_function(
        std_function: &StdFunction,
        arguments: &Vec<Rc<RefCell<Value>>>,
        span: Span,
    ) -> Result<Option<Value>, Box<dyn IError>> {
        (std_function.execute)(arguments, span)
            .map_err(|err| Box::new(InterpreterError::at(ErrorSeverity::HIGH, err.message(), span)) as Box<dyn IError>)
    }

    pub(in crate::backend::interpreter::interpreter) fn call_function(
        &mut self,
        identifier: &Node<String>,
        arguments: &'a [Box<Node<Argument>>],
        span: Span,
    ) -> Result<(), Box<dyn IError>> {
        let name = identifier.value.as_str();

        let mut args: Vec<Rc<RefCell<Value>>> = vec![];

        for arg in arguments {
            match arg.value.passed_by {
                PassedBy::Value => {
                    self.visit_expression(&arg.value.value)?;

                    let value = self.read_last_result()?;

                    match value {
                        Value::Vector { ref kind, ref values } => {
                            let shallow_copy = Rc::new(RefCell::new(values.borrow().iter().map(Rc::clone).collect::<Vec<_>>()));

                            let shallow_copy_vector = Value::Vector {
                                kind: kind.clone(),
                                values: shallow_copy,
                            };

                            args.push(Rc::new(RefCell::new(shallow_copy_vector)));
                        }

                        Value::Struct {
                            ref identifier,
                            ref fields_types,
                            ref fields,
                        } => {
                            let original_fields = fields.borrow();
                            let mut shallow_fields = HashMap::new();

                            for (field_name, field_value) in original_fields.iter() {
                                let copied_field = match &*field_value.borrow() {
                                    Value::Vector { .. } | Value::Struct { .. } => Rc::clone(field_value),
                                    other_value => Rc::new(RefCell::new(other_value.clone())),
                                };

                                shallow_fields.insert(field_name.clone(), copied_field);
                            }

                            drop(original_fields);

                            let shallow_copy_struct = Value::Struct {
                                identifier: identifier.clone(),
                                fields_types: Rc::clone(fields_types),
                                fields: Rc::new(RefCell::new(shallow_fields)),
                            };

                            args.push(Rc::new(RefCell::new(shallow_copy_struct)));
                        }

                        _ => args.push(Rc::new(RefCell::new(value))),
                    }
                }

                PassedBy::Reference => {
                    let reference = self.resolve_reference(&arg.value.value)?;
                    args.push(reference);
                }
            }
        }

        self.last_arguments = args;

        if let Some(std_function) = self.program.std_functions.get(name) {
            if arguments.len() != std_function.params.len() {
                self.last_arguments.clear();

                return Err(Box::new(InterpreterError::expected_found(
                    ErrorSeverity::HIGH,
                    format!("invalid number of arguments for function `{}`", name),
                    std_function.params.len().to_string(),
                    arguments.len().to_string(),
                    span,
                )));
            }

            if let Some(return_value) = Self::execute_std_function(std_function, &self.last_arguments, span)? {
                self.last_result = Some(return_value);
            }

            self.last_arguments.clear();

            return Ok(());
        }

        if let Some(function_declaration) = self.program.functions.get(name) {
            let expected_arguments = function_declaration.value.parameters.len();

            if arguments.len() != expected_arguments {
                self.last_arguments.clear();

                return Err(Box::new(InterpreterError::expected_found(
                    ErrorSeverity::HIGH,
                    format!("invalid number of arguments for function `{}`", name),
                    expected_arguments.to_string(),
                    arguments.len().to_string(),
                    span,
                )));
            }

            self.execute_function(&function_declaration.value, span)?;

            if let Some(AbortState::Return) = self.abort_state {
                self.abort_state = None;
            }

            self.last_arguments.clear();

            return Ok(());
        }

        self.last_arguments.clear();

        Err(Box::new(InterpreterError::at(
            ErrorSeverity::HIGH,
            format!("use of undeclared function `{}`", name),
            span,
        )))
    }

    pub(in crate::backend::interpreter::interpreter) fn execute_function(
        &mut self,
        function_declaration: &'a FunctionDeclaration,
        span: Span,
    ) -> Result<(), Box<dyn IError>> {
        let name = function_declaration.identifier.value.as_str();
        let statements = &function_declaration.block.value.0;

        self.stack
            .push_stack_frame(span)
            .map_err(|err| Box::new(InterpreterError::at(ErrorSeverity::HIGH, err.message(), span)) as Box<dyn IError>)?;

        for idx in 0..self.last_arguments.len() {
            let parameter = function_declaration.parameters.get(idx).ok_or_else(|| {
                Box::new(InterpreterError::at(
                    ErrorSeverity::HIGH,
                    format!("Invalid parameter index {} while calling function '{}'.", idx, name),
                    span,
                )) as Box<dyn IError>
            })?;

            let desired_type = self.resolve_type(&parameter.value.parameter_type.value);
            let param_name = &parameter.value.identifier.value;
            let value = &self.last_arguments[idx];

            if !type_accepts_value(&desired_type, &value.borrow()) {
                self.stack.pop_stack_frame();

                return Err(Box::new(InterpreterError::expected_found(
                    ErrorSeverity::HIGH,
                    format!("Function '{}' parameter '{}': wrong argument type.", name, param_name),
                    format!("{}", desired_type),
                    format!("{}", value.borrow().to_type()),
                    parameter.span,
                )));
            }

            self.stack
                .declare_variable(param_name.as_str(), Rc::clone(value), span)
                .map_err(|err| Box::new(InterpreterError::at(ErrorSeverity::HIGH, err.message(), parameter.span)) as Box<dyn IError>)?;
        }

        for statement in statements {
            if let Some(AbortState::Return) = self.abort_state {
                self.abort_state = None;
                break;
            }

            self.visit_statement(statement)?;

            if let Some(abort) = &self.abort_state {
                match abort {
                    AbortState::Break => {
                        self.stack.pop_stack_frame();

                        return Err(Box::new(InterpreterError::at(
                            ErrorSeverity::HIGH,
                            String::from("Break called outside 'for' or 'switch'."),
                            statement.span,
                        )));
                    }

                    AbortState::Continue => {
                        self.stack.pop_stack_frame();

                        return Err(Box::new(InterpreterError::at(
                            ErrorSeverity::HIGH,
                            String::from("Continue called outside 'for' or 'while'."),
                            statement.span,
                        )));
                    }

                    _ => {}
                }
            }
        }

        let expected_return_type = self.resolve_type(&function_declaration.return_type.value);

        match &self.last_result {
            None if expected_return_type == Type::Void => {}

            Some(value) if type_accepts_value(&expected_return_type, value) => {}

            result => {
                let result_type = match result {
                    None => Type::Void,
                    Some(value) => value.to_type(),
                };

                self.stack.pop_stack_frame();

                return Err(Box::new(InterpreterError::expected_found(
                    ErrorSeverity::HIGH,
                    format!("Bad return type from function '{}'.", name),
                    format!("{}", expected_return_type),
                    format!("{}", result_type),
                    function_declaration.return_type.span,
                )));
            }
        }

        self.stack.pop_stack_frame();

        Ok(())
    }

    pub(in crate::backend::interpreter::interpreter) fn resolve_type(&self, ty: &Type) -> Type {
        match ty {
            Type::Unresolved(name) => self.program.types.get(name).cloned().unwrap_or_else(|| ty.clone()),
            Type::Vector(inner) => Type::Vector(Box::new(self.resolve_type(inner))),
            other => other.clone(),
        }
    }
}
