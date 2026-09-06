use crate::common::{position::Position, span::Span, types::Type};
use std::fmt::Debug;

pub trait IError: Debug {
    fn message(&self) -> String;

    #[allow(dead_code)]
    fn set_message(&mut self, text: String);

    fn get_severity(&self) -> ErrorSeverity;

    fn get_span(&self) -> Span;

    fn expected_found(level: ErrorSeverity, summary: String, expected: String, found: String, span: Span) -> Self
    where
        Self: Sized;

    fn at(level: ErrorSeverity, summary: String, span: Span) -> Self
    where
        Self: Sized;

    fn get_stderr_message(&self) -> String;
}

#[derive(Debug, Clone)]
pub enum ErrorSeverity {
    HIGH, // can't continue execution
    LOW,  // can continue execution
}

macro_rules! define_error {
    ($name:ident) => {
        #[derive(Debug, Clone)]
        pub struct $name {
            _message: String,
            _level: ErrorSeverity,
            _span: Span,
        }

        impl $name {
            pub fn new(level: ErrorSeverity, message: String, span: Span) -> Self {
                $name {
                    _message: message,
                    _level: level,
                    _span: span,
                }
            }
        }

        impl IError for $name {
            fn message(&self) -> String {
                self._message.clone()
            }

            fn set_message(&mut self, text: String) {
                self._message = text;
            }

            fn get_severity(&self) -> ErrorSeverity {
                self._level.clone()
            }

            fn get_span(&self) -> Span {
                self._span
            }

            fn expected_found(level: ErrorSeverity, summary: String, expected: String, found: String, span: Span) -> Self {
                let message = format!("{}\n  expected: {}\n  found:    {}", summary, expected, found,);

                $name::new(level, message, span)
            }

            fn at(level: ErrorSeverity, summary: String, span: Span) -> Self {
                $name::new(level, summary, span)
            }

            fn get_stderr_message(&self) -> String {
                let start = self._span.start();
                let end = self._span.end();

                let mut output = format!("{}: {}", severity_to_string(&self._level), self._message);

                let Some(filename) = start.filename else {
                    return output;
                };

                let Ok(source) = std::fs::read_to_string(filename) else {
                    return output;
                };

                let lines: Vec<&str> = source.lines().collect();

                let start_line = start.line.saturating_sub(1) as usize;

                let Some(source_line) = lines.get(start_line) else {
                    return output;
                };

                let start_column = start.column.saturating_sub(1) as usize;

                let end_column = if start.line == end.line {
                    end.column.saturating_sub(1) as usize
                } else {
                    source_line.len()
                };

                let underline_length = end_column.saturating_sub(start_column).max(1);

                let underline = format!("{}{}{}", severity_color(&self._level), "^".repeat(underline_length), RESET);

                output.push_str(&format!(
                    "\n  --> {}:{}:{}\n\n{:>4} | {}\n     | {}{}",
                    filename,
                    start.line,
                    start.column,
                    start.line,
                    source_line,
                    " ".repeat(start_column),
                    underline,
                ));

                output
            }
        }
    };
}

define_error!(LexerError);
define_error!(ParserError);
define_error!(SemanticCheckerError);
define_error!(InterpreterError);
define_error!(ComputationError);
define_error!(ScopeManagerError);
define_error!(StackOverflowError);
define_error!(StdFunctionError);
define_error!(CompilerError);

#[allow(dead_code)]
pub struct ErrorsManager;

impl ErrorsManager {
    #[allow(dead_code)]
    pub fn append_position(mut error: Box<dyn IError>, position: Position) -> Box<dyn IError> {
        error.set_message(format!("{}\nAt {}.", error.message(), position));

        error
    }
}

const RED: &str = "\x1b[1;31m";
const YELLOW: &str = "\x1b[1;33m";
const RESET: &str = "\x1b[0m";

pub fn severity_to_string(severity: &ErrorSeverity) -> String {
    match severity {
        ErrorSeverity::HIGH => format!("{}error{}", RED, RESET),
        ErrorSeverity::LOW => format!("{}warning{}", YELLOW, RESET),
    }
}

fn severity_color(severity: &ErrorSeverity) -> &'static str {
    match severity {
        ErrorSeverity::HIGH => RED,
        ErrorSeverity::LOW => YELLOW,
    }
}

impl SemanticCheckerError {
    pub fn type_mismatch(level: ErrorSeverity, summary: String, expected: &Type, found: &Type, span: Span) -> Self {
        Self::expected_found(level, summary, format!("{}", expected), format!("{}", found), span)
    }
}
