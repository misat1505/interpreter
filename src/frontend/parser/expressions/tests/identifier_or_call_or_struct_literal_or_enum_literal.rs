use crate::frontend::parser::tests::{create_token, LexerMock};
use crate::frontend::{
    ast::{Argument, Expression, Literal, PassedBy},
    parser::{tests::test_node, IParser, Parser},
    tokens::{TokenCategory, TokenValue},
};

#[test]
fn parse_identifier_or_call_fail() {
    let token_series = [
        vec![
            // print(5,)
            create_token(TokenCategory::Identifier, TokenValue::String(String::from("print"))),
            create_token(TokenCategory::ParenOpen, TokenValue::Null),
            create_token(TokenCategory::I64Value, TokenValue::I64(5)),
            create_token(TokenCategory::Comma, TokenValue::Null),
            create_token(TokenCategory::ParenClose, TokenValue::Null),
            create_token(TokenCategory::ETX, TokenValue::Null),
        ],
        vec![
            create_token(
                // print(
                TokenCategory::Identifier,
                TokenValue::String(String::from("print")),
            ),
            create_token(TokenCategory::ParenOpen, TokenValue::Null),
            create_token(TokenCategory::ETX, TokenValue::Null),
        ],
    ];

    for idx in 0..token_series.len() {
        let mock_lexer = LexerMock::new(token_series[idx].clone());
        let mut parser = Parser::new(mock_lexer);

        assert!(parser.identifier_or_call_or_struct_literal_or_enum_literal().is_err());
    }
}

#[test]
fn parse_identifier_or_call() {
    let token_series = [
        vec![
            // print
            create_token(TokenCategory::Identifier, TokenValue::String(String::from("print"))),
            create_token(TokenCategory::ETX, TokenValue::Null),
        ],
        vec![
            // print()
            create_token(TokenCategory::Identifier, TokenValue::String(String::from("print"))),
            create_token(TokenCategory::ParenOpen, TokenValue::Null),
            create_token(TokenCategory::ParenClose, TokenValue::Null),
            create_token(TokenCategory::ETX, TokenValue::Null),
        ],
        vec![
            // print(5)
            create_token(TokenCategory::Identifier, TokenValue::String(String::from("print"))),
            create_token(TokenCategory::ParenOpen, TokenValue::Null),
            create_token(TokenCategory::I64Value, TokenValue::I64(5)),
            create_token(TokenCategory::ParenClose, TokenValue::Null),
            create_token(TokenCategory::ETX, TokenValue::Null),
        ],
        vec![
            // print(5, x)
            create_token(TokenCategory::Identifier, TokenValue::String(String::from("print"))),
            create_token(TokenCategory::ParenOpen, TokenValue::Null),
            create_token(TokenCategory::Reference, TokenValue::Null),
            create_token(TokenCategory::I64Value, TokenValue::I64(5)),
            create_token(TokenCategory::Comma, TokenValue::Null),
            create_token(TokenCategory::Identifier, TokenValue::String(String::from("x"))),
            create_token(TokenCategory::ParenClose, TokenValue::Null),
            create_token(TokenCategory::ETX, TokenValue::Null),
        ],
    ];

    let expected = [
        Expression::Variable(String::from("print")),
        Expression::FunctionCall {
            identifier: test_node!(String::from("print")),
            arguments: vec![],
        },
        Expression::FunctionCall {
            identifier: test_node!(String::from("print")),
            arguments: vec![Box::new(test_node!(Argument {
                value: test_node!(Expression::Literal(Literal::I64(5))),
                passed_by: PassedBy::Value,
            }))],
        },
        Expression::FunctionCall {
            identifier: test_node!(String::from("print")),
            arguments: vec![
                Box::new(test_node!(Argument {
                    value: test_node!(Expression::Literal(Literal::I64(5))),
                    passed_by: PassedBy::Reference,
                })),
                Box::new(test_node!(Argument {
                    value: test_node!(Expression::Variable(String::from("x"))),
                    passed_by: PassedBy::Value,
                })),
            ],
        },
    ];

    for (idx, series) in token_series.iter().enumerate() {
        let mock_lexer = LexerMock::new(series.to_vec());
        let mut parser = Parser::new(mock_lexer);

        let node = parser.identifier_or_call_or_struct_literal_or_enum_literal().unwrap().unwrap();
        assert_eq!(node.value, expected[idx]);
    }
}

#[test]
fn parse_identifier_or_call_with_index() {
    let token_series = [
        vec![
            // x[0]
            create_token(TokenCategory::Identifier, TokenValue::String(String::from("x"))),
            create_token(TokenCategory::BracketOpen, TokenValue::Null),
            create_token(TokenCategory::I64Value, TokenValue::I64(0)),
            create_token(TokenCategory::BracketClose, TokenValue::Null),
            create_token(TokenCategory::ETX, TokenValue::Null),
        ],
        vec![
            // x[0][1]
            create_token(TokenCategory::Identifier, TokenValue::String(String::from("x"))),
            create_token(TokenCategory::BracketOpen, TokenValue::Null),
            create_token(TokenCategory::I64Value, TokenValue::I64(0)),
            create_token(TokenCategory::BracketClose, TokenValue::Null),
            create_token(TokenCategory::BracketOpen, TokenValue::Null),
            create_token(TokenCategory::I64Value, TokenValue::I64(1)),
            create_token(TokenCategory::BracketClose, TokenValue::Null),
            create_token(TokenCategory::ETX, TokenValue::Null),
        ],
    ];

    let expected = [
        Expression::Index {
            collection: Box::new(test_node!(Expression::Variable(String::from("x")))),
            index: Box::new(test_node!(Expression::Literal(Literal::I64(0)))),
        },
        Expression::Index {
            collection: Box::new(test_node!(Expression::Index {
                collection: Box::new(test_node!(Expression::Variable(String::from("x")))),
                index: Box::new(test_node!(Expression::Literal(Literal::I64(0)))),
            })),
            index: Box::new(test_node!(Expression::Literal(Literal::I64(1)))),
        },
    ];

    for (idx, series) in token_series.iter().enumerate() {
        let mock_lexer = LexerMock::new(series.to_vec());
        let mut parser = Parser::new(mock_lexer);

        let node = parser.identifier_or_call_or_struct_literal_or_enum_literal().unwrap().unwrap();
        assert_eq!(node.value, expected[idx]);
    }
}
