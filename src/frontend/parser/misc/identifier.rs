use crate::{
    common::errors::{ErrorSeverity, IError, ParserError},
    frontend::{
        ast::Node,
        lexer::lexer::ILexer,
        parser::{core::try_consume_token, Parser},
        tokens::{TokenCategory, TokenValue},
    },
};

impl<L: ILexer> Parser<L> {
    pub(in crate::frontend::parser) fn parse_identifier(&mut self) -> Result<Option<Node<String>>, Box<dyn IError>> {
        let token = try_consume_token!(self, TokenCategory::Identifier);

        if let TokenValue::String(name) = token.value {
            let node = Node {
                value: name,
                span: token.span,
            };
            return Ok(Some(node));
        }
        Err(Box::new(ParserError::expected_found(
            ErrorSeverity::HIGH,
            "Wrong token value type".to_string(),
            "str".to_string(),
            format!("{}", token.category),
            token.span,
        )))
    }
}
