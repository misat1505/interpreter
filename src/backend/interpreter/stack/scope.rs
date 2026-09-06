use std::{cell::RefCell, collections::HashMap, rc::Rc};

use crate::{
    backend::{interpreter::Value, type_utils::type_accepts_value},
    common::{
        errors::{ErrorSeverity, ScopeManagerError},
        span::Span,
    },
};

#[derive(Debug, Clone)]
pub(in crate::backend::interpreter::stack) struct Scope<'a> {
    pub(in crate::backend::interpreter::stack) variables: HashMap<&'a str, Rc<RefCell<Value>>>,
}

impl<'a> Scope<'a> {
    pub(in crate::backend::interpreter::stack) fn new() -> Self {
        Scope { variables: HashMap::new() }
    }

    pub(in crate::backend::interpreter::stack) fn get_variable(&self, searched: &'a str) -> Option<&Rc<RefCell<Value>>> {
        self.variables.get(searched)
    }

    pub(in crate::backend::interpreter::stack) fn assign_variable(
        &mut self,
        name: &'a str,
        value: Rc<RefCell<Value>>,
        span: Span,
    ) -> Result<(), ScopeManagerError> {
        let current_value_option = self.get_variable(name);

        match current_value_option {
            None => Err(ScopeManagerError::new(
                ErrorSeverity::HIGH,
                format!("Variable '{}' not declared.", name),
                span,
            )),

            Some(prev_val) => {
                let mut prev_val_borrow = prev_val.borrow_mut();
                let new_val_borrow = value.borrow();

                if type_accepts_value(&prev_val_borrow.to_type(), &new_val_borrow) {
                    *prev_val_borrow = new_val_borrow.clone();
                    Ok(())
                } else {
                    Err(ScopeManagerError::new(
                        ErrorSeverity::HIGH,
                        format!(
                            "Cannot assign '{}' to variable '{}' which was previously declared as '{}'.",
                            new_val_borrow.to_type(),
                            name,
                            prev_val_borrow.to_type()
                        ),
                        span,
                    ))
                }
            }
        }
    }

    pub(in crate::backend::interpreter::stack) fn declare_variable(
        &mut self,
        name: &'a str,
        value: Rc<RefCell<Value>>,
        span: Span,
    ) -> Result<(), ScopeManagerError> {
        match self.get_variable(name) {
            Some(_) => Err(ScopeManagerError::new(
                ErrorSeverity::HIGH,
                format!("Cannot redeclare variable '{}'.", name),
                span,
            )),
            None => {
                self.variables.insert(name, value);
                Ok(())
            }
        }
    }
}
