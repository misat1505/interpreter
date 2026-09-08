use std::{collections::HashMap, rc::Rc};

use crate::{
    backend::std_functions::std_functions::get_std_functions,
    common::{
        errors::{ErrorSeverity, IError, ParserError},
        span::Span,
    },
    frontend::{
        ast::{DeclaredType, ExternFunctionDeclaration, FunctionDeclaration, Node, Program, Statement},
        lexer::lexer::ILexer,
        parser::{resolve_declared_types::resolve_declared_types, Parser},
        tokens::TokenCategory,
    },
};

impl<L: ILexer> Parser<L> {
    pub(in crate::frontend::parser) fn parse_program(&mut self) -> Result<Program, Box<dyn IError>> {
        // program = { import_declaration | struct_declaration | enum_declaration | function_declaration | extern_function_declaration
        //           | assign_or_call | if_statement | for_statement | while_statement | switch_statement
        //           | declaration, ";" };
        let mut statements: Vec<Node<Statement>> = vec![];
        let mut functions: HashMap<String, Rc<Node<FunctionDeclaration>>> = HashMap::new();
        let std_functions = get_std_functions();
        let mut extern_functions: HashMap<String, Rc<Node<ExternFunctionDeclaration>>> = HashMap::new();
        let mut declared_types: HashMap<String, Rc<Node<DeclaredType>>> = HashMap::new();

        loop {
            if let Some(struct_declaration) = self.parse_struct_declaration()? {
                let type_name = struct_declaration.value.identifier.value.clone();

                Self::check_name_collision(
                    &type_name,
                    struct_declaration.span,
                    &functions,
                    &std_functions,
                    &extern_functions,
                    &declared_types,
                )?;

                let declared_type = Node {
                    value: DeclaredType::Struct(struct_declaration.value),
                    span: struct_declaration.span,
                };

                declared_types.insert(type_name, Rc::new(declared_type));
            } else if let Some(enum_declaration) = self.parse_enum_declaration()? {
                let type_name = enum_declaration.value.identifier.value.clone();

                Self::check_name_collision(
                    &type_name,
                    enum_declaration.span,
                    &functions,
                    &std_functions,
                    &extern_functions,
                    &declared_types,
                )?;

                let declared_type = Node {
                    value: DeclaredType::Enum(enum_declaration.value),
                    span: enum_declaration.span,
                };

                declared_types.insert(type_name, Rc::new(declared_type));
            } else if let Some(statement) = self.parse_program_statement()? {
                statements.push(statement);
            } else if let Some(function_declaration) = self.parse_function_declaration()? {
                let function_name = function_declaration.value.identifier.value.clone();

                Self::check_name_collision(
                    &function_name,
                    function_declaration.span,
                    &functions,
                    &std_functions,
                    &extern_functions,
                    &declared_types,
                )?;

                functions.insert(function_name, Rc::new(function_declaration));
            } else if let Some(function_declaration) = self.parse_extern_function_declaration()? {
                let function_name = function_declaration
                    .value
                    .alias
                    .as_ref()
                    .unwrap_or(&function_declaration.value.identifier)
                    .value
                    .clone();

                Self::check_name_collision(
                    &function_name,
                    function_declaration.span,
                    &functions,
                    &std_functions,
                    &extern_functions,
                    &declared_types,
                )?;

                extern_functions.insert(function_name, Rc::new(function_declaration));
            } else {
                break;
            }
        }

        self.consume_must_be(TokenCategory::ETX)?;

        let types = resolve_declared_types(&declared_types)?;

        Ok(Program {
            statements,
            functions,
            std_functions,
            extern_functions,
            declared_types,
            types,
        })
    }

    fn check_name_collision(
        name: &str,
        span: Span,
        functions: &HashMap<String, Rc<Node<FunctionDeclaration>>>,
        std_functions: &HashMap<String, crate::backend::std_functions::std_functions::StdFunction>,
        extern_functions: &HashMap<String, Rc<Node<ExternFunctionDeclaration>>>,
        declared_types: &HashMap<String, Rc<Node<DeclaredType>>>,
    ) -> Result<(), Box<dyn IError>> {
        if functions.contains_key(name)
            || std_functions.contains_key(name)
            || extern_functions.contains_key(name)
            || declared_types.contains_key(name)
        {
            return Err(Box::new(ParserError::new(
                ErrorSeverity::HIGH,
                format!("Redeclaration of '{}'.", name),
                span,
            )));
        }

        Ok(())
    }

    pub(in crate::frontend::parser) fn parse_program_statement(&mut self) -> Result<Option<Node<Statement>>, Box<dyn IError>> {
        // program_statement = import_declaration | assign_or_call | if_statement | for_statement | while_statement
        //                    | switch_statement | (declaration, ";") | let_declaration | match_statement;
        let generators = [
            Self::parse_assign_or_call,
            Self::parse_if_statement,
            Self::parse_for_statement,
            Self::parse_while_statement,
            Self::parse_switch_statement,
            Self::parse_variable_declaration,
            Self::parse_let_variable_declaration,
            Self::parse_import_declaration,
            Self::parse_match_statement,
        ];

        for generator in &generators {
            if let Some(statement) = generator(self)? {
                return Ok(Some(statement));
            }
        }

        Ok(None)
    }

    pub(in crate::frontend::parser) fn parse_statement(&mut self) -> Result<Option<Node<Statement>>, Box<dyn IError>> {
        let generators = [
            Self::parse_assign_or_call,
            Self::parse_if_statement,
            Self::parse_for_statement,
            Self::parse_while_statement,
            Self::parse_switch_statement,
            Self::parse_return_statement,
            Self::parse_break_statement,
            Self::parse_continue_statement,
            Self::parse_variable_declaration,
            Self::parse_let_variable_declaration,
            Self::parse_match_statement,
        ];

        for generator in &generators {
            if let Some(statement) = generator(self)? {
                return Ok(Some(statement));
            }
        }

        Ok(None)
    }
}
