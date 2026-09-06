use crate::{
    common::{errors::IError, span::Span, types::Type},
    frontend::ast::{Argument, Block, Expression, Literal, Node, Parameter, Program, Statement, SwitchCase, SwitchExpression},
};

pub trait Visitor<'a> {
    fn visit_program(&mut self, program: &'a Program) -> Result<(), Box<dyn IError>>;
    fn visit_statement(&mut self, statement: &'a Node<Statement>) -> Result<(), Box<dyn IError>>;
    fn visit_expression(&mut self, expression: &'a Node<Expression>) -> Result<(), Box<dyn IError>>;
    #[allow(dead_code)]
    fn visit_parameter(&mut self, parameter: &'a Node<Parameter>) -> Result<(), Box<dyn IError>>;
    #[allow(dead_code)]
    fn visit_argument(&mut self, argument: &'a Node<Argument>) -> Result<(), Box<dyn IError>>;
    fn visit_type(&mut self, node_type: &'a Node<Type>) -> Result<(), Box<dyn IError>>;
    fn visit_block(&mut self, block: &'a Node<Block>) -> Result<(), Box<dyn IError>>;
    fn visit_switch_expression(&mut self, switch_expression: &'a Node<SwitchExpression>) -> Result<(), Box<dyn IError>>;
    fn visit_switch_case(&mut self, switch_case: &'a Node<SwitchCase>) -> Result<(), Box<dyn IError>>;
    fn visit_literal(&mut self, literal: &'a Literal) -> Result<(), Box<dyn IError>>;
    fn visit_vector_literal(&mut self, vector: &'a [Box<Node<Expression>>]) -> Result<(), Box<dyn IError>>;
    fn visit_variable(&mut self, variable: &'a str, span: Span) -> Result<(), Box<dyn IError>>;
}
