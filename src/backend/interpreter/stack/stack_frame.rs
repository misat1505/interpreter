use std::fmt::Debug;

use crate::backend::interpreter::stack::scope_manager::ScopeManager;

#[derive(Debug, Clone)]
pub(in crate::backend::interpreter::stack) struct StackFrame<'a> {
    pub(in crate::backend::interpreter::stack) scope_manager: ScopeManager<'a>,
}

impl<'a> StackFrame<'a> {
    pub(in crate::backend::interpreter::stack) fn new() -> Self {
        StackFrame {
            scope_manager: ScopeManager::new(),
        }
    }
}
