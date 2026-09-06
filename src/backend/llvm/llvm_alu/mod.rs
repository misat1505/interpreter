mod arithmetic;
mod cast;
mod comparison;
pub mod llvm_value;
mod logical;
mod unary;

#[cfg(test)]
mod tests;

use inkwell::builder::{Builder, BuilderError};
use inkwell::values::IntValue;
use inkwell::IntPredicate;

use crate::backend::llvm::libc_functions::LibcFunctions;
use crate::backend::llvm::llvm_alu::llvm_value::LlvmValue;
use crate::common::errors::{CompilerError, ErrorSeverity, IError};
use crate::common::span::Span;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum OverflowPolicy {
    /// Default. No extra checks are generated - truncating casts silently
    /// wrap. Zero runtime cost.
    #[default]
    Ignore,
    /// Emits a runtime check before truncating casts; if the value doesn't
    /// fit, prints a warning to stderr and continues with the wrapped value.
    #[allow(dead_code)]
    Warn,
    /// Emits a runtime check before truncating casts; if the value doesn't
    /// fit, prints an error to stderr and aborts the program.
    Error,
}

impl OverflowPolicy {
    pub fn severity(self) -> ErrorSeverity {
        match self {
            OverflowPolicy::Ignore => ErrorSeverity::LOW,
            OverflowPolicy::Warn => ErrorSeverity::LOW,
            OverflowPolicy::Error => ErrorSeverity::HIGH,
        }
    }
}

pub struct LlvmAlu {
    overflow_policy: OverflowPolicy,
}

impl LlvmAlu {
    pub fn new(overflow_policy: OverflowPolicy) -> Self {
        Self { overflow_policy }
    }

    pub(in crate::backend::llvm::llvm_alu) fn map_err(err: BuilderError, span: Span) -> Box<dyn IError> {
        Box::new(CompilerError::at(ErrorSeverity::HIGH, err.to_string(), span))
    }

    pub(in crate::backend::llvm::llvm_alu) fn type_error<'ctx>(
        op_name: &str,
        left: LlvmValue<'ctx>,
        right: LlvmValue<'ctx>,
        span: Span,
    ) -> Box<dyn IError> {
        Box::new(CompilerError::at(
            ErrorSeverity::HIGH,
            format!(
                "Cannot perform {} between values of type '{}' and '{}'.",
                op_name,
                left.to_type(),
                right.to_type()
            ),
            span,
        ))
    }

    pub(in crate::backend::llvm::llvm_alu) fn unary_type_error<'ctx>(op_name: &str, value: LlvmValue<'ctx>, span: Span) -> Box<dyn IError> {
        Box::new(CompilerError::at(
            ErrorSeverity::HIGH,
            format!("Cannot perform {} on type '{}'.", op_name, value.to_type()),
            span,
        ))
    }

    #[allow(dead_code)]
    #[allow(clippy::too_many_arguments)]
    fn emit_overflow_check<'ctx>(
        &self,
        builder: &Builder<'ctx>,
        libc: &LibcFunctions<'ctx>,
        value: IntValue<'ctx>,
        target_bits: u32,
        source_signed: bool,
        target_signed: bool,
        span: Span,
    ) -> Result<(), Box<dyn IError>> {
        if self.overflow_policy == OverflowPolicy::Ignore {
            return Ok(());
        }

        let context = Self::context(builder);
        let source_type = value.get_type();

        // MIN/MAX of the *target* type, computed at the *source* width so the
        // comparison is apples-to-apples.
        let (min, max): (i128, i128) = match (target_bits, target_signed) {
            (8, true) => (i8::MIN as i128, i8::MAX as i128),
            (16, true) => (i16::MIN as i128, i16::MAX as i128),
            (32, true) => (i32::MIN as i128, i32::MAX as i128),
            (64, true) => (i64::MIN as i128, i64::MAX as i128),
            (8, false) => (0, u8::MAX as i128),
            (16, false) => (0, u16::MAX as i128),
            (32, false) => (0, u32::MAX as i128),
            (64, false) => (0, u64::MAX as i128),
            _ => unreachable!(),
        };

        let min_const = source_type.const_int(min as u64, source_signed);
        let max_const = source_type.const_int(max as u64, source_signed);

        let lt_min = if source_signed {
            builder
                .build_int_compare(IntPredicate::SLT, value, min_const, "overflow.lt_min")
                .map_err(|err| Self::map_err(err, span))?
        } else {
            // Unsigned source can never be below an unsigned/zero min, so this
            // check is only meaningful when min > 0, which cannot happen here
            // since unsigned mins are always 0. Use a constant `false`.
            builder
                .build_int_compare(IntPredicate::ULT, value, min_const, "overflow.lt_min")
                .map_err(|err| Self::map_err(err, span))?
        };

        let gt_max = if source_signed {
            builder
                .build_int_compare(IntPredicate::SGT, value, max_const, "overflow.gt_max")
                .map_err(|err| Self::map_err(err, span))?
        } else {
            builder
                .build_int_compare(IntPredicate::UGT, value, max_const, "overflow.gt_max")
                .map_err(|err| Self::map_err(err, span))?
        };

        let out_of_range = builder
            .build_or(lt_min, gt_max, "overflow.out_of_range")
            .map_err(|err| Self::map_err(err, span))?;

        let function = builder
            .get_insert_block()
            .expect("builder should be positioned inside a function")
            .get_parent()
            .expect("basic block should belong to a function");

        let overflow_block = context.append_basic_block(function, "overflow.check");
        let merge_block = context.append_basic_block(function, "overflow.continue");

        builder
            .build_conditional_branch(out_of_range, overflow_block, merge_block)
            .map_err(|err| Self::map_err(err, span))?;

        builder.position_at_end(overflow_block);

        let error = CompilerError::at(
            self.overflow_policy.severity(),
            format!(
                "Value does not fit in target type ({}:{})",
                target_bits,
                if target_signed { "signed" } else { "unsigned" }
            ),
            span,
        );

        let message = format!("{}\n", error.get_stderr_message());

        let format_str = builder
            .build_global_string_ptr(&message, "overflow.msg")
            .map_err(|err| Self::map_err(err, span))?;

        builder
            .build_call(libc.printf_fn, &[format_str.as_pointer_value().into()], "overflow.printf")
            .map_err(|err| Self::map_err(err, span))?;

        if self.overflow_policy == OverflowPolicy::Error {
            let i32_type = context.i32_type();

            builder
                .build_call(libc.exit_fn, &[i32_type.const_int(1, false).into()], "overflow.exit")
                .map_err(|err| Self::map_err(err, span))?;

            builder.build_unreachable().map_err(|err| Self::map_err(err, span))?;
        } else {
            builder.build_unconditional_branch(merge_block).map_err(|err| Self::map_err(err, span))?;
        }

        builder.position_at_end(merge_block);

        Ok(())
    }
}
