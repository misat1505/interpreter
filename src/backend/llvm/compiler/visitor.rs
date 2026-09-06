use super::Compiler;
use crate::common::visitor::Visitor;
use crate::{
    backend::llvm::llvm_alu::llvm_value::{LlvmValue, STR_DATA, STR_REFCOUNT},
    common::{
        errors::{CompilerError, ErrorSeverity, IError},
        span::Span,
        types::Type,
    },
    frontend::ast::{Argument, Block, Expression, Literal, Node, Parameter, Program, Statement, SwitchCase, SwitchExpression},
};

impl<'a, 'ctx> Visitor<'a> for Compiler<'a, 'ctx> {
    fn visit_program(&mut self, program: &'a Program) -> Result<(), Box<dyn IError>> {
        self.push_scope();

        for statement in &program.statements {
            self.visit_statement(statement)?;
        }

        self.pop_scope_and_release(Span::default())?;

        Ok(())
    }

    fn visit_expression(&mut self, expression: &'a Node<Expression>) -> Result<(), Box<dyn IError>> {
        self.compile_expression(expression)
    }

    fn visit_statement(&mut self, statement: &'a Node<Statement>) -> Result<(), Box<dyn IError>> {
        self.compile_statement(statement)
    }

    fn visit_parameter(&mut self, _parameter: &'a Node<Parameter>) -> Result<(), Box<dyn IError>> {
        Ok(())
    }

    fn visit_argument(&mut self, _argument: &'a Node<Argument>) -> Result<(), Box<dyn IError>> {
        Ok(())
    }

    fn visit_type(&mut self, _node_type: &'a Node<Type>) -> Result<(), Box<dyn IError>> {
        Ok(())
    }

    /// Every block is its own lexical scope: locals declared directly in it
    /// (owned `Str`/`Vector`/`Struct` values) are automatically released
    /// when the block ends, unless control already left it via a
    /// `return`/`break`/`continue` (which releases scopes itself before
    /// jumping away - see `memory.rs`).
    fn visit_block(&mut self, block: &'a Node<Block>) -> Result<(), Box<dyn IError>> {
        self.push_scope();

        for statement in &block.value.0 {
            self.visit_statement(statement)?;
        }

        self.pop_scope_and_release(block.span)?;

        Ok(())
    }

    fn visit_switch_expression(&mut self, _switch_expression: &'a Node<SwitchExpression>) -> Result<(), Box<dyn IError>> {
        Ok(())
    }

    fn visit_switch_case(&mut self, _switch_case: &'a Node<SwitchCase>) -> Result<(), Box<dyn IError>> {
        Ok(())
    }

    fn visit_literal(&mut self, literal: &'a Literal) -> Result<(), Box<dyn IError>> {
        match literal {
            Literal::I64(value) => {
                let const_value = self.i64_type().const_int(*value as u64, true);
                self.last_value = Some(LlvmValue::I64(const_value));
                Ok(())
            }
            Literal::F64(value) => {
                let const_value = self.f64_type().const_float(*value);
                self.last_value = Some(LlvmValue::F64(const_value));
                Ok(())
            }
            Literal::Char(value) => {
                let const_value = self.context.i8_type().const_int(*value as u64, false);
                self.last_value = Some(LlvmValue::Char(const_value));
                Ok(())
            }
            Literal::String(value) => {
                let err = Self::builder_err(self.span);

                let global = self.builder.build_global_string_ptr(value, "str.lit").map_err(&err)?;

                let len = self.context.i64_type().const_int(value.len() as u64 + 1, false); // +1 na \0

                let malloc_fn = self.libc.malloc_fn;

                let data_ptr = self
                    .builder
                    .build_call(malloc_fn, &[len.into()], "str.lit.heap")
                    .map_err(&err)?
                    .try_as_basic_value()
                    .basic()
                    .expect("malloc returns a pointer value")
                    .into_pointer_value();

                self.builder.build_memcpy(data_ptr, 1, global.as_pointer_value(), 1, len).map_err(&err)?;

                let header_ptr = self.build_str_header(data_ptr, self.span)?;

                self.last_value = Some(LlvmValue::Str(header_ptr));

                Ok(())
            }

            Literal::True => {
                self.last_value = Some(LlvmValue::Bool(self.context.bool_type().const_int(1, false)));
                Ok(())
            }

            Literal::False => {
                self.last_value = Some(LlvmValue::Bool(self.context.bool_type().const_int(0, false)));
                Ok(())
            }
        }
    }

    fn visit_vector_literal(&mut self, _vector: &'a [Box<Node<Expression>>]) -> Result<(), Box<dyn IError>> {
        Err(Box::new(CompilerError::at(
            ErrorSeverity::HIGH,
            String::from("Compiling vector literals is not yet supported."),
            self.span,
        )))
    }

    fn visit_variable(&mut self, variable: &'a str, span: Span) -> Result<(), Box<dyn IError>> {
        let (ptr, var_type) = self.get_variable(variable)?;
        let llvm_type = LlvmValue::type_to_basic_type_enum(&var_type, self.context).ok_or_else(|| {
            Box::new(CompilerError::at(
                ErrorSeverity::HIGH,
                format!("Compiling variables of type '{}' is not yet supported.", var_type),
                span,
            )) as Box<dyn IError>
        })?;

        let raw_value = self
            .builder
            .build_load(llvm_type, ptr, variable)
            .map_err(|err| Box::new(CompilerError::at(ErrorSeverity::HIGH, err.to_string(), span)) as Box<dyn IError>)?;

        self.last_value = Some(LlvmValue::from_basic_value_enum(raw_value, &var_type));

        Ok(())
    }
}

impl<'a, 'ctx> Compiler<'a, 'ctx> {
    /// Allocates a fresh `StrHeader { refcount: 1, data: data_ptr }` around
    /// an already-heap-allocated `data_ptr` C string.
    pub(in crate::backend::llvm::compiler) fn build_str_header(
        &mut self,
        data_ptr: inkwell::values::PointerValue<'ctx>,
        span: Span,
    ) -> Result<inkwell::values::PointerValue<'ctx>, Box<dyn IError>> {
        let err = Self::builder_err(span);
        let header_type = LlvmValue::str_header_type(self.context);
        let header_size = header_type.size_of().expect("StrHeader must have a known size");

        let header_ptr = self
            .builder
            .build_call(self.libc.malloc_fn, &[header_size.into()], "str.header.malloc")
            .map_err(&err)?
            .try_as_basic_value()
            .basic()
            .expect("malloc should return a value")
            .into_pointer_value();

        let rc_field = self
            .builder
            .build_struct_gep(header_type, header_ptr, STR_REFCOUNT, "str.rc")
            .map_err(&err)?;
        self.builder
            .build_store(rc_field, self.context.i64_type().const_int(1, false))
            .map_err(&err)?;

        let data_field = self
            .builder
            .build_struct_gep(header_type, header_ptr, STR_DATA, "str.data")
            .map_err(&err)?;
        self.builder.build_store(data_field, data_ptr).map_err(&err)?;

        Ok(header_ptr)
    }

    /// Loads the `data: i8*` field out of a `StrHeader` pointer.
    pub(in crate::backend) fn str_data_ptr(
        &mut self,
        header_ptr: inkwell::values::PointerValue<'ctx>,
        span: Span,
    ) -> Result<inkwell::values::PointerValue<'ctx>, Box<dyn IError>> {
        let err = Self::builder_err(span);
        let header_type = LlvmValue::str_header_type(self.context);
        let ptr_type = self.context.ptr_type(inkwell::AddressSpace::default());

        let data_field = self
            .builder
            .build_struct_gep(header_type, header_ptr, STR_DATA, "str.data.field")
            .map_err(&err)?;

        let data = self
            .builder
            .build_load(ptr_type, data_field, "str.data.val")
            .map_err(&err)?
            .into_pointer_value();

        Ok(data)
    }
}
