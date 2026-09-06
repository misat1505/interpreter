mod control_flow;
mod core;
mod expressions;
mod functions;
mod memory;
mod statements;
mod stringify;
mod utils;
mod vectors;
mod visitor;

#[cfg(test)]
mod tests;

use std::collections::HashMap;

use inkwell::basic_block::BasicBlock;
use inkwell::builder::Builder;
use inkwell::context::Context;
use inkwell::module::Module;
use inkwell::types::{FloatType, IntType};
use inkwell::values::{FunctionValue, PointerValue};

use crate::backend::llvm::llvm_alu::{LlvmAlu, OverflowPolicy};
use crate::common::span::Span;
use crate::{
    backend::llvm::{libc_functions::LibcFunctions, llvm_alu::llvm_value::LlvmValue},
    common::{
        errors::{CompilerError, ErrorSeverity, IError},
        types::Type,
    },
    frontend::ast::Program,
};

#[derive(Clone, Copy)]
pub(in crate::backend::llvm::compiler) enum ControlFrame<'ctx> {
    Loop {
        continue_block: BasicBlock<'ctx>,
        break_block: BasicBlock<'ctx>,
        /// Number of active lexical scopes (`Compiler::scopes.len()`) at the
        /// point the loop was entered, i.e. *before* the loop body's own
        /// block scope was pushed. `break`/`continue` release every scope
        /// from this depth onward, since those scopes only exist for the
        /// duration of the loop.
        scope_depth: usize,
    },
    Switch {
        break_block: BasicBlock<'ctx>,
        /// See `Loop::scope_depth`.
        scope_depth: usize,
    },
}

/// One entry in a lexical scope: a locally-declared variable that the
/// refcounting runtime must release when the scope it was declared in is
/// exited (normally, or via `return`/`break`/`continue`).
pub(in crate::backend::llvm::compiler) type ScopedVariable<'ctx> = (String, PointerValue<'ctx>, Type);

pub struct Compiler<'a, 'ctx> {
    program: &'a Program,
    context: &'ctx Context,
    module: Module<'ctx>,
    builder: Builder<'ctx>,

    main_fn: Option<FunctionValue<'ctx>>,
    functions: HashMap<String, FunctionValue<'ctx>>,
    libc: LibcFunctions<'ctx>,
    control_stack: Vec<ControlFrame<'ctx>>,

    variables: HashMap<String, (PointerValue<'ctx>, Type)>,

    /// Stack of lexical scopes currently active in the function being
    /// compiled. Each entry is the list of variables declared directly in
    /// that scope, in declaration order. Used by the refcounting runtime to
    /// automatically release owned (`Str`/`Vector`/`Struct`) locals when a
    /// block, function, loop iteration, or switch case is exited.
    scopes: Vec<Vec<ScopedVariable<'ctx>>>,

    last_value: Option<LlvmValue<'ctx>>,

    span: Span,
    llvm_alu: LlvmAlu,
}

impl<'a, 'ctx> Compiler<'a, 'ctx> {
    pub fn new(program: &'a Program, context: &'ctx Context, overflow_policy: OverflowPolicy) -> Self {
        let module = context.create_module("main_module");
        let builder = context.create_builder();
        let libc = LibcFunctions::new(context, &module);

        let span = Span::default();

        let llvm_alu = LlvmAlu::new(overflow_policy);

        Compiler {
            program,
            context,
            module,
            builder,
            main_fn: None,
            functions: HashMap::new(),
            libc,
            control_stack: vec![],
            variables: HashMap::new(),
            scopes: vec![],
            last_value: None,
            span,
            llvm_alu,
        }
    }

    pub(in crate::backend::llvm::compiler) fn i64_type(&self) -> IntType<'ctx> {
        self.context.i64_type()
    }

    pub(in crate::backend::llvm::compiler) fn i32_type(&self) -> IntType<'ctx> {
        self.context.i32_type()
    }

    pub(in crate::backend::llvm::compiler) fn f64_type(&self) -> FloatType<'ctx> {
        self.context.f64_type()
    }

    pub fn read_last_value(&mut self) -> Result<LlvmValue<'ctx>, Box<dyn IError>> {
        self.last_value.take().ok_or_else(|| {
            Box::new(CompilerError::at(
                ErrorSeverity::HIGH,
                String::from("No value produced where it is needed."),
                self.span,
            )) as Box<dyn IError>
        })
    }

    pub fn builder(&self) -> &Builder<'ctx> {
        &self.builder
    }

    pub fn libc(&self) -> &LibcFunctions<'ctx> {
        &self.libc
    }

    pub fn get_variable(&self, name: &str) -> Result<(PointerValue<'ctx>, Type), Box<dyn IError>> {
        self.variables.get(name).cloned().ok_or_else(|| {
            Box::new(CompilerError::at(
                ErrorSeverity::HIGH,
                format!("Undeclared variable '{}'.", name),
                self.span,
            )) as Box<dyn IError>
        })
    }

    pub(in crate::backend::llvm::compiler) fn current_function(&self) -> FunctionValue<'ctx> {
        self.builder
            .get_insert_block()
            .expect("builder should be positioned inside a function")
            .get_parent()
            .expect("basic block should belong to a function")
    }

    pub fn context(&self) -> &'ctx Context {
        self.context
    }

    pub fn set_last_value(&mut self, value: LlvmValue<'ctx>) {
        self.last_value = Some(value);
    }

    pub(in crate::backend::llvm::compiler) fn builder_err(span: Span) -> impl Fn(inkwell::builder::BuilderError) -> Box<dyn IError> {
        move |err| Box::new(CompilerError::at(ErrorSeverity::HIGH, err.to_string(), span)) as Box<dyn IError>
    }
}
