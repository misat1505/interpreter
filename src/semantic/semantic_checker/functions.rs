use crate::{
    common::{
        errors::{ErrorSeverity, IError, SemanticCheckerError},
        types::Type,
        visitor::Visitor,
    },
    frontend::ast::{Expression, Node, Parameter, PassedBy, Statement},
    semantic::semantic_checker::{checker::HoverInfo, SemanticChecker},
};

pub(in crate::semantic::semantic_checker) enum FunctionCallType<'a> {
    Statement(&'a Node<Statement>),
    Expression(&'a Node<Expression>),
}

impl<'a> SemanticChecker<'a> {
    pub(in crate::semantic::semantic_checker) fn is_valid_reference_expression(expression: &Expression) -> bool {
        match expression {
            Expression::Variable(_) => true,
            Expression::Index { collection, .. } => Self::is_valid_reference_expression(&collection.value),
            Expression::FieldAccess { instance, .. } => Self::is_valid_reference_expression(&instance.value),
            _ => false,
        }
    }

    pub(in crate::semantic::semantic_checker) fn check_function_call(&mut self, function: FunctionCallType<'a>) {
        match function {
            FunctionCallType::Statement(Node {
                value: Statement::FunctionCall { identifier, arguments },
                span,
            })
            | FunctionCallType::Expression(Node {
                value: Expression::FunctionCall { identifier, arguments },
                span,
            }) => {
                let name = &identifier.value;
                // std function
                if let Some(std_function) = self.program.std_functions.get(name) {
                    if arguments.len() != std_function.params.len() {
                        self.errors.push(Box::new(SemanticCheckerError::expected_found(
                            ErrorSeverity::HIGH,
                            format!("Invalid number of arguments for function `{}`.", name),
                            std_function.params.len().to_string(),
                            arguments.len().to_string(),
                            *span,
                        )));
                    }
                    let mut collected_types: Vec<Type> = vec![];
                    for (idx, argument) in arguments.iter().enumerate() {
                        let _ = self.visit_expression(&argument.value.value);
                        let actual_type = self.read_last_result(argument.span).ok();
                        let expected_passed_by = std_function.passed_by.get(idx).unwrap_or(&PassedBy::Value);
                        if &argument.value.passed_by != expected_passed_by {
                            self.errors.push(Box::new(SemanticCheckerError::expected_found(
                                ErrorSeverity::HIGH,
                                format!(
                                    "Parameter {} of function `{}` expects to be passed by {}, but was passed by {}.",
                                    idx, name, expected_passed_by, argument.value.passed_by
                                ),
                                format!("{}", expected_passed_by),
                                format!("{}", argument.value.passed_by),
                                argument.span,
                            )));
                        }
                        if *expected_passed_by == PassedBy::Reference && !Self::is_valid_reference_expression(&argument.value.value.value) {
                            self.errors.push(Box::new(SemanticCheckerError::at(
                                ErrorSeverity::HIGH,
                                format!(
                                    "Parameter {} of function `{}` must be a variable, index, or field access when passed by reference.",
                                    idx, name
                                ),
                                argument.span,
                            )));
                        }
                        if let Some(t) = actual_type {
                            collected_types.push(t);
                        }
                    }
                    match &std_function.type_check {
                        Some(check_fn) if collected_types.len() == arguments.len() => match check_fn(&collected_types) {
                            Ok(return_type) => self.last_result = Some(return_type),
                            Err(msg) => {
                                self.errors.push(Box::new(SemanticCheckerError::at(ErrorSeverity::HIGH, msg, *span)));
                                self.last_result = None;
                            }
                        },
                        Some(_) => {
                            self.last_result = None;
                        }
                        None => {
                            for idx in 0..collected_types.len() {
                                if let Some(expected) = std_function.params.get(idx) {
                                    let actual = &collected_types[idx];
                                    if !expected.is_compatible(actual) {
                                        self.errors.push(Box::new(SemanticCheckerError::type_mismatch(
                                            ErrorSeverity::HIGH,
                                            format!("Parameter {} of function `{}` has an incompatible type.", idx, name),
                                            expected,
                                            actual,
                                            arguments[idx].span,
                                        )));
                                    }
                                }
                            }
                            self.last_result = Some(std_function.return_type.clone());
                        }
                    }
                    let params_str = std_function
                        .params
                        .iter()
                        .enumerate()
                        .map(|(idx, t)| {
                            let by_ref = std_function.passed_by.get(idx) == Some(&PassedBy::Reference);
                            format!("{}{}", if by_ref { "&" } else { "" }, t)
                        })
                        .collect::<Vec<_>>()
                        .join(", ");
                    self.hovers.push(HoverInfo {
                        contents: format!("```raptor\nfn {}({}): {}\n```", name, params_str, std_function.return_type),
                        span: identifier.span,
                    });
                    return;
                }
                // extern function
                if let Some(function_declaration) = self.program.extern_functions.get(name) {
                    let parameters = &function_declaration.value.parameters;
                    if arguments.len() != parameters.len() {
                        self.errors.push(Box::new(SemanticCheckerError::expected_found(
                            ErrorSeverity::HIGH,
                            format!("Invalid number of arguments for extern function `{}`.", name),
                            parameters.len().to_string(),
                            arguments.len().to_string(),
                            *span,
                        )));
                    }
                    for (idx, argument) in arguments.iter().enumerate() {
                        let _ = self.visit_expression(&argument.value.value);
                        let actual_type = self.read_last_result(argument.span).ok();
                        if let Some(parameter) = parameters.get(idx) {
                            if argument.value.passed_by != parameter.value.passed_by {
                                self.errors.push(Box::new(SemanticCheckerError::expected_found(
                                    ErrorSeverity::HIGH,
                                    format!(
                                        "Parameter `{}` of extern function `{}` expects to be passed by {}, but was passed by {}.",
                                        parameter.value.identifier.value, name, parameter.value.passed_by, argument.value.passed_by
                                    ),
                                    format!("{}", parameter.value.passed_by),
                                    format!("{}", argument.value.passed_by),
                                    argument.span,
                                )));
                            }
                            if parameter.value.passed_by == PassedBy::Reference && !Self::is_valid_reference_expression(&argument.value.value.value) {
                                self.errors.push(Box::new(SemanticCheckerError::at(
                                    ErrorSeverity::HIGH,
                                    format!(
                                        "Parameter `{}` of extern function `{}` must be a variable, index, or field access when passed by reference.",
                                        parameter.value.identifier.value, name
                                    ),
                                    argument.span,
                                )));
                            }
                            let expected_type = self
                                .resolve_type_fully_checked(&parameter.value.parameter_type.value, parameter.value.parameter_type.span)
                                .ok();
                            if let (Some(expected), Some(actual)) = (&expected_type, &actual_type) {
                                if !expected.is_compatible(actual) {
                                    self.errors.push(Box::new(SemanticCheckerError::type_mismatch(
                                        ErrorSeverity::HIGH,
                                        format!(
                                            "Parameter `{}` of extern function `{}` has an incompatible type.",
                                            parameter.value.identifier.value, name
                                        ),
                                        expected,
                                        actual,
                                        argument.span,
                                    )));
                                }
                            }
                        }
                    }
                    self.last_result = self
                        .resolve_type_fully_checked(&function_declaration.value.return_type.value, function_declaration.value.return_type.span)
                        .ok();
                    self.hovers.push(HoverInfo {
                        contents: format!(
                            "```raptor\nextern fn {}({}): {};\n```",
                            name,
                            format_parameters(parameters),
                            function_declaration.value.return_type.value
                        ),
                        span: identifier.span,
                    });
                    return;
                }
                // user function
                if let Some(function_declaration) = self.program.functions.get(name) {
                    let parameters = &function_declaration.value.parameters;
                    if arguments.len() != parameters.len() {
                        self.errors.push(Box::new(SemanticCheckerError::expected_found(
                            ErrorSeverity::HIGH,
                            format!("Invalid number of arguments for function `{}`.", name),
                            parameters.len().to_string(),
                            arguments.len().to_string(),
                            *span,
                        )));
                    }
                    for (idx, argument) in arguments.iter().enumerate() {
                        let _ = self.visit_expression(&argument.value.value);
                        let actual_type = self.read_last_result(argument.span).ok();
                        if let Some(parameter) = parameters.get(idx) {
                            if argument.value.passed_by != parameter.value.passed_by {
                                self.errors.push(Box::new(SemanticCheckerError::expected_found(
                                    ErrorSeverity::HIGH,
                                    format!(
                                        "Parameter `{}` of function `{}` expects to be passed by {}, but was passed by {}.",
                                        parameter.value.identifier.value, name, parameter.value.passed_by, argument.value.passed_by
                                    ),
                                    format!("{}", parameter.value.passed_by),
                                    format!("{}", argument.value.passed_by),
                                    argument.span,
                                )));
                            }
                            if parameter.value.passed_by == PassedBy::Reference && !Self::is_valid_reference_expression(&argument.value.value.value) {
                                self.errors.push(Box::new(SemanticCheckerError::at(
                                    ErrorSeverity::HIGH,
                                    format!(
                                        "Parameter `{}` of function `{}` must be a variable, index, or field access when passed by reference.",
                                        parameter.value.identifier.value, name
                                    ),
                                    argument.span,
                                )));
                            }
                            let _ = self.visit_type(&parameter.value.parameter_type);
                            let expected_type = self
                                .read_last_result(parameter.value.parameter_type.span)
                                .ok()
                                .and_then(|raw| self.resolve_type_fully_checked(&raw, parameter.value.parameter_type.span).ok());
                            if let (Some(expected), Some(actual)) = (&expected_type, &actual_type) {
                                if !expected.is_compatible(actual) {
                                    self.errors.push(Box::new(SemanticCheckerError::type_mismatch(
                                        ErrorSeverity::HIGH,
                                        format!(
                                            "Parameter `{}` of function `{}` has an incompatible type.",
                                            parameter.value.identifier.value, name
                                        ),
                                        expected,
                                        actual,
                                        argument.span,
                                    )));
                                }
                            }
                        }
                    }
                    self.last_result = self
                        .resolve_type_fully_checked(&function_declaration.value.return_type.value, function_declaration.value.return_type.span)
                        .ok();
                    self.hovers.push(HoverInfo {
                        contents: format!(
                            "```raptor\nfn {}({}): {}\n```",
                            name,
                            format_parameters(parameters),
                            function_declaration.value.return_type.value
                        ),
                        span: identifier.span,
                    });
                    return;
                }
                self.errors.push(Box::new(SemanticCheckerError::at(
                    ErrorSeverity::HIGH,
                    format!("Use of undeclared function `{}`.", name),
                    *span,
                )));
            }
            _ => {}
        }
    }
}

fn format_parameters(parameters: &[Node<Parameter>]) -> String {
    parameters
        .iter()
        .map(|p| {
            let by_ref = if p.value.passed_by == PassedBy::Reference { "&" } else { "" };
            format!("{}{} {}", by_ref, p.value.parameter_type.value, p.value.identifier.value)
        })
        .collect::<Vec<_>>()
        .join(", ")
}
