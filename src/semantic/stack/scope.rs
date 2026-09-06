use crate::common::{
    errors::{ErrorSeverity, ScopeManagerError},
    span::Span,
    types::Type,
};
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub(in crate::semantic::stack) struct VariableInfo {
    pub ty: Type,
    pub span: Span,
    pub used: bool,
}

#[derive(Debug, Clone)]
pub(in crate::semantic::stack) struct StaticCheckerScope<'a> {
    variables: HashMap<&'a str, VariableInfo>,
}

impl<'a> StaticCheckerScope<'a> {
    pub(in crate::semantic::stack) fn new() -> Self {
        StaticCheckerScope { variables: HashMap::new() }
    }

    pub(in crate::semantic::stack) fn get_variable(&self, searched: &'a str) -> Option<&VariableInfo> {
        self.variables.get(searched)
    }

    pub(in crate::semantic::stack) fn get_variable_mut(&mut self, searched: &'a str) -> Option<&mut VariableInfo> {
        self.variables.get_mut(searched)
    }

    pub(in crate::semantic::stack) fn unused_variables(&self) -> Vec<(&'a str, Span)> {
        self.variables
            .iter()
            .filter(|(_, info)| !info.used)
            .map(|(name, info)| (*name, info.span))
            .collect()
    }

    pub(in crate::semantic::stack) fn assign_variable(&mut self, name: &'a str, value: Type, span: Span) -> Result<(), ScopeManagerError> {
        let current_value_option = self.get_variable(name);
        match current_value_option {
            None => Err(ScopeManagerError::new(
                ErrorSeverity::HIGH,
                format!("Variable '{}' not declared.", name),
                span,
            )),
            Some(prev) => {
                if prev.ty == value {
                    // Assignment alone does NOT mark the variable as used.
                    Ok(())
                } else {
                    Err(ScopeManagerError::new(
                        ErrorSeverity::HIGH,
                        format!(
                            "Cannot assign '{}' to variable '{}' which was previously declared as '{}'.",
                            value, name, prev.ty
                        ),
                        span,
                    ))
                }
            }
        }
    }

    pub(in crate::semantic::stack) fn declare_variable(&mut self, name: &'a str, value: Type, span: Span) -> Result<(), ScopeManagerError> {
        match self.get_variable(name) {
            Some(_) => Err(ScopeManagerError::new(
                ErrorSeverity::HIGH,
                format!("Cannot redeclare variable '{}'.", name),
                span,
            )),
            None => {
                self.variables.insert(
                    name,
                    VariableInfo {
                        ty: value,
                        span,
                        used: false,
                    },
                );
                Ok(())
            }
        }
    }
}
