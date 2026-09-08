use crate::{
    common::errors::IError,
    frontend::{
        ast::{Expression, Node},
        lexer::lexer::ILexer,
        parser::Parser,
        tokens::TokenCategory,
    },
};

impl<L: ILexer> Parser<L> {
    pub(in crate::frontend::parser) fn parse_factor(&mut self) -> Result<Option<Node<Expression>>, Box<dyn IError>> {
        // factor = literal | ( "(", expression, ")" ) | vector_literal | identifier_or_call_or_struct_literal_or_enum_literal;
        if let Ok(Some(literal)) = self.parse_literal() {
            let node = Node {
                value: Expression::Literal(literal.value),
                span: literal.span,
            };
            return Ok(Some(node));
        }

        if self.consume_if_matches(TokenCategory::ParenOpen)?.is_some() {
            let expression = self
                .parse_expression()?
                .ok_or_else(|| self.create_parser_error(String::from("Couldn't create expression while parsing nested expression.")))?;

            self.consume_must_be(TokenCategory::ParenClose)?;
            return Ok(Some(expression));
        }

        if let Ok(Some(vector)) = self.parse_vector_literal() {
            return Ok(Some(vector));
        }

        self.identifier_or_call_or_struct_literal_or_enum_literal()
    }
}
