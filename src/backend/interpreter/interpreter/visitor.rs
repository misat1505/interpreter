use crate::{
    backend::interpreter::interpreter::Interpreter,
    common::{errors::IError, span::Span, types::Type, visitor::Visitor},
    frontend::ast::{Argument, Block, Expression, Literal, Node, Parameter, Program, Statement, SwitchCase, SwitchExpression},
};

impl<'a> Visitor<'a> for Interpreter<'a> {
    fn visit_program(&mut self, program: &'a Program) -> Result<(), Box<dyn IError>> {
        self.exec_program(program)
    }

    fn visit_expression(&mut self, expression: &'a Node<Expression>) -> Result<(), Box<dyn IError>> {
        self.span = expression.span;
        self.eval_expression(expression)
    }

    fn visit_statement(&mut self, statement: &'a Node<Statement>) -> Result<(), Box<dyn IError>> {
        self.span = statement.span;
        self.exec_statement(statement)
    }

    fn visit_argument(&mut self, argument: &'a Node<Argument>) -> Result<(), Box<dyn IError>> {
        self.visit_expression(&argument.value.value)?;
        Ok(())
    }

    fn visit_block(&mut self, block: &'a Node<Block>) -> Result<(), Box<dyn IError>> {
        self.exec_block(block)
    }

    fn visit_parameter(&mut self, parameter: &'a Node<Parameter>) -> Result<(), Box<dyn IError>> {
        self.visit_type(&parameter.value.parameter_type)?;
        Ok(())
    }

    fn visit_switch_case(&mut self, switch_case: &'a Node<SwitchCase>) -> Result<(), Box<dyn IError>> {
        self.exec_switch_case(switch_case)
    }

    fn visit_switch_expression(&mut self, switch_expression: &'a Node<SwitchExpression>) -> Result<(), Box<dyn IError>> {
        self.exec_switch_expression(switch_expression)
    }

    fn visit_type(&mut self, _node_type: &Node<Type>) -> Result<(), Box<dyn IError>> {
        Ok(())
    }

    fn visit_literal(&mut self, literal: &Literal) -> Result<(), Box<dyn IError>> {
        self.eval_literal(literal)
    }

    fn visit_vector_literal(&mut self, expressions: &'a [Box<Node<Expression>>]) -> Result<(), Box<dyn IError>> {
        self.eval_vector_literal(expressions)
    }

    fn visit_variable(&mut self, variable: &'a str, span: Span) -> Result<(), Box<dyn IError>> {
        self.eval_variable(variable, span)
    }
}
