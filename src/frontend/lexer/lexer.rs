use phf::phf_map;

use crate::{
    common::{
        errors::{ErrorSeverity, IError, LexerError},
        position::Position,
        span::Span,
    },
    frontend::{
        lexer::lazy_stream_reader::{ILazyStreamReader, ETX},
        tokens::{Token, TokenCategory, TokenValue},
    },
};

#[derive(Clone)]
pub struct LexerOptions {
    pub max_comment_length: u32,
    pub max_identifier_length: u32,
}

pub trait ILexer {
    fn current(&self) -> &Option<Token>;
    fn next(&mut self) -> Result<Token, Box<dyn IError>>;
}

pub struct Lexer {
    pub src: Box<dyn ILazyStreamReader>,
    current: Option<Token>,

    position: Position,

    options: LexerOptions,
    on_warning: fn(warning: Box<dyn IError>),
}

impl ILexer for Lexer {
    fn current(&self) -> &Option<Token> {
        &self.current
    }

    fn next(&mut self) -> Result<Token, Box<dyn IError>> {
        self.generate_token()
    }
}

impl Lexer {
    pub fn new(
        src: impl ILazyStreamReader + 'static,
        options: LexerOptions,
        on_warning: fn(warning: Box<dyn IError>),
    ) -> Result<Self, Box<dyn IError>> {
        let position = src.position();

        let mut lexer = Lexer {
            src: Box::new(src),
            current: None,
            position,
            options,
            on_warning,
        };

        lexer.generate_token()?;
        lexer.generate_token()?;

        Ok(lexer)
    }

    #[allow(dead_code)]
    pub fn new_unsafe(
        src: impl ILazyStreamReader + 'static,
        options: LexerOptions,
        on_warning: fn(warning: Box<dyn IError>),
    ) -> Result<Self, Box<dyn IError>> {
        let position = src.position();

        let lexer = Lexer {
            src: Box::new(src),
            current: None,
            position,
            options,
            on_warning,
        };

        Ok(lexer)
    }

    fn current_span(&self) -> Span {
        let start = self.position;
        let end = self.src.position();

        Span::new(start, end)
    }

    fn token_text(token: &Token) -> String {
        match &token.value {
            TokenValue::F64(value) => value.to_string(),
            TokenValue::I64(value) => value.to_string(),
            TokenValue::String(value) => value.clone(),
            TokenValue::Char(value) => value.to_string(),
            TokenValue::Null => format!("{}", token.category),
        }
    }

    #[allow(dead_code)]
    fn consume_must_be(&mut self, category: TokenCategory) -> Result<Token, Box<dyn IError>> {
        let span = self.current_span();

        let current_token = self
            .current
            .take()
            .ok_or_else(|| Box::new(LexerError::at(ErrorSeverity::HIGH, "Expected a token".to_string(), span)) as Box<dyn IError>)?;

        if current_token.category == category {
            self.generate_token()?;
            return Ok(current_token);
        }

        Err(Box::new(LexerError::expected_found(
            ErrorSeverity::HIGH,
            "Unexpected token".to_string(),
            format!("{}", category),
            Self::token_text(&current_token),
            current_token.span,
        )))
    }

    fn handle_etx(&mut self) -> Result<Token, Box<dyn IError>> {
        Ok(self.current.clone().unwrap())
    }

    #[allow(irrefutable_let_patterns)]
    pub fn generate_token(&mut self) -> Result<Token, Box<dyn IError>> {
        self.skip_whitespaces();

        self.position = self.src.position();

        let result_methods = [
            Self::try_generating_sign,
            Self::try_generating_operator,
            Self::try_generating_comment,
            Self::try_generating_string,
            Self::try_generating_char,
            Self::try_generating_number,
            Self::try_creating_identifier_or_keyword,
        ];

        for generator in &result_methods {
            if let Some(token) = generator(self)? {
                match token.category {
                    TokenCategory::ETX => {
                        self.current = Some(token);
                        return self.handle_etx();
                    }

                    TokenCategory::Comment => {
                        return self.generate_token();
                    }

                    _ => {
                        self.current = Some(token.clone());
                        return Ok(token);
                    }
                }
            }
        }

        Err(self.create_lexer_error("Unexpected token".to_string()))
    }

    fn skip_whitespaces(&mut self) {
        while self.src.current().is_whitespace() {
            let _ = self.src.next();
        }
    }

    fn try_generating_comment(&mut self) -> Result<Option<Token>, Box<dyn IError>> {
        let current_char = self.src.current();

        if *current_char != '#' {
            return Ok(None);
        }

        let mut comment = String::new();

        while let Ok(current) = self.src.next() {
            if *current == '\n' || *current == ETX {
                break;
            }

            if (comment.len() as u32) == self.options.max_comment_length {
                return Err(self.create_lexer_error(format!("Comment too long. Max comment length: {}", self.options.max_comment_length)));
            }

            comment.push(*current);
        }

        Ok(Some(Token {
            category: TokenCategory::Comment,
            value: TokenValue::String(comment),
            span: self.current_span(),
        }))
    }

    fn try_generating_sign(&mut self) -> Result<Option<Token>, Box<dyn IError>> {
        let current_char = self.src.current();

        match SIGNS.get(current_char) {
            None => Ok(None),

            Some(token_category) => {
                let span_start = self.position;

                let _ = self.src.next();

                let span = Span::new(span_start, self.src.position());

                let token = Token {
                    category: *token_category,
                    value: TokenValue::Null,
                    span,
                };

                Ok(Some(token))
            }
        }
    }

    fn try_generating_operator(&mut self) -> Result<Option<Token>, Box<dyn IError>> {
        let current_char = self.src.current();

        let token = match current_char {
            '+' => Some(self.extend_to_next('=', TokenCategory::Plus, TokenCategory::PlusEquals)),
            '-' => Some(self.extend_minus()),
            '*' => Some(self.extend_to_next('=', TokenCategory::Multiply, TokenCategory::TimesEquals)),
            '/' => Some(self.extend_to_next('=', TokenCategory::Divide, TokenCategory::DivideEquals)),
            '%' => Some(self.extend_to_next('=', TokenCategory::Modulo, TokenCategory::ModuloEquals)),
            '<' => Some(self.extend_to_next('=', TokenCategory::Less, TokenCategory::LessOrEqual)),
            '>' => Some(self.extend_to_next('=', TokenCategory::Greater, TokenCategory::GreaterOrEqual)),
            '!' => Some(self.extend_to_next('=', TokenCategory::Negate, TokenCategory::NotEqual)),
            '=' => Some(self.extend_to_next('=', TokenCategory::Assign, TokenCategory::Equal)),
            '&' => Some(self.extend_to_next('&', TokenCategory::Reference, TokenCategory::And)),
            '|' => Some(self.extend_to_next_or_warning('|', TokenCategory::Or)),
            '.' => Some(self.single_char(TokenCategory::Dot)),
            _ => None,
        };

        Ok(token)
    }

    fn single_char(&mut self, category: TokenCategory) -> Token {
        let start = self.position;

        let _ = self.src.next();

        let end = self.src.position();

        Token {
            category,
            value: TokenValue::Null,
            span: Span::new(start, end),
        }
    }

    fn extend_to_next(&mut self, char_to_search: char, not_found: TokenCategory, found: TokenCategory) -> Token {
        let start = self.position;

        let next_char = self.src.next().unwrap();

        if *next_char == char_to_search {
            let _ = self.src.next();

            return Token {
                category: found,
                value: TokenValue::Null,
                span: Span::new(start, self.src.position()),
            };
        }

        Token {
            category: not_found,
            value: TokenValue::Null,
            span: Span::new(start, self.src.position()),
        }
    }

    fn extend_to_next_or_warning(&mut self, char_to_search: char, found: TokenCategory) -> Token {
        let start = self.position;

        let next_char = self.src.next().unwrap();

        if *next_char == char_to_search {
            let _ = self.src.next();
        } else {
            let span = Span::new(start, self.src.position());

            (self.on_warning)(Box::new(LexerError::at(
                ErrorSeverity::LOW,
                self.prepare_warning_message_without_span(format!("Expected '{}'", char_to_search)),
                span,
            )));
        }

        Token {
            category: found,
            value: TokenValue::Null,
            span: Span::new(start, self.src.position()),
        }
    }

    fn extend_minus(&mut self) -> Token {
        let start = self.position;

        let next_char = self.src.next().unwrap();

        if *next_char == '>' {
            let _ = self.src.next();

            return Token {
                category: TokenCategory::Arrow,
                value: TokenValue::Null,
                span: Span::new(start, self.src.position()),
            };
        }

        if *next_char == '=' {
            let _ = self.src.next();

            return Token {
                category: TokenCategory::MinusEquals,
                value: TokenValue::Null,
                span: Span::new(start, self.src.position()),
            };
        }

        Token {
            category: TokenCategory::Minus,
            value: TokenValue::Null,
            span: Span::new(start, self.src.position()),
        }
    }

    fn try_generating_char(&mut self) -> Result<Option<Token>, Box<dyn IError>> {
        let start = self.position;

        let current_char = self.src.current();

        if *current_char != '\'' {
            return Ok(None);
        }

        let mut next_char = self.src.next().unwrap();

        if *next_char == '\'' {
            let span = Span::new(start, self.src.position());
            return Err(Box::new(LexerError::at(ErrorSeverity::HIGH, "Empty char literal".to_string(), span)));
        }

        if *next_char == '\n' {
            return Err(self.create_lexer_error("Unexpected newline in char literal".to_string()));
        }

        if *next_char == ETX {
            let span = Span::new(start, self.src.position());
            return Err(Box::new(LexerError::at(
                ErrorSeverity::HIGH,
                "Unexpected end of file in char literal".to_string(),
                span,
            )));
        }

        let value: char;

        if *next_char == '\\' {
            let escaped_char = *self.src.next().unwrap();

            match ESCAPES.get(&escaped_char) {
                Some(escaped) => {
                    value = *escaped;
                }
                None => {
                    let span = Span::new(start, self.src.position());
                    return Err(Box::new(LexerError::at(
                        ErrorSeverity::HIGH,
                        format!("Invalid escape symbol detected '\\{}'", escaped_char),
                        span,
                    )));
                }
            }
        } else {
            value = *next_char;
        }

        next_char = self.src.next().unwrap();

        if *next_char != '\'' {
            let span = Span::new(start, self.src.position());
            return Err(Box::new(LexerError::at(
                ErrorSeverity::HIGH,
                "Char literal must contain exactly one character".to_string(),
                span,
            )));
        }

        let _ = self.src.next();

        Ok(Some(Token {
            category: TokenCategory::CharValue,
            value: TokenValue::Char(value),
            span: Span::new(start, self.src.position()),
        }))
    }

    fn try_generating_string(&mut self) -> Result<Option<Token>, Box<dyn IError>> {
        let start = self.position;

        let mut current_char = *self.src.current();

        if current_char != '"' {
            return Ok(None);
        }

        let mut created_string = String::new();

        current_char = *self.src.next().unwrap();

        while current_char != '"' {
            if current_char == '\\' {
                let next_char = *self.src.next().unwrap();

                match ESCAPES.get(&next_char) {
                    Some(char) => {
                        created_string.push(*char);
                        current_char = *self.src.next().unwrap();
                        continue;
                    }

                    None => {
                        let warning_text = format!("Invalid escape symbol detected '\\{}'", next_char);
                        let span = Span::new(start, self.src.position());

                        (self.on_warning)(Box::new(LexerError::at(ErrorSeverity::LOW, warning_text, span)));

                        created_string.push('\\');
                        current_char = next_char;

                        continue;
                    }
                }
            }

            if current_char == '\n' {
                return Err(self.create_lexer_error("Unexpected newline in string".to_string()));
            }

            if current_char == ETX {
                let span = Span::new(start, self.src.position());

                (self.on_warning)(Box::new(LexerError::at(ErrorSeverity::LOW, "String not closed".to_string(), span)));

                return Ok(Some(Token {
                    category: TokenCategory::StringValue,
                    value: TokenValue::String(created_string),
                    span: Span::new(start, self.src.position()),
                }));
            }

            created_string.push(current_char);

            current_char = *self.src.next().unwrap();
        }

        // consume closing "
        let _ = self.src.next();

        Ok(Some(Token {
            category: TokenCategory::StringValue,
            value: TokenValue::String(created_string),
            span: Span::new(start, self.src.position()),
        }))
    }

    fn try_generating_number(&mut self) -> Result<Option<Token>, Box<dyn IError>> {
        let start = self.position;

        let mut current_char = *self.src.current();

        if !current_char.is_ascii_digit() {
            return Ok(None);
        }

        let mut decimal = 0;

        if current_char != '0' {
            (decimal, _) = self.parse_integer()?;
        } else {
            let next_char = self.src.next().unwrap();

            if next_char.is_ascii_digit() {
                return Err(self.create_lexer_error("Cannot prefix number with 0's.".to_string()));
            }
        }

        current_char = *self.src.current();

        if current_char != '.' {
            return Ok(Some(Token {
                category: TokenCategory::I64Value,
                value: TokenValue::I64(decimal),
                span: Span::new(start, self.src.position()),
            }));
        }

        let _ = self.src.next();

        let (fraction, fraction_length) = self.parse_integer()?;

        let float_value = Self::merge_to_float(decimal, fraction, fraction_length);

        Ok(Some(Token {
            category: TokenCategory::F64Value,
            value: TokenValue::F64(float_value),
            span: Span::new(start, self.src.position()),
        }))
    }

    fn parse_integer(&mut self) -> Result<(i64, i64), Box<dyn IError>> {
        let mut current_char = self.src.current();

        let mut length = 0;
        let mut total: i64 = 0;

        loop {
            if current_char.is_ascii_digit() {
                let digit = *current_char as i64 - '0' as i64;

                total = total
                    .checked_mul(10)
                    .ok_or_else(|| self.create_lexer_error("Overflow occurred while parsing integer".to_string()))?;

                total = total
                    .checked_add(digit)
                    .ok_or_else(|| self.create_lexer_error("Overflow occurred while parsing integer".to_string()))?;

                length += 1;

                current_char = self.src.next().unwrap();

                continue;
            }

            if *current_char == '\'' {
                let next_char = self.src.next().unwrap();

                if !next_char.is_ascii_digit() {
                    return Err(self.create_lexer_error("Digit separator ' must be placed between two digits".to_string()));
                }

                current_char = next_char;

                continue;
            }

            break;
        }

        Ok((total, length))
    }

    fn merge_to_float(decimal: i64, fraction: i64, fraction_length: i64) -> f64 {
        let fraction_value = fraction as f64 / f64::powi(10.0, fraction_length as i32);

        decimal as f64 + fraction_value
    }

    fn try_creating_identifier_or_keyword(&mut self) -> Result<Option<Token>, Box<dyn IError>> {
        let start = self.position;

        let mut current_char = self.src.current();

        if !current_char.is_ascii_alphabetic() {
            return Ok(None);
        }

        let mut created_string = String::new();

        while current_char.is_ascii_digit() || current_char.is_ascii_alphabetic() || *current_char == '_' {
            if (created_string.len() as u32) == self.options.max_identifier_length {
                return Err(self.create_lexer_error(format!(
                    "Identifier name too long. Max identifier length: {}",
                    self.options.max_identifier_length
                )));
            }

            created_string.push(*current_char);

            current_char = self.src.next().unwrap();
        }

        let span = Span::new(start, self.src.position());

        match KEYWORDS.get(created_string.as_str()) {
            Some(category) => Ok(Some(Token {
                category: *category,
                value: TokenValue::Null,
                span,
            })),

            None => Ok(Some(Token {
                category: TokenCategory::Identifier,
                value: TokenValue::String(created_string),
                span,
            })),
        }
    }

    fn create_lexer_error(&mut self, text: String) -> Box<dyn IError> {
        let end = self.src.position();

        let span = Span::new(self.position, end);

        Box::new(LexerError::at(ErrorSeverity::HIGH, text, span))
    }

    fn prepare_warning_message_without_span(&self, text: String) -> String {
        text
    }
}

static SIGNS: phf::Map<char, TokenCategory> = phf_map! {
    '('     => TokenCategory::ParenOpen,
    ')'     => TokenCategory::ParenClose,
    '['     => TokenCategory::BracketOpen,
    ']'     => TokenCategory::BracketClose,
    '{'     => TokenCategory::BraceOpen,
    '}'     => TokenCategory::BraceClose,
    ';'     => TokenCategory::Semicolon,
    ':'     => TokenCategory::Colon,
    ','     => TokenCategory::Comma,
    '\u{2}' => TokenCategory::STX,
    '\u{3}' => TokenCategory::ETX,
};

static KEYWORDS: phf::Map<&'static str, TokenCategory> = phf_map! {
    "fn" => TokenCategory::Fn,
    "for" => TokenCategory::For,
    "while" => TokenCategory::While,
    "if" => TokenCategory::If,
    "else" => TokenCategory::Else,
    "return" => TokenCategory::Return,
    "i8" => TokenCategory::I8,
    "i16" => TokenCategory::I16,
    "i32" => TokenCategory::I32,
    "i64" => TokenCategory::I64,
    "u8" => TokenCategory::U8,
    "u16" => TokenCategory::U16,
    "u32" => TokenCategory::U32,
    "u64" => TokenCategory::U64,
    "f64" => TokenCategory::F64,
    "str" => TokenCategory::String,
    "char" => TokenCategory::Char,
    "void" => TokenCategory::Void,
    "bool" => TokenCategory::Bool,
    "true" => TokenCategory::True,
    "false" => TokenCategory::False,
    "as" => TokenCategory::As,
    "switch" => TokenCategory::Switch,
    "break" => TokenCategory::Break,
    "continue" => TokenCategory::Continue,
    "import" => TokenCategory::Import,
    "extern" => TokenCategory::Extern,
    "let" => TokenCategory::Let,
    "struct" => TokenCategory::Struct,
};

static ESCAPES: phf::Map<char, char> = phf_map! {
    'n'  => '\n',
    'r'  => '\r',
    't'  => '\t',
    '"'  => '"',
    '\\' => '\\',
    'e'  => '\x1b',
    '0'  => '\0',
};
