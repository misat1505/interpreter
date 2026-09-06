use crate::{
    common::{errors::IError, span::Span},
    frontend::{
        ast::{Expression, Node, StructLiteral, StructLiteralField},
        lexer::lexer::ILexer,
        parser::{core::try_consume, Parser},
        tokens::TokenCategory,
    },
};

impl<L: ILexer> Parser<L> {
    pub(in crate::frontend::parser) fn parse_identifier_or_call_or_struct_literal(&mut self) -> Result<Option<Node<Expression>>, Box<dyn IError>> {
        // identifier_or_call_or_struct_literal = identifier, ( call_or_index_tail | struct_literal_tail );
        let identifier = try_consume!(self, parse_identifier);

        if self.current_token().category == TokenCategory::BraceOpen {
            return Ok(Some(self.parse_struct_literal_tail(identifier)?));
        }

        self.parse_call_or_index_tail(identifier)
    }

    fn parse_call_or_index_tail(&mut self, identifier: Node<String>) -> Result<Option<Node<Expression>>, Box<dyn IError>> {
        // call_or_index_tail = [ "(", arguments, ")" ], { access_tail };
        let start = identifier.span.start();

        let (mut result, mut end) = match self.consume_if_matches(TokenCategory::ParenOpen)? {
            Some(_) => {
                let args = self.parse_arguments()?.into_iter().map(Box::new).collect();
                let paren_close_token = self.consume_must_be(TokenCategory::ParenClose)?;
                (Expression::FunctionCall { identifier, arguments: args }, paren_close_token.span.end())
            }
            None => {
                let end = identifier.span.end();
                (Expression::Variable(identifier.value), end)
            }
        };

        loop {
            match self.current_token().category {
                TokenCategory::BracketOpen => {
                    let _ = self.consume_must_be(TokenCategory::BracketOpen);
                    let index_expr = self
                        .parse_expression()?
                        .ok_or_else(|| self.create_parser_error(String::from("Expected an expression inside '[]' index.")))?;
                    let bracket_close_token = self.consume_must_be(TokenCategory::BracketClose)?;

                    end = bracket_close_token.span.end();

                    result = Expression::Index {
                        collection: Box::new(Node {
                            value: result,
                            span: Span::new(start, index_expr.span.start()),
                        }),
                        index: Box::new(index_expr),
                    };
                }

                TokenCategory::Dot => {
                    let _ = self.consume_must_be(TokenCategory::Dot);
                    let field = self
                        .parse_identifier()?
                        .ok_or_else(|| self.create_parser_error(String::from("Expected an identifier after '.'.")))?;

                    end = field.span.end();

                    result = Expression::FieldAccess {
                        instance: Box::new(Node {
                            value: result,
                            span: Span::new(start, field.span.start()),
                        }),
                        field,
                    };
                }

                _ => break,
            }
        }

        Ok(Some(Node {
            value: result,
            span: Span::new(start, end),
        }))
    }

    fn parse_struct_literal_tail(&mut self, identifier: Node<String>) -> Result<Node<Expression>, Box<dyn IError>> {
        // struct_literal_tail = "{", [ struct_literal_fields ], "}";
        let start = identifier.span.start();

        let _ = self.consume_must_be(TokenCategory::BraceOpen)?;

        let fields = self.parse_struct_literal_fields()?;

        let brace_close_token = self.consume_must_be(TokenCategory::BraceClose)?;

        Ok(Node {
            span: Span::new(start, brace_close_token.span.end()),
            value: Expression::StructLiteral(Node {
                value: StructLiteral { identifier, fields },
                span: Span::new(start, brace_close_token.span.end()),
            }),
        })
    }

    fn parse_struct_literal_fields(&mut self) -> Result<Vec<Node<StructLiteralField>>, Box<dyn IError>> {
        // struct_literal_fields = struct_literal_field, { ",", struct_literal_field };
        let mut fields = Vec::new();

        let Some(first_field) = self.parse_struct_literal_field()? else {
            return Ok(fields);
        };
        fields.push(first_field);

        while self.consume_if_matches(TokenCategory::Comma)?.is_some() {
            let field = self
                .parse_struct_literal_field()?
                .ok_or_else(|| self.create_parser_error(String::from("Expected struct literal field after comma.")))?;

            fields.push(field);
        }

        Ok(fields)
    }

    fn parse_struct_literal_field(&mut self) -> Result<Option<Node<StructLiteralField>>, Box<dyn IError>> {
        // struct_literal_field = identifier, [ ":", expression ];
        let Some(identifier) = self.parse_identifier()? else {
            return Ok(None);
        };

        if self.consume_if_matches(TokenCategory::Colon)?.is_some() {
            let value = self
                .parse_expression()?
                .ok_or_else(|| self.create_parser_error(String::from("Expected expression after ':' in struct literal field.")))?;

            Ok(Some(Node {
                span: Span::new(identifier.span.start(), value.span.end()),
                value: StructLiteralField { identifier, value },
            }))
        } else {
            let span = identifier.span;

            Ok(Some(Node {
                span,
                value: StructLiteralField {
                    identifier: identifier.clone(),
                    value: Node {
                        value: Expression::Variable(identifier.value),
                        span,
                    },
                },
            }))
        }
    }
}
