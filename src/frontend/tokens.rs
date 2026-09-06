use std::fmt::{Debug, Display};

use crate::common::span::Span;

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum TokenCategory {
    // Comparison
    Greater,
    GreaterOrEqual,
    Less,
    LessOrEqual,
    Equal,
    NotEqual,
    // Arithmetic
    Plus,
    Minus,
    Multiply,
    Divide,
    Modulo,
    // Boolean arithmetic
    Negate,
    And,
    Or,
    // Parentheses
    ParenOpen,
    ParenClose,
    BracketOpen,
    BracketClose,
    BraceOpen,
    BraceClose,
    // Keywords
    For,
    While,
    If,
    Else,
    As,
    Fn,
    True,
    False,
    Return,
    Switch,
    Break,
    Continue,
    Import,
    // Type keywords
    Bool,
    String,
    I8,
    I16,
    I32,
    I64,
    U8,
    U16,
    U32,
    U64,
    F64,
    Void,
    Char,
    // Others
    Assign,
    PlusEquals,
    MinusEquals,
    TimesEquals,
    DivideEquals,
    ModuloEquals,
    Colon,
    Semicolon,
    Comma,
    Reference,
    Arrow,
    STX,
    ETX,
    // Complex
    Identifier,
    Comment,
    // Literals
    CharValue,
    StringValue,
    I64Value,
    F64Value,

    Extern,
    Let,
    Struct,
    Dot,
}

impl Display for TokenCategory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use TokenCategory::*;

        let text = match self {
            Greater => ">",
            GreaterOrEqual => ">=",
            Less => "<",
            LessOrEqual => "<=",
            Equal => "==",
            NotEqual => "!=",
            Plus => "+",
            Minus => "-",
            Multiply => "*",
            Divide => "/",
            Modulo => "%",
            Negate => "!",
            And => "&&",
            Or => "||",
            ParenOpen => "(",
            ParenClose => ")",
            BracketOpen => "[",
            BracketClose => "]",
            BraceOpen => "{",
            BraceClose => "}",
            For => "for",
            While => "while",
            If => "if",
            Else => "else",
            As => "as",
            Fn => "fn",
            True => "true",
            False => "false",
            Return => "return",
            Switch => "switch",
            Break => "break",
            Continue => "continue",
            Import => "import",
            Bool => "bool type",
            String => "str type",
            I8 => "i8 type",
            I16 => "i16 type",
            I32 => "i32 type",
            I64 => "i64 type",
            U8 => "u8 type",
            U16 => "u16 type",
            U32 => "u32 type",
            U64 => "u64 type",
            F64 => "f64 type",
            Char => "char type",
            Void => "void",
            Assign => "=",
            PlusEquals => "+=",
            MinusEquals => "-=",
            TimesEquals => "*=",
            DivideEquals => "/=",
            ModuloEquals => "%=",
            Colon => ":",
            Semicolon => ";",
            Comma => ",",
            Reference => "&",
            Arrow => "->",
            STX => "STX",
            ETX => "ETX",
            Identifier => "identifier",
            Comment => "comment",
            StringValue => "str value",
            I64Value => "i64 value",
            F64Value => "f64 value",
            CharValue => "char value",
            Extern => "extern",
            Let => "let",
            Struct => "struct",
            Dot => "dot",
        };

        Ok(write!(f, "{}", text)?)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum TokenValue {
    Char(char),
    String(String),
    F64(f64),
    I64(i64),
    Null,
}

#[derive(Debug, Clone)]
pub struct Token {
    pub category: TokenCategory,
    pub value: TokenValue,
    pub span: Span,
}
