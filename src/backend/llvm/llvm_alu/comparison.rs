use inkwell::{
    builder::Builder,
    values::{IntValue, PointerValue},
    FloatPredicate, IntPredicate,
};

use crate::{
    backend::llvm::{
        libc_functions::LibcFunctions,
        llvm_alu::{llvm_value::LlvmValue, LlvmAlu},
    },
    common::{errors::IError, span::Span},
};

impl LlvmAlu {
    /// Compares two `StrHeader` pointers by their character data (`strcmp`
    /// on the underlying C strings, not the header pointers themselves).
    pub(in crate::backend::llvm::llvm_alu) fn strcmp<'ctx>(
        builder: &Builder<'ctx>,
        libc: &LibcFunctions<'ctx>,
        left: PointerValue<'ctx>,
        right: PointerValue<'ctx>,
        span: Span,
    ) -> Result<IntValue<'ctx>, Box<dyn IError>> {
        let context = Self::context(builder);
        let ptr_type = context.ptr_type(inkwell::AddressSpace::default());
        let i64_type = context.i64_type();

        // StrHeader { refcount: i64, data: i8* }
        let header_type = context.struct_type(&[i64_type.into(), ptr_type.into()], false);

        let left_data_field = builder
            .build_struct_gep(header_type, left, crate::backend::llvm::llvm_alu::llvm_value::STR_DATA, "strcmp.left.data")
            .map_err(|err| Self::map_err(err, span))?;
        let left_data = builder
            .build_load(ptr_type, left_data_field, "strcmp.left.data.val")
            .map_err(|err| Self::map_err(err, span))?
            .into_pointer_value();

        let right_data_field = builder
            .build_struct_gep(header_type, right, crate::backend::llvm::llvm_alu::llvm_value::STR_DATA, "strcmp.right.data")
            .map_err(|err| Self::map_err(err, span))?;
        let right_data = builder
            .build_load(ptr_type, right_data_field, "strcmp.right.data.val")
            .map_err(|err| Self::map_err(err, span))?
            .into_pointer_value();

        let call = builder
            .build_call(libc.strcmp_fn, &[left_data.into(), right_data.into()], "strcmp")
            .map_err(|err| Self::map_err(err, span))?;

        Ok(call.try_as_basic_value().basic().expect("strcmp should return a value").into_int_value())
    }

    pub fn greater<'ctx>(
        &self,
        builder: &Builder<'ctx>,
        _libc: &LibcFunctions<'ctx>,
        left: LlvmValue<'ctx>,
        right: LlvmValue<'ctx>,
        span: Span,
    ) -> Result<LlvmValue<'ctx>, Box<dyn IError>> {
        match (left, right) {
            // Signed integers
            (LlvmValue::I8(l), LlvmValue::I8(r)) => Self::int_compare(builder, IntPredicate::SGT, l, r, "greater", span),
            (LlvmValue::I16(l), LlvmValue::I16(r)) => Self::int_compare(builder, IntPredicate::SGT, l, r, "greater", span),
            (LlvmValue::I32(l), LlvmValue::I32(r)) => Self::int_compare(builder, IntPredicate::SGT, l, r, "greater", span),
            (LlvmValue::I64(l), LlvmValue::I64(r)) => Self::int_compare(builder, IntPredicate::SGT, l, r, "greater", span),

            // Unsigned integers
            (LlvmValue::U8(l), LlvmValue::U8(r)) => Self::int_compare(builder, IntPredicate::UGT, l, r, "greater", span),
            (LlvmValue::U16(l), LlvmValue::U16(r)) => Self::int_compare(builder, IntPredicate::UGT, l, r, "greater", span),
            (LlvmValue::U32(l), LlvmValue::U32(r)) => Self::int_compare(builder, IntPredicate::UGT, l, r, "greater", span),
            (LlvmValue::U64(l), LlvmValue::U64(r)) => Self::int_compare(builder, IntPredicate::UGT, l, r, "greater", span),

            // Floating point
            (LlvmValue::F64(l), LlvmValue::F64(r)) => builder
                .build_float_compare(FloatPredicate::OGT, l, r, "greater")
                .map(LlvmValue::Bool)
                .map_err(|err| Self::map_err(err, span)),

            (l, r) => Err(Self::type_error("greater", l, r, span)),
        }
    }

    pub fn greater_or_equal<'ctx>(
        &self,
        builder: &Builder<'ctx>,
        _libc: &LibcFunctions<'ctx>,
        left: LlvmValue<'ctx>,
        right: LlvmValue<'ctx>,
        span: Span,
    ) -> Result<LlvmValue<'ctx>, Box<dyn IError>> {
        match (left, right) {
            (LlvmValue::I8(l), LlvmValue::I8(r)) => Self::int_compare(builder, IntPredicate::SGE, l, r, "greater_or_equal", span),
            (LlvmValue::I16(l), LlvmValue::I16(r)) => Self::int_compare(builder, IntPredicate::SGE, l, r, "greater_or_equal", span),
            (LlvmValue::I32(l), LlvmValue::I32(r)) => Self::int_compare(builder, IntPredicate::SGE, l, r, "greater_or_equal", span),
            (LlvmValue::I64(l), LlvmValue::I64(r)) => Self::int_compare(builder, IntPredicate::SGE, l, r, "greater_or_equal", span),

            (LlvmValue::U8(l), LlvmValue::U8(r)) => Self::int_compare(builder, IntPredicate::UGE, l, r, "greater_or_equal", span),
            (LlvmValue::U16(l), LlvmValue::U16(r)) => Self::int_compare(builder, IntPredicate::UGE, l, r, "greater_or_equal", span),
            (LlvmValue::U32(l), LlvmValue::U32(r)) => Self::int_compare(builder, IntPredicate::UGE, l, r, "greater_or_equal", span),
            (LlvmValue::U64(l), LlvmValue::U64(r)) => Self::int_compare(builder, IntPredicate::UGE, l, r, "greater_or_equal", span),

            (LlvmValue::F64(l), LlvmValue::F64(r)) => builder
                .build_float_compare(FloatPredicate::OGE, l, r, "greater_or_equal")
                .map(LlvmValue::Bool)
                .map_err(|err| Self::map_err(err, span)),

            (l, r) => Err(Self::type_error("greater or equal", l, r, span)),
        }
    }

    pub fn less<'ctx>(
        &self,
        builder: &Builder<'ctx>,
        _libc: &LibcFunctions<'ctx>,
        left: LlvmValue<'ctx>,
        right: LlvmValue<'ctx>,
        span: Span,
    ) -> Result<LlvmValue<'ctx>, Box<dyn IError>> {
        match (left, right) {
            (LlvmValue::I8(l), LlvmValue::I8(r)) => Self::int_compare(builder, IntPredicate::SLT, l, r, "less", span),
            (LlvmValue::I16(l), LlvmValue::I16(r)) => Self::int_compare(builder, IntPredicate::SLT, l, r, "less", span),
            (LlvmValue::I32(l), LlvmValue::I32(r)) => Self::int_compare(builder, IntPredicate::SLT, l, r, "less", span),
            (LlvmValue::I64(l), LlvmValue::I64(r)) => Self::int_compare(builder, IntPredicate::SLT, l, r, "less", span),

            (LlvmValue::U8(l), LlvmValue::U8(r)) => Self::int_compare(builder, IntPredicate::ULT, l, r, "less", span),
            (LlvmValue::U16(l), LlvmValue::U16(r)) => Self::int_compare(builder, IntPredicate::ULT, l, r, "less", span),
            (LlvmValue::U32(l), LlvmValue::U32(r)) => Self::int_compare(builder, IntPredicate::ULT, l, r, "less", span),
            (LlvmValue::U64(l), LlvmValue::U64(r)) => Self::int_compare(builder, IntPredicate::ULT, l, r, "less", span),

            (LlvmValue::F64(l), LlvmValue::F64(r)) => builder
                .build_float_compare(FloatPredicate::OLT, l, r, "less")
                .map(LlvmValue::Bool)
                .map_err(|err| Self::map_err(err, span)),

            (l, r) => Err(Self::type_error("less", l, r, span)),
        }
    }

    pub fn less_or_equal<'ctx>(
        &self,
        builder: &Builder<'ctx>,
        _libc: &LibcFunctions<'ctx>,
        left: LlvmValue<'ctx>,
        right: LlvmValue<'ctx>,
        span: Span,
    ) -> Result<LlvmValue<'ctx>, Box<dyn IError>> {
        match (left, right) {
            (LlvmValue::I8(l), LlvmValue::I8(r)) => Self::int_compare(builder, IntPredicate::SLE, l, r, "less_or_equal", span),
            (LlvmValue::I16(l), LlvmValue::I16(r)) => Self::int_compare(builder, IntPredicate::SLE, l, r, "less_or_equal", span),
            (LlvmValue::I32(l), LlvmValue::I32(r)) => Self::int_compare(builder, IntPredicate::SLE, l, r, "less_or_equal", span),
            (LlvmValue::I64(l), LlvmValue::I64(r)) => Self::int_compare(builder, IntPredicate::SLE, l, r, "less_or_equal", span),

            (LlvmValue::U8(l), LlvmValue::U8(r)) => Self::int_compare(builder, IntPredicate::ULE, l, r, "less_or_equal", span),
            (LlvmValue::U16(l), LlvmValue::U16(r)) => Self::int_compare(builder, IntPredicate::ULE, l, r, "less_or_equal", span),
            (LlvmValue::U32(l), LlvmValue::U32(r)) => Self::int_compare(builder, IntPredicate::ULE, l, r, "less_or_equal", span),
            (LlvmValue::U64(l), LlvmValue::U64(r)) => Self::int_compare(builder, IntPredicate::ULE, l, r, "less_or_equal", span),

            (LlvmValue::F64(l), LlvmValue::F64(r)) => builder
                .build_float_compare(FloatPredicate::OLE, l, r, "less_or_equal")
                .map(LlvmValue::Bool)
                .map_err(|err| Self::map_err(err, span)),

            (l, r) => Err(Self::type_error("less or equal", l, r, span)),
        }
    }

    pub fn equal<'ctx>(
        &self,
        builder: &Builder<'ctx>,
        libc: &LibcFunctions<'ctx>,
        left: LlvmValue<'ctx>,
        right: LlvmValue<'ctx>,
        span: Span,
    ) -> Result<LlvmValue<'ctx>, Box<dyn IError>> {
        match (left, right) {
            // All integer types can be compared with all integer types
            (left, right) if left.is_integer() && right.is_integer() => Self::integer_equality(builder, left, right, false, span),

            (LlvmValue::F64(l), LlvmValue::F64(r)) => builder
                .build_float_compare(FloatPredicate::OEQ, l, r, "equal")
                .map(LlvmValue::Bool)
                .map_err(|err| Self::map_err(err, span)),

            (LlvmValue::Bool(l), LlvmValue::Bool(r)) => builder
                .build_int_compare(IntPredicate::EQ, l, r, "equal")
                .map(LlvmValue::Bool)
                .map_err(|err| Self::map_err(err, span)),

            (LlvmValue::Str(l), LlvmValue::Str(r)) => {
                let cmp = Self::strcmp(builder, libc, l, r, span)?;
                let zero = cmp.get_type().const_zero();

                builder
                    .build_int_compare(IntPredicate::EQ, cmp, zero, "equal")
                    .map(LlvmValue::Bool)
                    .map_err(|err| Self::map_err(err, span))
            }

            (LlvmValue::Char(l), LlvmValue::Char(r)) => builder
                .build_int_compare(IntPredicate::EQ, l, r, "char_equal")
                .map(LlvmValue::Bool)
                .map_err(|err| Self::map_err(err, span)),

            (l, r) => Err(Self::type_error("equal", l, r, span)),
        }
    }

    pub fn not_equal<'ctx>(
        &self,
        builder: &Builder<'ctx>,
        libc: &LibcFunctions<'ctx>,
        left: LlvmValue<'ctx>,
        right: LlvmValue<'ctx>,
        span: Span,
    ) -> Result<LlvmValue<'ctx>, Box<dyn IError>> {
        match (left, right) {
            (left, right) if left.is_integer() && right.is_integer() => Self::integer_equality(builder, left, right, true, span),

            (LlvmValue::F64(l), LlvmValue::F64(r)) => builder
                .build_float_compare(FloatPredicate::ONE, l, r, "not_equal")
                .map(LlvmValue::Bool)
                .map_err(|err| Self::map_err(err, span)),

            (LlvmValue::Bool(l), LlvmValue::Bool(r)) => builder
                .build_int_compare(IntPredicate::NE, l, r, "not_equal")
                .map(LlvmValue::Bool)
                .map_err(|err| Self::map_err(err, span)),

            (LlvmValue::Str(l), LlvmValue::Str(r)) => {
                let cmp = Self::strcmp(builder, libc, l, r, span)?;
                let zero = cmp.get_type().const_zero();

                builder
                    .build_int_compare(IntPredicate::NE, cmp, zero, "not_equal")
                    .map(LlvmValue::Bool)
                    .map_err(|err| Self::map_err(err, span))
            }

            (LlvmValue::Char(l), LlvmValue::Char(r)) => builder
                .build_int_compare(IntPredicate::NE, l, r, "char_not_equal")
                .map(LlvmValue::Bool)
                .map_err(|err| Self::map_err(err, span)),

            (l, r) => Err(Self::type_error("not equal", l, r, span)),
        }
    }

    fn int_compare<'ctx>(
        builder: &Builder<'ctx>,
        predicate: IntPredicate,
        left: IntValue<'ctx>,
        right: IntValue<'ctx>,
        name: &str,
        span: Span,
    ) -> Result<LlvmValue<'ctx>, Box<dyn IError>> {
        builder
            .build_int_compare(predicate, left, right, name)
            .map(LlvmValue::Bool)
            .map_err(|err| Self::map_err(err, span))
    }

    fn integer_equality<'ctx>(
        builder: &Builder<'ctx>,
        left: LlvmValue<'ctx>,
        right: LlvmValue<'ctx>,
        not_equal: bool,
        span: Span,
    ) -> Result<LlvmValue<'ctx>, Box<dyn IError>> {
        let (left, right) = Self::normalize_integer_pair(builder, left, right, span)?;

        let predicate = if not_equal { IntPredicate::NE } else { IntPredicate::EQ };

        builder
            .build_int_compare(predicate, left, right, "integer_cmp")
            .map(LlvmValue::Bool)
            .map_err(|err| Self::map_err(err, span))
    }

    fn normalize_integer_pair<'ctx>(
        builder: &Builder<'ctx>,
        left: LlvmValue<'ctx>,
        right: LlvmValue<'ctx>,
        span: Span,
    ) -> Result<(IntValue<'ctx>, IntValue<'ctx>), Box<dyn IError>> {
        let left = Self::normalize_integer(builder, left, span)?;
        let right = Self::normalize_integer(builder, right, span)?;

        Ok((left, right))
    }

    fn normalize_integer<'ctx>(builder: &Builder<'ctx>, value: LlvmValue<'ctx>, span: Span) -> Result<IntValue<'ctx>, Box<dyn IError>> {
        let target = builder
            .get_insert_block()
            .expect("builder should be positioned inside a block")
            .get_context()
            .i64_type();

        match value {
            LlvmValue::I8(v) | LlvmValue::I16(v) | LlvmValue::I32(v) => builder
                .build_int_s_extend(v, target, "normalize_signed")
                .map_err(|err| Self::map_err(err, span)),

            LlvmValue::I64(v) => Ok(v),

            LlvmValue::U8(v) | LlvmValue::U16(v) | LlvmValue::U32(v) => builder
                .build_int_z_extend(v, target, "normalize_unsigned")
                .map_err(|err| Self::map_err(err, span)),

            LlvmValue::U64(v) => Ok(v),

            other => Err(Self::type_error("integer equality", other, LlvmValue::I64(target.const_zero()), span)),
        }
    }
}
