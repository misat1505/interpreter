use crate::{
    common::errors::{ErrorSeverity, IError, ParserError},
    frontend::{
        lexer::lexer::ILexer,
        parser::Parser,
        tokens::{Token, TokenCategory, TokenValue},
    },
};

macro_rules! try_consume_token {
    ($self:ident, $token_category:expr) => {
        match $self.consume_must_be($token_category) {
            Ok(t) => t,
            Err(_) => return Ok(None),
        }
    };
}

macro_rules! try_consume {
    ($self:ident, $method:ident) => {
        match $self.$method()? {
            Some(t) => t,
            None => return Ok(None),
        }
    };
}

pub(in crate::frontend::parser) use try_consume;
pub(in crate::frontend::parser) use try_consume_token;

impl<L: ILexer> Parser<L> {
    pub(in crate::frontend::parser) fn next_token(&mut self) -> Result<Option<Token>, Box<dyn IError>> {
        let mut current_token = self.lexer.next()?;

        while current_token.category == TokenCategory::Comment {
            current_token = self.lexer.next()?;
        }

        Ok(Some(current_token))
    }

    pub(in crate::frontend::parser) fn current_token(&self) -> Token {
        self.lexer.current().clone().unwrap()
    }

    pub(in crate::frontend::parser) fn consume_must_be(&mut self, category: TokenCategory) -> Result<Token, Box<dyn IError>> {
        let current_token = self.current_token();

        if current_token.category == category {
            self.next_token()?;
            return Ok(current_token);
        }

        Err(Box::new(ParserError::expected_found(
            ErrorSeverity::HIGH,
            "Unexpected token".to_string(),
            format!("{}", category),
            Self::token_text(&current_token),
            current_token.span,
        )))
    }

    pub(in crate::frontend::parser) fn consume_if_matches(&mut self, category: TokenCategory) -> Result<Option<Token>, Box<dyn IError>> {
        let current_token = self.current_token();

        if current_token.category == category {
            self.next_token()?;
            return Ok(Some(current_token));
        }

        Ok(None)
    }

    pub(in crate::frontend::parser) fn create_parser_error(&self, text: String) -> Box<dyn IError> {
        Box::new(ParserError::at(ErrorSeverity::HIGH, text, self.current_token().span))
    }

    pub(in crate::frontend::parser) fn token_text(token: &Token) -> String {
        match &token.value {
            TokenValue::F64(value) => value.to_string(),
            TokenValue::I64(value) => value.to_string(),
            TokenValue::String(value) => value.clone(),
            TokenValue::Char(value) => value.to_string(),
            TokenValue::Null => format!("{}", token.category),
        }
    }
}
