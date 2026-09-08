use crate::{
    common::{errors::IError, span::Span, types::Type},
    frontend::{
        ast::{EnumDeclaration, EnumMember, Node},
        lexer::lexer::ILexer,
        parser::{
            core::{try_consume, try_consume_token},
            Parser,
        },
        tokens::TokenCategory,
    },
};

impl<L: ILexer> Parser<L> {
    pub(in crate::frontend::parser) fn parse_enum_declaration(&mut self) -> Result<Option<Node<EnumDeclaration>>, Box<dyn IError>> {
        // enum_declaration = "enum", identifier, "{", [ enum_members ], "}", ";";
        let struct_token = try_consume_token!(self, TokenCategory::Enum);

        let identifier = self
            .parse_identifier()?
            .ok_or_else(|| self.create_parser_error(String::from("Couldn't create identifier while parsing struct declaration.")))?;

        let _ = self.consume_must_be(TokenCategory::BraceOpen)?;
        let members = self.parse_enum_members()?;
        let _ = self.consume_must_be(TokenCategory::BraceClose)?;
        let semicolon_token = self.consume_must_be(TokenCategory::Semicolon)?;

        Ok(Some(Node {
            value: EnumDeclaration { identifier, members },
            span: Span::new(struct_token.span.start(), semicolon_token.span.end()),
        }))
    }

    fn parse_enum_members(&mut self) -> Result<Vec<Node<EnumMember>>, Box<dyn IError>> {
        // enum_members = enum_member, { ",", enum_member };
        let mut members = Vec::new();

        let Some(first_member) = self.parse_enum_member()? else {
            return Ok(members);
        };
        members.push(first_member);

        while self.current_token().category == TokenCategory::Comma {
            let _ = self.consume_must_be(TokenCategory::Comma)?;

            let member = self
                .parse_enum_member()?
                .ok_or_else(|| self.create_parser_error(String::from("Expected enum member after comma.")))?;

            members.push(member);
        }

        Ok(members)
    }

    fn parse_enum_member(&mut self) -> Result<Option<Node<EnumMember>>, Box<dyn IError>> {
        // enum_member = identifier, [ "(", type, ")" ];
        let identifier = try_consume!(self, parse_identifier);

        let mut member_type: Option<Node<Type>> = None;
        let mut end_pos = identifier.span.end();
        if self.consume_if_matches(TokenCategory::ParenOpen)?.is_some() {
            member_type = Some(
                self.parse_type()?
                    .ok_or_else(|| self.create_parser_error(String::from("Couldn't create type while parsing enum member declaration.")))?,
            );
            let paren_close_token = self.consume_must_be(TokenCategory::ParenClose)?;
            end_pos = paren_close_token.span.end();
        }

        Ok(Some(Node {
            value: EnumMember {
                identifier: identifier.clone(),
                member_type: member_type.clone(),
            },
            span: Span::new(identifier.span.start(), end_pos),
        }))
    }
}
