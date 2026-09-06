use crate::{
    common::errors::IError,
    frontend::{ast::Program, lexer::lexer::ILexer},
};

mod core;
mod expressions;
mod functions;
mod misc;
mod program;
pub mod resolve_declared_types;
mod statements;
mod structs;

#[cfg(test)]
mod tests;

pub struct Parser<L: ILexer> {
    lexer: L,
}

pub trait IParser<L: ILexer> {
    fn new(lexer: L) -> Self;
    fn parse(&mut self) -> Result<Program, Box<dyn IError>>;
}

impl<L: ILexer> IParser<L> for Parser<L> {
    fn new(lexer: L) -> Self {
        Parser { lexer }
    }

    fn parse(&mut self) -> Result<Program, Box<dyn IError>> {
        self.parse_program()
    }
}
