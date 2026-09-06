use inkwell::values::{IntValue, PointerValue};
use inkwell::AddressSpace;
use inkwell::{builder::Builder, IntPredicate};

use crate::common::errors::CompilerError;
use crate::{
    backend::llvm::{
        libc_functions::LibcFunctions,
        llvm_alu::llvm_value::{LlvmValue, STR_DATA, STR_REFCOUNT},
        llvm_alu::{LlvmAlu, OverflowPolicy},
    },
    common::{errors::IError, span::Span},
};

#[derive(Clone, Copy)]
enum IntegerBinaryOp {
    Add,
    Sub,
    Mul,
}

impl LlvmAlu {
    // ========================================================================
    // String helpers
    // ========================================================================

    /// Allocates a fresh `StrHeader { refcount: 1, data }` around an
    /// already-heap-allocated `data` C string. Mirrors
    /// `Compiler::build_str_header`, duplicated here because `LlvmAlu` has
    /// no access to a `Compiler`.
    pub(in crate::backend::llvm::llvm_alu) fn wrap_str_header<'ctx>(
        builder: &Builder<'ctx>,
        libc: &LibcFunctions<'ctx>,
        data: PointerValue<'ctx>,
        span: Span,
    ) -> Result<PointerValue<'ctx>, Box<dyn IError>> {
        let context = Self::context(builder);
        let ptr_type = context.ptr_type(AddressSpace::default());
        let i64_type = context.i64_type();

        // StrHeader { refcount: i64, data: i8* } - field order matches STR_REFCOUNT/STR_DATA.
        let header_type = context.struct_type(&[i64_type.into(), ptr_type.into()], false);
        let header_size = header_type.size_of().expect("StrHeader must have a known size");

        let header_ptr = builder
            .build_call(libc.malloc_fn, &[header_size.into()], "str.header.malloc")
            .map_err(|err| Self::map_err(err, span))?
            .try_as_basic_value()
            .basic()
            .expect("malloc should return a value")
            .into_pointer_value();

        let rc_field = builder
            .build_struct_gep(header_type, header_ptr, STR_REFCOUNT, "str.rc")
            .map_err(|err| Self::map_err(err, span))?;
        builder
            .build_store(rc_field, i64_type.const_int(1, false))
            .map_err(|err| Self::map_err(err, span))?;

        let data_field = builder
            .build_struct_gep(header_type, header_ptr, STR_DATA, "str.data")
            .map_err(|err| Self::map_err(err, span))?;
        builder.build_store(data_field, data).map_err(|err| Self::map_err(err, span))?;

        Ok(header_ptr)
    }

    /// Loads the `data: i8*` field out of a `StrHeader` pointer.
    pub(in crate::backend::llvm::llvm_alu) fn str_data_ptr<'ctx>(
        builder: &Builder<'ctx>,
        header_ptr: PointerValue<'ctx>,
        span: Span,
    ) -> Result<PointerValue<'ctx>, Box<dyn IError>> {
        let context = Self::context(builder);
        let ptr_type = context.ptr_type(AddressSpace::default());
        let i64_type = context.i64_type();

        let header_type = context.struct_type(&[i64_type.into(), ptr_type.into()], false);

        let data_field = builder
            .build_struct_gep(header_type, header_ptr, STR_DATA, "str.data.field")
            .map_err(|err| Self::map_err(err, span))?;

        let data = builder
            .build_load(ptr_type, data_field, "str.data.val")
            .map_err(|err| Self::map_err(err, span))?
            .into_pointer_value();

        Ok(data)
    }

    /// Concatenates two strings (given as `StrHeader` pointers) into a
    /// brand new, independently owned `StrHeader` (refcount 1).
    fn concat_strings<'ctx>(
        &self,
        builder: &Builder<'ctx>,
        libc: &LibcFunctions<'ctx>,
        left: PointerValue<'ctx>,
        right: PointerValue<'ctx>,
        span: Span,
    ) -> Result<LlvmValue<'ctx>, Box<dyn IError>> {
        let left_data = Self::str_data_ptr(builder, left, span)?;
        let right_data = Self::str_data_ptr(builder, right, span)?;

        let len_l_call = builder
            .build_call(libc.strlen_fn, &[left_data.into()], "strlen_l")
            .map_err(|err| Self::map_err(err, span))?;

        let len_l = len_l_call
            .try_as_basic_value()
            .basic()
            .expect("strlen should return a value")
            .into_int_value();

        let len_r_call = builder
            .build_call(libc.strlen_fn, &[right_data.into()], "strlen_r")
            .map_err(|err| Self::map_err(err, span))?;

        let len_r = len_r_call
            .try_as_basic_value()
            .basic()
            .expect("strlen should return a value")
            .into_int_value();

        /*
         * strlen returns size_t.
         *
         * We need:
         *
         *     len_l + len_r + 1
         *
         * and this must not overflow.
         */
        let size_type = len_l.get_type();
        let one = size_type.const_int(1, false);

        if self.overflow_policy != OverflowPolicy::Ignore {
            let max = size_type.const_all_ones();

            let max_minus_one = builder
                .build_int_sub(max, one, "concat.max_minus_one")
                .map_err(|err| Self::map_err(err, span))?;

            let l_too_large = builder
                .build_int_compare(IntPredicate::UGT, len_l, max_minus_one, "concat.l_too_large")
                .map_err(|err| Self::map_err(err, span))?;

            let r_too_large = builder
                .build_int_compare(IntPredicate::UGT, len_r, max_minus_one, "concat.r_too_large")
                .map_err(|err| Self::map_err(err, span))?;

            let partial_overflow = builder
                .build_or(l_too_large, r_too_large, "concat.partial_overflow")
                .map_err(|err| Self::map_err(err, span))?;

            let safe_max = builder
                .build_int_sub(max_minus_one, len_l, "concat.safe_max")
                .map_err(|err| Self::map_err(err, span))?;

            let sum_overflow = builder
                .build_int_compare(IntPredicate::UGT, len_r, safe_max, "concat.sum_overflow")
                .map_err(|err| Self::map_err(err, span))?;

            let overflow = builder
                .build_or(partial_overflow, sum_overflow, "concat.overflow")
                .map_err(|err| Self::map_err(err, span))?;

            self.emit_overflow_branch(builder, libc, overflow, span, "string concatenation length overflow")?;
        }

        let total_len = builder
            .build_int_add(len_l, len_r, "concat_len")
            .and_then(|sum_len| builder.build_int_add(sum_len, one, "concat_total_len"))
            .map_err(|err| Self::map_err(err, span))?;

        /*
         * Allocate the character buffer and copy both strings into it.
         */
        let malloc_data_call = builder
            .build_call(libc.malloc_fn, &[total_len.into()], "concat_buf")
            .map_err(|err| Self::map_err(err, span))?;

        let data = malloc_data_call
            .try_as_basic_value()
            .basic()
            .expect("malloc should return a value")
            .into_pointer_value();

        builder
            .build_call(libc.strcpy_fn, &[data.into(), left_data.into()], "strcpy_call")
            .map_err(|err| Self::map_err(err, span))?;

        builder
            .build_call(libc.strcat_fn, &[data.into(), right_data.into()], "strcat_call")
            .map_err(|err| Self::map_err(err, span))?;

        let header_ptr = Self::wrap_str_header(builder, libc, data, span)?;

        Ok(LlvmValue::Str(header_ptr))
    }

    /// Turns a single `char` into a freshly-allocated, headered one-character `Str`.
    fn char_to_string<'ctx>(
        &self,
        builder: &Builder<'ctx>,
        libc: &LibcFunctions<'ctx>,
        ch: IntValue<'ctx>,
        span: Span,
    ) -> Result<PointerValue<'ctx>, Box<dyn IError>> {
        let i64_type = ch.get_type().get_context().i64_type();

        let size = i64_type.const_int(2, false);

        let malloc_call = builder
            .build_call(libc.malloc_fn, &[size.into()], "char_str_buf")
            .map_err(|err| Self::map_err(err, span))?;

        let buf = malloc_call
            .try_as_basic_value()
            .basic()
            .expect("malloc should return a value")
            .into_pointer_value();

        builder.build_store(buf, ch).map_err(|err| Self::map_err(err, span))?;

        let one = i64_type.const_int(1, false);

        let null_ptr = unsafe {
            builder
                .build_gep(ch.get_type(), buf, &[one], "char_str_null")
                .map_err(|err| Self::map_err(err, span))?
        };

        let zero = ch.get_type().const_zero();

        builder.build_store(null_ptr, zero).map_err(|err| Self::map_err(err, span))?;

        Self::wrap_str_header(builder, libc, buf, span)
    }

    fn free_temp_str<'ctx>(
        builder: &Builder<'ctx>,
        libc: &LibcFunctions<'ctx>,
        header_ptr: PointerValue<'ctx>,
        span: Span,
    ) -> Result<(), Box<dyn IError>> {
        let data_ptr = Self::str_data_ptr(builder, header_ptr, span)?;

        builder
            .build_call(libc.free_fn, &[data_ptr.into()], "temp_str.free_data")
            .map_err(|err| Self::map_err(err, span))?;

        builder
            .build_call(libc.free_fn, &[header_ptr.into()], "temp_str.free_header")
            .map_err(|err| Self::map_err(err, span))?;

        Ok(())
    }

    // ========================================================================
    // Addition
    // ========================================================================

    pub fn add<'ctx>(
        &self,
        builder: &Builder<'ctx>,
        libc: &LibcFunctions<'ctx>,
        left: LlvmValue<'ctx>,
        right: LlvmValue<'ctx>,
        span: Span,
    ) -> Result<LlvmValue<'ctx>, Box<dyn IError>> {
        match (left, right) {
            (LlvmValue::I8(l), LlvmValue::I8(r)) => self
                .checked_integer_operation(builder, libc, l, r, true, 8, IntegerBinaryOp::Add, span)
                .map(LlvmValue::I8),

            (LlvmValue::I16(l), LlvmValue::I16(r)) => self
                .checked_integer_operation(builder, libc, l, r, true, 16, IntegerBinaryOp::Add, span)
                .map(LlvmValue::I16),

            (LlvmValue::I32(l), LlvmValue::I32(r)) => self
                .checked_integer_operation(builder, libc, l, r, true, 32, IntegerBinaryOp::Add, span)
                .map(LlvmValue::I32),

            (LlvmValue::I64(l), LlvmValue::I64(r)) => self
                .checked_integer_operation(builder, libc, l, r, true, 64, IntegerBinaryOp::Add, span)
                .map(LlvmValue::I64),

            (LlvmValue::U8(l), LlvmValue::U8(r)) => self
                .checked_integer_operation(builder, libc, l, r, false, 8, IntegerBinaryOp::Add, span)
                .map(LlvmValue::U8),

            (LlvmValue::U16(l), LlvmValue::U16(r)) => self
                .checked_integer_operation(builder, libc, l, r, false, 16, IntegerBinaryOp::Add, span)
                .map(LlvmValue::U16),

            (LlvmValue::U32(l), LlvmValue::U32(r)) => self
                .checked_integer_operation(builder, libc, l, r, false, 32, IntegerBinaryOp::Add, span)
                .map(LlvmValue::U32),

            (LlvmValue::U64(l), LlvmValue::U64(r)) => self
                .checked_integer_operation(builder, libc, l, r, false, 64, IntegerBinaryOp::Add, span)
                .map(LlvmValue::U64),

            (LlvmValue::F64(l), LlvmValue::F64(r)) => builder
                .build_float_add(l, r, "addtmp")
                .map(LlvmValue::F64)
                .map_err(|err| Self::map_err(err, span)),

            (LlvmValue::Char(l), LlvmValue::Char(r)) => {
                let left_str = self.char_to_string(builder, libc, l, span)?;
                let right_str = self.char_to_string(builder, libc, r, span)?;

                let result = self.concat_strings(builder, libc, left_str, right_str, span)?;

                Self::free_temp_str(builder, libc, left_str, span)?;
                Self::free_temp_str(builder, libc, right_str, span)?;

                Ok(result)
            }

            (LlvmValue::Str(l), LlvmValue::Char(r)) => {
                let right_str = self.char_to_string(builder, libc, r, span)?;

                let result = self.concat_strings(builder, libc, l, right_str, span)?;

                Self::free_temp_str(builder, libc, right_str, span)?;

                Ok(result)
            }

            (LlvmValue::Char(l), LlvmValue::Str(r)) => {
                let left_str = self.char_to_string(builder, libc, l, span)?;

                let result = self.concat_strings(builder, libc, left_str, r, span)?;

                Self::free_temp_str(builder, libc, left_str, span)?;

                Ok(result)
            }

            (LlvmValue::Str(l), LlvmValue::Str(r)) => self.concat_strings(builder, libc, l, r, span),

            (l, r) => Err(Self::type_error("addition", l, r, span)),
        }
    }

    // ========================================================================
    // Subtraction
    // ========================================================================

    pub fn subtract<'ctx>(
        &self,
        builder: &Builder<'ctx>,
        libc: &LibcFunctions<'ctx>,
        left: LlvmValue<'ctx>,
        right: LlvmValue<'ctx>,
        span: Span,
    ) -> Result<LlvmValue<'ctx>, Box<dyn IError>> {
        match (left, right) {
            (LlvmValue::I8(l), LlvmValue::I8(r)) => self
                .checked_integer_operation(builder, libc, l, r, true, 8, IntegerBinaryOp::Sub, span)
                .map(LlvmValue::I8),

            (LlvmValue::I16(l), LlvmValue::I16(r)) => self
                .checked_integer_operation(builder, libc, l, r, true, 16, IntegerBinaryOp::Sub, span)
                .map(LlvmValue::I16),

            (LlvmValue::I32(l), LlvmValue::I32(r)) => self
                .checked_integer_operation(builder, libc, l, r, true, 32, IntegerBinaryOp::Sub, span)
                .map(LlvmValue::I32),

            (LlvmValue::I64(l), LlvmValue::I64(r)) => self
                .checked_integer_operation(builder, libc, l, r, true, 64, IntegerBinaryOp::Sub, span)
                .map(LlvmValue::I64),

            (LlvmValue::U8(l), LlvmValue::U8(r)) => self
                .checked_integer_operation(builder, libc, l, r, false, 8, IntegerBinaryOp::Sub, span)
                .map(LlvmValue::U8),

            (LlvmValue::U16(l), LlvmValue::U16(r)) => self
                .checked_integer_operation(builder, libc, l, r, false, 16, IntegerBinaryOp::Sub, span)
                .map(LlvmValue::U16),

            (LlvmValue::U32(l), LlvmValue::U32(r)) => self
                .checked_integer_operation(builder, libc, l, r, false, 32, IntegerBinaryOp::Sub, span)
                .map(LlvmValue::U32),

            (LlvmValue::U64(l), LlvmValue::U64(r)) => self
                .checked_integer_operation(builder, libc, l, r, false, 64, IntegerBinaryOp::Sub, span)
                .map(LlvmValue::U64),

            (LlvmValue::F64(l), LlvmValue::F64(r)) => builder
                .build_float_sub(l, r, "subtmp")
                .map(LlvmValue::F64)
                .map_err(|err| Self::map_err(err, span)),

            (l, r) => Err(Self::type_error("subtraction", l, r, span)),
        }
    }

    // ========================================================================
    // Multiplication
    // ========================================================================

    pub fn multiplication<'ctx>(
        &self,
        builder: &Builder<'ctx>,
        libc: &LibcFunctions<'ctx>,
        left: LlvmValue<'ctx>,
        right: LlvmValue<'ctx>,
        span: Span,
    ) -> Result<LlvmValue<'ctx>, Box<dyn IError>> {
        match (left, right) {
            (LlvmValue::I8(l), LlvmValue::I8(r)) => self
                .checked_integer_operation(builder, libc, l, r, true, 8, IntegerBinaryOp::Mul, span)
                .map(LlvmValue::I8),

            (LlvmValue::I16(l), LlvmValue::I16(r)) => self
                .checked_integer_operation(builder, libc, l, r, true, 16, IntegerBinaryOp::Mul, span)
                .map(LlvmValue::I16),

            (LlvmValue::I32(l), LlvmValue::I32(r)) => self
                .checked_integer_operation(builder, libc, l, r, true, 32, IntegerBinaryOp::Mul, span)
                .map(LlvmValue::I32),

            (LlvmValue::I64(l), LlvmValue::I64(r)) => self
                .checked_integer_operation(builder, libc, l, r, true, 64, IntegerBinaryOp::Mul, span)
                .map(LlvmValue::I64),

            (LlvmValue::U8(l), LlvmValue::U8(r)) => self
                .checked_integer_operation(builder, libc, l, r, false, 8, IntegerBinaryOp::Mul, span)
                .map(LlvmValue::U8),

            (LlvmValue::U16(l), LlvmValue::U16(r)) => self
                .checked_integer_operation(builder, libc, l, r, false, 16, IntegerBinaryOp::Mul, span)
                .map(LlvmValue::U16),

            (LlvmValue::U32(l), LlvmValue::U32(r)) => self
                .checked_integer_operation(builder, libc, l, r, false, 32, IntegerBinaryOp::Mul, span)
                .map(LlvmValue::U32),

            (LlvmValue::U64(l), LlvmValue::U64(r)) => self
                .checked_integer_operation(builder, libc, l, r, false, 64, IntegerBinaryOp::Mul, span)
                .map(LlvmValue::U64),

            (LlvmValue::F64(l), LlvmValue::F64(r)) => builder
                .build_float_mul(l, r, "multmp")
                .map(LlvmValue::F64)
                .map_err(|err| Self::map_err(err, span)),

            (l, r) => Err(Self::type_error("multiplication", l, r, span)),
        }
    }

    // ========================================================================
    // Division
    // ========================================================================

    pub fn division<'ctx>(
        &self,
        builder: &Builder<'ctx>,
        libc: &LibcFunctions<'ctx>,
        left: LlvmValue<'ctx>,
        right: LlvmValue<'ctx>,
        span: Span,
    ) -> Result<LlvmValue<'ctx>, Box<dyn IError>> {
        match (left, right) {
            (LlvmValue::I8(l), LlvmValue::I8(r)) => {
                self.check_signed_division_overflow(builder, libc, l, r, 8, span)?;

                builder
                    .build_int_signed_div(l, r, "divtmp")
                    .map(LlvmValue::I8)
                    .map_err(|err| Self::map_err(err, span))
            }

            (LlvmValue::I16(l), LlvmValue::I16(r)) => {
                self.check_signed_division_overflow(builder, libc, l, r, 16, span)?;

                builder
                    .build_int_signed_div(l, r, "divtmp")
                    .map(LlvmValue::I16)
                    .map_err(|err| Self::map_err(err, span))
            }

            (LlvmValue::I32(l), LlvmValue::I32(r)) => {
                self.check_signed_division_overflow(builder, libc, l, r, 32, span)?;

                builder
                    .build_int_signed_div(l, r, "divtmp")
                    .map(LlvmValue::I32)
                    .map_err(|err| Self::map_err(err, span))
            }

            (LlvmValue::I64(l), LlvmValue::I64(r)) => {
                self.check_signed_division_overflow(builder, libc, l, r, 64, span)?;

                builder
                    .build_int_signed_div(l, r, "divtmp")
                    .map(LlvmValue::I64)
                    .map_err(|err| Self::map_err(err, span))
            }

            (LlvmValue::U8(l), LlvmValue::U8(r)) => builder
                .build_int_unsigned_div(l, r, "divtmp")
                .map(LlvmValue::U8)
                .map_err(|err| Self::map_err(err, span)),

            (LlvmValue::U16(l), LlvmValue::U16(r)) => builder
                .build_int_unsigned_div(l, r, "divtmp")
                .map(LlvmValue::U16)
                .map_err(|err| Self::map_err(err, span)),

            (LlvmValue::U32(l), LlvmValue::U32(r)) => builder
                .build_int_unsigned_div(l, r, "divtmp")
                .map(LlvmValue::U32)
                .map_err(|err| Self::map_err(err, span)),

            (LlvmValue::U64(l), LlvmValue::U64(r)) => builder
                .build_int_unsigned_div(l, r, "divtmp")
                .map(LlvmValue::U64)
                .map_err(|err| Self::map_err(err, span)),

            (LlvmValue::F64(l), LlvmValue::F64(r)) => builder
                .build_float_div(l, r, "divtmp")
                .map(LlvmValue::F64)
                .map_err(|err| Self::map_err(err, span)),

            (l, r) => Err(Self::type_error("division", l, r, span)),
        }
    }

    // ========================================================================
    // Modulo
    // ========================================================================

    pub fn modulo<'ctx>(
        &self,
        builder: &Builder<'ctx>,
        libc: &LibcFunctions<'ctx>,
        left: LlvmValue<'ctx>,
        right: LlvmValue<'ctx>,
        span: Span,
    ) -> Result<LlvmValue<'ctx>, Box<dyn IError>> {
        match (left, right) {
            (LlvmValue::I8(l), LlvmValue::I8(r)) => {
                self.check_signed_division_overflow(builder, libc, l, r, 8, span)?;

                builder
                    .build_int_signed_rem(l, r, "remtmp")
                    .map(LlvmValue::I8)
                    .map_err(|err| Self::map_err(err, span))
            }

            (LlvmValue::I16(l), LlvmValue::I16(r)) => {
                self.check_signed_division_overflow(builder, libc, l, r, 16, span)?;

                builder
                    .build_int_signed_rem(l, r, "remtmp")
                    .map(LlvmValue::I16)
                    .map_err(|err| Self::map_err(err, span))
            }

            (LlvmValue::I32(l), LlvmValue::I32(r)) => {
                self.check_signed_division_overflow(builder, libc, l, r, 32, span)?;

                builder
                    .build_int_signed_rem(l, r, "remtmp")
                    .map(LlvmValue::I32)
                    .map_err(|err| Self::map_err(err, span))
            }

            (LlvmValue::I64(l), LlvmValue::I64(r)) => {
                self.check_signed_division_overflow(builder, libc, l, r, 64, span)?;

                builder
                    .build_int_signed_rem(l, r, "remtmp")
                    .map(LlvmValue::I64)
                    .map_err(|err| Self::map_err(err, span))
            }

            (LlvmValue::U8(l), LlvmValue::U8(r)) => builder
                .build_int_unsigned_rem(l, r, "remtmp")
                .map(LlvmValue::U8)
                .map_err(|err| Self::map_err(err, span)),

            (LlvmValue::U16(l), LlvmValue::U16(r)) => builder
                .build_int_unsigned_rem(l, r, "remtmp")
                .map(LlvmValue::U16)
                .map_err(|err| Self::map_err(err, span)),

            (LlvmValue::U32(l), LlvmValue::U32(r)) => builder
                .build_int_unsigned_rem(l, r, "remtmp")
                .map(LlvmValue::U32)
                .map_err(|err| Self::map_err(err, span)),

            (LlvmValue::U64(l), LlvmValue::U64(r)) => builder
                .build_int_unsigned_rem(l, r, "remtmp")
                .map(LlvmValue::U64)
                .map_err(|err| Self::map_err(err, span)),

            (l, r) => Err(Self::type_error("modulo", l, r, span)),
        }
    }

    // ========================================================================
    // Generic integer overflow checker
    // ========================================================================

    #[allow(clippy::too_many_arguments)]
    fn checked_integer_operation<'ctx>(
        &self,
        builder: &Builder<'ctx>,
        libc: &LibcFunctions<'ctx>,
        left: IntValue<'ctx>,
        right: IntValue<'ctx>,
        signed: bool,
        bits: u32,
        operation: IntegerBinaryOp,
        span: Span,
    ) -> Result<IntValue<'ctx>, Box<dyn IError>> {
        if self.overflow_policy != OverflowPolicy::Ignore {
            let overflow = self.build_integer_overflow_condition(builder, left, right, signed, bits, operation, span)?;

            self.emit_overflow_branch(
                builder,
                libc,
                overflow,
                span,
                match operation {
                    IntegerBinaryOp::Add => "integer addition overflow",
                    IntegerBinaryOp::Sub => "integer subtraction overflow",
                    IntegerBinaryOp::Mul => "integer multiplication overflow",
                },
            )?;
        }

        let result = match operation {
            IntegerBinaryOp::Add => builder.build_int_add(left, right, "int_add"),

            IntegerBinaryOp::Sub => builder.build_int_sub(left, right, "int_sub"),

            IntegerBinaryOp::Mul => builder.build_int_mul(left, right, "int_mul"),
        }
        .map_err(|err| Self::map_err(err, span))?;

        Ok(result)
    }

    /*
     * Build an overflow condition WITHOUT first calculating the overflowing
     * N-bit result.
     *
     * We extend both operands to i128 and perform the operation there.
     *
     * This is especially useful for i64/u64:
     *
     *     i64 * i64
     *     u64 * u64
     *
     * because i128 can represent the complete mathematical result of either
     * operation.
     *
     * For add/sub this also makes the implementation very simple and avoids
     * subtle signed-overflow formulas.
     */
    #[allow(clippy::too_many_arguments)]
    fn build_integer_overflow_condition<'ctx>(
        &self,
        builder: &Builder<'ctx>,
        left: IntValue<'ctx>,
        right: IntValue<'ctx>,
        signed: bool,
        bits: u32,
        operation: IntegerBinaryOp,
        span: Span,
    ) -> Result<IntValue<'ctx>, Box<dyn IError>> {
        let context = Self::context(builder);
        let wide_type = context.i128_type();

        let left_wide = if signed {
            builder
                .build_int_s_extend(left, wide_type, "overflow.left_sext")
                .map_err(|err| Self::map_err(err, span))?
        } else {
            builder
                .build_int_z_extend(left, wide_type, "overflow.left_zext")
                .map_err(|err| Self::map_err(err, span))?
        };

        let right_wide = if signed {
            builder
                .build_int_s_extend(right, wide_type, "overflow.right_sext")
                .map_err(|err| Self::map_err(err, span))?
        } else {
            builder
                .build_int_z_extend(right, wide_type, "overflow.right_zext")
                .map_err(|err| Self::map_err(err, span))?
        };

        let wide_result = match operation {
            IntegerBinaryOp::Add => builder
                .build_int_add(left_wide, right_wide, "overflow.wide_add")
                .map_err(|err| Self::map_err(err, span))?,

            IntegerBinaryOp::Sub => builder
                .build_int_sub(left_wide, right_wide, "overflow.wide_sub")
                .map_err(|err| Self::map_err(err, span))?,

            IntegerBinaryOp::Mul => builder
                .build_int_mul(left_wide, right_wide, "overflow.wide_mul")
                .map_err(|err| Self::map_err(err, span))?,
        };

        let (min, max) = if signed {
            let max: i128 = (1i128 << (bits - 1)) - 1;
            let min: i128 = -(1i128 << (bits - 1));

            (min, max)
        } else {
            let max: i128 = (1i128 << bits) - 1;

            (0, max)
        };

        let min_const = wide_type.const_int(min as u128 as u64, signed);
        let max_const = wide_type.const_int(max as u128 as u64, signed);

        let below_min = builder
            .build_int_compare(IntPredicate::SLT, wide_result, min_const, "overflow.below_min")
            .map_err(|err| Self::map_err(err, span))?;

        let above_max = builder
            .build_int_compare(
                if signed { IntPredicate::SGT } else { IntPredicate::UGT },
                wide_result,
                max_const,
                "overflow.above_max",
            )
            .map_err(|err| Self::map_err(err, span))?;

        builder
            .build_or(below_min, above_max, "overflow.condition")
            .map_err(|err| Self::map_err(err, span))
    }

    // ========================================================================
    // Signed division special case
    // ========================================================================

    /*
     * Signed integer division has one actual arithmetic overflow case:
     *
     *     MIN / -1
     *
     * Example for i8:
     *
     *     -128 / -1 = 128
     *
     * but 128 is not representable by i8.
     *
     * For modulo MIN % -1 the mathematical result is 0, but LLVM's signed
     * division/remainder semantics still make the MIN/-1 pair special, so we
     * handle it consistently here.
     */
    fn check_signed_division_overflow<'ctx>(
        &self,
        builder: &Builder<'ctx>,
        libc: &LibcFunctions<'ctx>,
        left: IntValue<'ctx>,
        right: IntValue<'ctx>,
        bits: u32,
        span: Span,
    ) -> Result<(), Box<dyn IError>> {
        if self.overflow_policy == OverflowPolicy::Ignore {
            return Ok(());
        }

        let int_type = left.get_type();

        let min_value: i128 = -(1i128 << (bits - 1));

        let min_const = int_type.const_int(min_value as u128 as u64, true);

        let minus_one = int_type.const_int((-1i8) as u8 as u64, true);

        let is_min = builder
            .build_int_compare(IntPredicate::EQ, left, min_const, "div.is_min")
            .map_err(|err| Self::map_err(err, span))?;

        let is_minus_one = builder
            .build_int_compare(IntPredicate::EQ, right, minus_one, "div.is_minus_one")
            .map_err(|err| Self::map_err(err, span))?;

        let overflow = builder
            .build_and(is_min, is_minus_one, "div.overflow")
            .map_err(|err| Self::map_err(err, span))?;

        self.emit_overflow_branch(builder, libc, overflow, span, "signed integer division overflow")
    }

    // ========================================================================
    // Runtime overflow branch
    // ========================================================================

    fn emit_overflow_branch<'ctx>(
        &self,
        builder: &Builder<'ctx>,
        libc: &LibcFunctions<'ctx>,
        overflow: IntValue<'ctx>,
        span: Span,
        description: &str,
    ) -> Result<(), Box<dyn IError>> {
        let context = Self::context(builder);

        let function = builder
            .get_insert_block()
            .expect("builder should be positioned inside a function")
            .get_parent()
            .expect("basic block should belong to a function");

        let overflow_block = context.append_basic_block(function, "overflow.check");

        let continue_block = context.append_basic_block(function, "overflow.continue");

        builder
            .build_conditional_branch(overflow, overflow_block, continue_block)
            .map_err(|err| Self::map_err(err, span))?;

        builder.position_at_end(overflow_block);

        let error = CompilerError::at(self.overflow_policy.severity(), description.to_string(), span);

        let message = format!("{}\n", error.get_stderr_message());

        let format_str = builder
            .build_global_string_ptr(&message, "overflow.msg")
            .map_err(|err| Self::map_err(err, span))?;

        let stderr = builder
            .build_load(context.ptr_type(AddressSpace::default()), libc.stderr.as_pointer_value(), "stderr")
            .map_err(|err| Self::map_err(err, span))?;

        builder
            .build_call(
                libc.fprintf_fn,
                &[stderr.into(), format_str.as_pointer_value().into()],
                "overflow.fprintf",
            )
            .map_err(|err| Self::map_err(err, span))?;

        if self.overflow_policy == OverflowPolicy::Error {
            let i32_type = context.i32_type();

            builder
                .build_call(libc.exit_fn, &[i32_type.const_int(1, false).into()], "overflow.exit")
                .map_err(|err| Self::map_err(err, span))?;

            builder.build_unreachable().map_err(|err| Self::map_err(err, span))?;
        } else {
            builder
                .build_unconditional_branch(continue_block)
                .map_err(|err| Self::map_err(err, span))?;
        }

        builder.position_at_end(continue_block);

        Ok(())
    }
}
