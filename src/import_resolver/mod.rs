use std::{
    collections::HashMap,
    fs::File,
    io::BufReader,
    path::{Path, PathBuf},
};

use crate::{
    backend::std_functions::std_functions::get_std_functions,
    common::{
        errors::{ErrorSeverity, IError, ParserError},
        span::Span,
    },
    frontend::{
        ast::{Node, Program, Statement},
        lexer::{
            lazy_stream_reader::LazyStreamReader,
            lexer::{Lexer, LexerOptions},
        },
        parser::{resolve_declared_types::resolve_declared_types, IParser, Parser},
    },
};

#[cfg(test)]
mod tests;

pub struct ImportResolver {
    stack: Vec<String>,
    visited: HashMap<String, ()>,

    lexer_options: LexerOptions,
    on_warning: fn(warning: Box<dyn IError>),
}

impl ImportResolver {
    pub fn new(lexer_options: LexerOptions, on_warning: fn(warning: Box<dyn IError>)) -> Self {
        ImportResolver {
            stack: Vec::new(),
            visited: HashMap::new(),
            lexer_options,
            on_warning,
        }
    }

    pub fn resolve(&mut self, entry_path: &str, entry_program: Program) -> Result<Program, Box<dyn IError>> {
        let normalized_entry = normalize_path(Path::new(entry_path)).to_string_lossy().into_owned();

        let mut merged = Program {
            statements: Vec::new(),
            functions: HashMap::new(),
            std_functions: get_std_functions(),
            extern_functions: HashMap::new(),
            declared_types: HashMap::new(),
            types: HashMap::new(),
        };

        self.stack.push(normalized_entry.clone());
        self.visited.insert(normalized_entry.clone(), ());

        self.merge_program(&normalized_entry, entry_program, &mut merged)?;

        self.stack.pop();

        merged.types = resolve_declared_types(&merged.declared_types)?;

        Ok(merged)
    }

    fn merge_program(&mut self, current_path: &str, program: Program, merged: &mut Program) -> Result<(), Box<dyn IError>> {
        for (name, function) in program.functions {
            self.check_collision(&name, function.span, merged)?;
            merged.functions.insert(name, function);
        }

        for (name, extern_function) in program.extern_functions {
            self.check_collision(&name, extern_function.span, merged)?;
            merged.extern_functions.insert(name, extern_function);
        }

        for (name, declared_type) in program.declared_types {
            self.check_collision(&name, declared_type.span, merged)?;
            merged.declared_types.insert(name, declared_type);
        }

        for statement in program.statements {
            match &statement.value {
                Statement::Import { path } => {
                    self.handle_import(current_path, path, statement.span, merged)?;
                }
                _ => merged.statements.push(statement),
            }
        }

        Ok(())
    }

    fn handle_import(&mut self, current_path: &str, path: &Node<String>, span: Span, merged: &mut Program) -> Result<(), Box<dyn IError>> {
        let base_dir = Path::new(current_path).parent().unwrap_or_else(|| Path::new(""));
        let resolved_path = normalize_path(&base_dir.join(&path.value));
        let resolved = resolved_path.to_string_lossy().into_owned();

        if let Some(pos) = self.stack.iter().position(|p| *p == resolved) {
            let cycle_chain = self.stack[pos..].join("\n    ↓\n    ");

            return Err(Box::new(ParserError::new(
                ErrorSeverity::HIGH,
                format!("Cyclic import detected:\n    {}\n    ↓\n    {}\n\n", cycle_chain, resolved),
                span,
            )));
        }

        if self.visited.contains_key(&resolved) {
            return Ok(());
        }

        let sub_program = self.parse_file(&resolved, span)?;

        self.stack.push(resolved.clone());
        self.visited.insert(resolved.clone(), ());

        self.merge_program(&resolved, sub_program, merged)?;

        self.stack.pop();

        Ok(())
    }

    fn parse_file(&self, path: &str, import_span: Span) -> Result<Program, Box<dyn IError>> {
        let file = File::open(path)
            .map_err(|_| Box::new(ParserError::new(ErrorSeverity::HIGH, format!("File '{}' not found.", path), import_span)) as Box<dyn IError>)?;

        let reader = BufReader::new(file);

        let filename: &'static str = Box::leak(path.to_string().into_boxed_str());

        let stream = LazyStreamReader::new(reader, Some(filename));
        let lexer = Lexer::new(stream, self.lexer_options_clone(), self.on_warning)?;

        let mut parser = Parser::new(lexer);
        parser.parse()
    }

    fn lexer_options_clone(&self) -> LexerOptions {
        LexerOptions {
            max_comment_length: self.lexer_options.max_comment_length,
            max_identifier_length: self.lexer_options.max_identifier_length,
        }
    }

    fn check_collision(&self, name: &str, span: Span, merged: &Program) -> Result<(), Box<dyn IError>> {
        if merged.functions.contains_key(name)
            || merged.std_functions.contains_key(name)
            || merged.extern_functions.contains_key(name)
            || merged.declared_types.contains_key(name)
        {
            return Err(Box::new(ParserError::new(
                ErrorSeverity::HIGH,
                format!("Redeclaration of '{}' across imported modules.", name),
                span,
            )));
        }

        Ok(())
    }
}

fn normalize_path(path: &Path) -> PathBuf {
    let mut result = PathBuf::new();
    for component in path.components() {
        match component {
            std::path::Component::ParentDir => {
                result.pop();
            }
            std::path::Component::CurDir => {}
            other => result.push(other.as_os_str()),
        }
    }
    result
}
