use crate::{
    common::{errors::IError, span::Span},
    frontend::{
        ast::{Block, MatchArm, Node, Statement},
        lexer::lexer::ILexer,
        parser::{core::try_consume_token, Parser},
        tokens::TokenCategory,
    },
};

impl<L: ILexer> Parser<L> {
    pub(in crate::frontend::parser) fn parse_match_statement(&mut self) -> Result<Option<Node<Statement>>, Box<dyn IError>> {
        // match_statement = "match", "(", expression, ")", "{", match_arms, [ ",", rest_arm ], "}";

        let match_token = try_consume_token!(self, TokenCategory::Match);

        self.consume_must_be(TokenCategory::ParenOpen)?;

        let expression = self
            .parse_expression()?
            .ok_or_else(|| self.create_parser_error(String::from("Expected an expression after 'match ('.")))?;

        self.consume_must_be(TokenCategory::ParenClose)?;
        self.consume_must_be(TokenCategory::BraceOpen)?;

        let match_arms = self.parse_match_arms()?;

        let rest_arm = if self.current_token().category == TokenCategory::Rest {
            Some(
                self.parse_rest_arm_block()?
                    .ok_or_else(|| self.create_parser_error(String::from("Expected a block after 'rest'.")))?,
            )
        } else {
            None
        };

        let brace_close_token = self.consume_must_be(TokenCategory::BraceClose)?;

        let node = Node {
            value: Statement::Match {
                expression,
                match_arms,
                rest_arm,
            },
            span: Span::new(match_token.span.start(), brace_close_token.span.end()),
        };

        Ok(Some(node))
    }

    fn parse_match_arms(&mut self) -> Result<Vec<Node<MatchArm>>, Box<dyn IError>> {
        // match_arms = [ match_arm, { ",", match_arm } ];

        let mut match_arms = Vec::new();

        if self.current_token().category == TokenCategory::BraceClose || self.current_token().category == TokenCategory::Rest {
            return Ok(match_arms);
        }

        match_arms.push(
            self.parse_match_arm()?
                .ok_or_else(|| self.create_parser_error(String::from("Expected a match arm.")))?,
        );

        loop {
            if self.current_token().category == TokenCategory::BraceClose {
                break;
            }

            self.consume_must_be(TokenCategory::Comma)?;

            if self.current_token().category == TokenCategory::Rest {
                break;
            }

            match_arms.push(
                self.parse_match_arm()?
                    .ok_or_else(|| self.create_parser_error(String::from("Expected a match arm after ','.")))?,
            );
        }

        Ok(match_arms)
    }

    fn parse_match_arm(&mut self) -> Result<Option<Node<MatchArm>>, Box<dyn IError>> {
        // match_arm = identifier, "::", identifier, [ "(", identifier, ")" ], statement_block;

        let start = self.current_token().span.start();

        let enum_name = self
            .parse_identifier()?
            .ok_or_else(|| self.create_parser_error(String::from("Expected an enum name in match arm.")))?;

        self.consume_must_be(TokenCategory::DoubleColon)?;

        let variant_name = self
            .parse_identifier()?
            .ok_or_else(|| self.create_parser_error(String::from("Expected an enum variant name in match arm.")))?;

        let variant_value = if self.consume_if_matches(TokenCategory::ParenOpen)?.is_some() {
            let value = self
                .parse_identifier()?
                .ok_or_else(|| self.create_parser_error(String::from("Expected an identifier inside match arm.")))?;

            self.consume_must_be(TokenCategory::ParenClose)?;

            Some(value)
        } else {
            None
        };

        let block = self
            .parse_statement_block()?
            .ok_or_else(|| self.create_parser_error(String::from("Expected a block after match arm.")))?;

        let span = Span::new(start, block.span.end().clone());

        let node = Node {
            value: MatchArm {
                enum_name,
                variant_name,
                variant_value,
                block: block,
            },
            span,
        };

        Ok(Some(node))
    }

    fn parse_rest_arm_block(&mut self) -> Result<Option<Node<Block>>, Box<dyn IError>> {
        // rest_arm = "rest", statement_block;

        self.consume_must_be(TokenCategory::Rest)?;

        self.parse_statement_block()
    }
}
