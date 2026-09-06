use crate::{
    common::{errors::IError, span::Span},
    frontend::{
        ast::{Argument, Node, PassedBy},
        lexer::lexer::ILexer,
        parser::{core::try_consume, Parser},
        tokens::TokenCategory,
    },
};

impl<L: ILexer> Parser<L> {
    pub(in crate::frontend::parser) fn parse_arguments(&mut self) -> Result<Vec<Node<Argument>>, Box<dyn IError>> {
        // arguments = [ argument, {",", argument} ];
        let expression = match self.parse_argument()? {
            Some(t) => t,
            None => return Ok(vec![]),
        };

        let mut arguments = vec![expression];
        while self.consume_if_matches(TokenCategory::Comma)?.is_some() {
            let argument = self
                .parse_argument()?
                .ok_or_else(|| self.create_parser_error(String::from("Couldn't create argument while parsing arguments.")))?;

            arguments.push(argument);
        }
        Ok(arguments)
    }

    pub(in crate::frontend::parser) fn parse_argument(&mut self) -> Result<Option<Node<Argument>>, Box<dyn IError>> {
        // argument = ["&"], expression;
        let start_pos = self.current_token().span.start();
        let passed_by = match self.consume_if_matches(TokenCategory::Reference)? {
            Some(_) => PassedBy::Reference,
            None => PassedBy::Value,
        };

        let expression = try_consume!(self, parse_expression);
        let argument = Argument {
            value: expression.clone(),
            passed_by,
        };

        Ok(Some(Node {
            value: argument,
            span: Span::new(start_pos, expression.span.end()),
        }))
    }
}
