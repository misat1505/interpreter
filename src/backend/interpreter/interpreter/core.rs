use crate::{
    backend::interpreter::{interpreter::Interpreter, Value},
    common::errors::{ErrorSeverity, IError, InterpreterError},
};

impl<'a> Interpreter<'a> {
    pub(in crate::backend::interpreter::interpreter) fn read_last_result(&mut self) -> Result<Value, Box<dyn IError>> {
        {
            let this = self.last_result.take();
            match this {
                Some(v) => Ok(v),
                None => Err(Box::new(InterpreterError::at(
                        ErrorSeverity::HIGH,
                        String::from("No value produced where it is needed."),
                        self.span,
                    ))),
            }
        }
    }
}
