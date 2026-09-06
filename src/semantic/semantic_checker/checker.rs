use crate::{
    common::{
        errors::{ErrorSeverity, IError, SemanticCheckerError},
        span::Span,
        types::Type,
        visitor::Visitor,
    },
    frontend::ast::{Expression, FunctionDeclaration, Node, Program},
    semantic::stack::stack::StaticCheckerStack,
};

#[derive(Debug, Clone)]
pub struct HoverInfo {
    pub span: Span,
    pub contents: String,
}

pub struct SemanticChecker<'a> {
    pub(in crate::semantic::semantic_checker) program: &'a Program,
    pub(in crate::semantic::semantic_checker) stack: StaticCheckerStack<'a>,
    pub(in crate::semantic::semantic_checker) last_result: Option<Type>,
    pub errors: Vec<Box<dyn IError>>,
    pub hovers: Vec<HoverInfo>,
    pub(in crate::semantic::semantic_checker) current_function_declaration: Option<FunctionDeclaration>,
}

impl<'a> SemanticChecker<'a> {
    pub fn new(program: &'a Program) -> Result<Self, Box<dyn IError>> {
        Ok(Self {
            program,
            errors: vec![],
            hovers: vec![],
            stack: StaticCheckerStack::new(),
            last_result: None,
            current_function_declaration: None,
        })
    }

    pub fn check(&mut self) {
        let _ = self.visit_program(self.program);
    }

    pub(in crate::semantic::semantic_checker) fn read_last_result(&mut self, span: Span) -> Result<Type, Box<dyn IError>> {
        match self.last_result.take() {
            Some(t) => Ok(t),
            None => {
                let error = SemanticCheckerError::at(ErrorSeverity::HIGH, String::from("Expected a type, but none was produced."), span);

                Err(Box::new(error))
            }
        }
    }

    pub(in crate::semantic::semantic_checker) fn evaluate_binary_op<F>(
        &mut self,
        lhs: &'a Node<Expression>,
        rhs: &'a Node<Expression>,
        op: F,
    ) -> Result<(), Box<dyn IError>>
    where
        F: Fn(Type, Type, Span) -> Result<Type, SemanticCheckerError>,
    {
        self.visit_expression(lhs)?;
        let left_value = self.read_last_result(lhs.span);

        self.visit_expression(rhs)?;
        let right_value = self.read_last_result(rhs.span);

        match (left_value, right_value) {
            (Ok(l), Ok(r)) => match op(l, r, Span::new(lhs.span.start(), rhs.span.end())) {
                Ok(result_type) => self.last_result = Some(result_type),

                Err(err) => {
                    self.errors.push(Box::new(SemanticCheckerError::at(
                        ErrorSeverity::HIGH,
                        err.message(),
                        Span::new(lhs.span.start(), rhs.span.end()),
                    )));

                    self.last_result = None;
                }
            },

            _ => self.last_result = None,
        }

        Ok(())
    }

    pub(in crate::semantic::semantic_checker) fn evaluate_unary_op<F>(&mut self, value: &'a Node<Expression>, op: F) -> Result<(), Box<dyn IError>>
    where
        F: Fn(Type, Span) -> Result<Type, SemanticCheckerError>,
    {
        self.visit_expression(value)?;
        let computed_type = self.read_last_result(value.span);

        match computed_type {
            Ok(t) => match op(t, value.span) {
                Ok(result_type) => self.last_result = Some(result_type),

                Err(err) => {
                    self.errors
                        .push(Box::new(SemanticCheckerError::at(ErrorSeverity::HIGH, err.message(), value.span)));

                    self.last_result = None;
                }
            },

            Err(_) => self.last_result = None,
        }

        Ok(())
    }

    pub(in crate::semantic::semantic_checker) fn unused_variables_in_last_scope_warn(&mut self) {
        for (name, span) in self.stack.unused_variables_in_current_scope() {
            self.errors.push(Box::new(SemanticCheckerError::at(
                ErrorSeverity::LOW,
                format!("Unused variable `{}`.", name),
                span,
            )));
        }
    }
}
