use std::collections::HashMap;

use inkwell::types::StructType;

use super::Compiler;
use crate::{
    backend::llvm::llvm_alu::llvm_value::LlvmValue,
    common::{
        errors::{CompilerError, ErrorSeverity, IError},
        span::Span,
        types::Type,
    },
    frontend::ast::{DeclaredType, StructDeclaration},
};

impl<'a, 'ctx> Compiler<'a, 'ctx> {
    pub(in crate::backend) fn resolve_type(&self, ty: &Type) -> Type {
        match ty {
            Type::Unresolved(name) => self.program.types.get(name).cloned().unwrap_or_else(|| ty.clone()),
            Type::Vector(inner) => Type::Vector(Box::new(self.resolve_type(inner))),
            other => other.clone(),
        }
    }

    fn struct_declaration(&self, identifier: &str, span: Span) -> Result<&'a StructDeclaration, Box<dyn IError>> {
        let declared = self.program.declared_types.get(identifier).ok_or_else(|| {
            Box::new(CompilerError::at(
                ErrorSeverity::HIGH,
                format!("Unknown struct type '{}'.", identifier),
                span,
            )) as Box<dyn IError>
        })?;

        #[allow(irrefutable_let_patterns)]
        let DeclaredType::Struct(struct_decl) = &declared.value
        else {
            return Err(Box::new(CompilerError::at(
                ErrorSeverity::HIGH,
                format!("'{}' is not a struct type.", identifier),
                span,
            )));
        };

        Ok(struct_decl)
    }

    /// Builds the LLVM layout for a declared struct type.
    ///
    /// The returned `StructType` has one extra trailing `i64` field appended
    /// after all user-declared fields: the refcounting runtime's `refcount`
    /// counter. Use `struct_refcount_field_index` to find it. `field_indices`
    /// only contains the user-declared fields (unaffected by the extra field).
    pub(in crate::backend::llvm::compiler) fn struct_llvm_type(
        &self,
        identifier: &str,
        span: Span,
    ) -> Result<(StructType<'ctx>, HashMap<String, u32>), Box<dyn IError>> {
        let struct_decl = self.struct_declaration(identifier, span)?;

        let mut field_types = Vec::with_capacity(struct_decl.members.len() + 1);
        let mut field_indices = HashMap::new();

        for (idx, member) in struct_decl.members.iter().enumerate() {
            let resolved_type = self.resolve_type(&member.value.member_type.value);

            let llvm_type = LlvmValue::type_to_basic_type_enum(&resolved_type, self.context).ok_or_else(|| {
                Box::new(CompilerError::at(
                    ErrorSeverity::HIGH,
                    format!("Compiling struct fields of type '{}' is not yet supported.", resolved_type),
                    member.span,
                )) as Box<dyn IError>
            })?;

            field_types.push(llvm_type);
            field_indices.insert(member.value.identifier.value.clone(), idx as u32);
        }

        // Trailing refcount field, not exposed in `field_indices`.
        field_types.push(self.context.i64_type().into());

        Ok((self.context.struct_type(&field_types, false), field_indices))
    }

    /// Index of the trailing `refcount: i64` field appended by `struct_llvm_type`.
    pub(in crate::backend::llvm::compiler) fn struct_refcount_field_index(struct_type: StructType<'ctx>) -> u32 {
        struct_type.count_fields() - 1
    }
}
