use crate::{
    common::{
        errors::{ErrorSeverity, ScopeManagerError, StackOverflowError},
        position::Position,
        span::Span,
        types::Type,
    },
    semantic::stack::stack_frame::StaticCheckerStackFrame,
};

#[derive(Debug, Clone)]
pub(in crate::semantic) struct StaticCheckerStack<'a>(pub(in crate::semantic::stack) Vec<StaticCheckerStackFrame<'a>>);

impl<'a> StaticCheckerStack<'a> {
    pub(in crate::semantic) fn new() -> Self {
        StaticCheckerStack(vec![StaticCheckerStackFrame::new()])
    }

    pub(in crate::semantic) fn size(&self) -> usize {
        self.0.len()
    }

    pub(in crate::semantic) fn enter_breakable(&mut self) {
        if let Some(last_frame) = self.0.last_mut() {
            last_frame.breakable_count += 1;
        }
    }

    pub(in crate::semantic) fn exit_breakable(&mut self) {
        if let Some(last_frame) = self.0.last_mut() {
            last_frame.breakable_count -= 1;
        }
    }

    pub(in crate::semantic) fn is_in_breakable(&self) -> bool {
        if let Some(last_frame) = self.0.last() {
            return last_frame.breakable_count > 0;
        }
        false
    }

    pub(in crate::semantic) fn enter_continuable(&mut self) {
        if let Some(last_frame) = self.0.last_mut() {
            last_frame.continuable_count += 1;
        }
    }

    pub(in crate::semantic) fn exit_continuable(&mut self) {
        if let Some(last_frame) = self.0.last_mut() {
            last_frame.continuable_count -= 1;
        }
    }

    pub(in crate::semantic) fn is_in_continuable(&self) -> bool {
        if let Some(last_frame) = self.0.last() {
            return last_frame.continuable_count > 0;
        }
        false
    }

    pub(in crate::semantic) fn push_stack_frame(&mut self) -> Result<(), StackOverflowError> {
        if self.0.len() == 500 {
            return Err(StackOverflowError::new(
                ErrorSeverity::HIGH,
                String::from("Stack overflow."),
                Span::new(
                    Position::new(0, 0, 0, Some("<stack_overflow>")),
                    Position::new(0, 0, 0, Some("<stack_overflow>")),
                ),
            ));
        }
        self.0.push(StaticCheckerStackFrame::new());
        Ok(())
    }

    pub(in crate::semantic) fn pop_stack_frame(&mut self) {
        self.0.pop();
    }

    pub(in crate::semantic) fn push_scope(&mut self) {
        if let Some(last_frame) = self.0.last_mut() {
            last_frame.scope_manager.push_scope();
        }
    }

    pub(in crate::semantic) fn pop_scope(&mut self) {
        if let Some(last_frame) = self.0.last_mut() {
            last_frame.scope_manager.pop_scope();
        }
    }

    pub(in crate::semantic) fn unused_variables_in_current_scope(&self) -> Vec<(&'a str, Span)> {
        match self.0.last() {
            Some(last_frame) => last_frame.scope_manager.unused_variables_in_current_scope(),
            None => vec![],
        }
    }

    pub(in crate::semantic) fn get_variable(&mut self, name: &'a str, span: Span) -> Result<&Type, ScopeManagerError> {
        match self.0.last_mut() {
            Some(last_frame) => last_frame.scope_manager.get_variable(name, span),
            None => unreachable!("Scope stack is empty"),
        }
    }

    pub(in crate::semantic) fn get_variable_declaration_span(&mut self, name: &'a str, span: Span) -> Result<&Span, ScopeManagerError> {
        match self.0.last_mut() {
            Some(last_frame) => last_frame.scope_manager.get_variable_declaration_span(name, span),
            None => unreachable!("Scope stack is empty"),
        }
    }

    pub(in crate::semantic) fn assign_variable(&mut self, name: &'a str, value: Type, span: Span) -> Result<(), ScopeManagerError> {
        if let Some(last_frame) = self.0.last_mut() {
            last_frame.scope_manager.assign_variable(name, value, span)?;
        }
        Ok(())
    }

    pub(in crate::semantic) fn declare_variable(&mut self, name: &'a str, value: Type, span: Span) -> Result<(), ScopeManagerError> {
        if let Some(last_frame) = self.0.last_mut() {
            last_frame.scope_manager.declare_variable(name, value, span)?;
        }
        Ok(())
    }
}
