use crate::{
    common::{errors::IError, span::Span},
    frontend::{
        ast::{Node, Parameter, PassedBy},
        lexer::lexer::ILexer,
        parser::{core::try_consume, Parser},
        tokens::TokenCategory,
    },
};

impl<L: ILexer> Parser<L> {
    pub(in crate::frontend::parser) fn parse_parameters(&mut self) -> Result<Vec<Node<Parameter>>, Box<dyn IError>> {
        // parameters = [ parameter, { ",", parameter } ];
        let expression = match self.parse_parameter()? {
            Some(t) => t,
            None => return Ok(vec![]),
        };

        let mut parameters = vec![expression];
        while self.consume_if_matches(TokenCategory::Comma)?.is_some() {
            let parameter = self
                .parse_parameter()?
                .ok_or_else(|| self.create_parser_error(String::from("Couldn't create parameter while parsing parameters.")))?;
            parameters.push(parameter);
        }

        Ok(parameters)
    }

    pub(in crate::frontend::parser) fn parse_parameter(&mut self) -> Result<Option<Node<Parameter>>, Box<dyn IError>> {
        // parameter = ["&"], type, identifier;
        let start_pos = self.current_token().span.start();
        let passed_by = match self.consume_if_matches(TokenCategory::Reference)? {
            Some(_) => PassedBy::Reference,
            None => PassedBy::Value,
        };

        let parameter_type = try_consume!(self, parse_type);
        let identifier = self
            .parse_identifier()?
            .ok_or_else(|| self.create_parser_error(String::from("Couldn't create identifier while parsing parameter.")))?;

        let node = Node {
            value: Parameter {
                passed_by,
                parameter_type,
                identifier: identifier.clone(),
            },
            span: Span::new(start_pos, identifier.span.end()),
        };
        Ok(Some(node))
    }
}
