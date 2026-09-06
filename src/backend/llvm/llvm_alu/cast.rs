use inkwell::{
    builder::Builder,
    values::{FloatValue, IntValue},
    AddressSpace, FloatPredicate, IntPredicate,
};

use crate::{
    backend::llvm::{
        libc_functions::LibcFunctions,
        llvm_alu::{llvm_value::LlvmValue, LlvmAlu, OverflowPolicy},
    },
    common::{
        errors::{CompilerError, ErrorSeverity, IError},
        span::Span,
        types::Type,
    },
};

impl LlvmAlu {
    pub fn cast_to_type<'ctx>(
        &self,
        builder: &Builder<'ctx>,
        libc: &LibcFunctions<'ctx>,
        value: LlvmValue<'ctx>,
        to_type: &Type,
        span: Span,
    ) -> Result<LlvmValue<'ctx>, Box<dyn IError>> {
        macro_rules! int_cast {
            ($value:expr, $bits:expr, $source_signed:expr, $target_signed:expr) => {
                self.int_cast(builder, libc, $value, $bits, $source_signed, $target_signed, span)
            };
        }

        match (value, to_type) {
            // Same type (Str excluded - casting str->str still needs a
            // fresh, independently owned copy; see the explicit arm below).
            (value, target_type) if value.to_type() == *target_type && !matches!(value, LlvmValue::Str(_)) => Ok(value),

            // String -> String (identity "cast"): still produces a fresh,
            // independently owned copy, consistent with every other string
            // assignment in the language.
            (LlvmValue::Str(value), Type::Str) => {
                let data = Self::str_data_ptr(builder, value, span)?;

                let len = builder
                    .build_call(libc.strlen_fn, &[data.into()], "str_identity.len")
                    .map_err(|err| Self::map_err(err, span))?
                    .try_as_basic_value()
                    .basic()
                    .expect("strlen should return a value")
                    .into_int_value();

                let size = builder
                    .build_int_add(len, len.get_type().const_int(1, false), "str_identity.size")
                    .map_err(|err| Self::map_err(err, span))?;

                let new_data = builder
                    .build_call(libc.malloc_fn, &[size.into()], "str_identity.malloc")
                    .map_err(|err| Self::map_err(err, span))?
                    .try_as_basic_value()
                    .basic()
                    .expect("malloc should return a value")
                    .into_pointer_value();

                builder
                    .build_call(libc.strcpy_fn, &[new_data.into(), data.into()], "str_identity.strcpy")
                    .map_err(|err| Self::map_err(err, span))?;

                Self::wrap_str_header(builder, libc, new_data, span).map(LlvmValue::Str)
            }

            // ============================================================
            // Integer -> Integer
            // ============================================================

            // I8
            (LlvmValue::I8(value), Type::I8) => Ok(LlvmValue::I8(value)),
            (LlvmValue::I8(value), Type::I16) => LlvmAlu::int_extend_signed(builder, value, 16, span),
            (LlvmValue::I8(value), Type::I32) => LlvmAlu::int_extend_signed(builder, value, 32, span),
            (LlvmValue::I8(value), Type::I64) => LlvmAlu::int_extend_signed(builder, value, 64, span),
            (LlvmValue::I8(value), Type::U8) => {
                int_cast!(value, 8, true, false)
            }
            (LlvmValue::I8(value), Type::U16) => LlvmAlu::int_extend_signed(builder, value, 16, span),
            (LlvmValue::I8(value), Type::U32) => LlvmAlu::int_extend_signed(builder, value, 32, span),
            (LlvmValue::I8(value), Type::U64) => LlvmAlu::int_extend_signed(builder, value, 64, span),

            // I16
            (LlvmValue::I16(value), Type::I8) => {
                int_cast!(value, 8, true, true)
            }
            (LlvmValue::I16(value), Type::I16) => Ok(LlvmValue::I16(value)),
            (LlvmValue::I16(value), Type::I32) => LlvmAlu::int_extend_signed(builder, value, 32, span),
            (LlvmValue::I16(value), Type::I64) => LlvmAlu::int_extend_signed(builder, value, 64, span),
            (LlvmValue::I16(value), Type::U8) => {
                int_cast!(value, 8, true, false)
            }
            (LlvmValue::I16(value), Type::U16) => {
                int_cast!(value, 16, true, false)
            }
            (LlvmValue::I16(value), Type::U32) => LlvmAlu::int_extend_signed(builder, value, 32, span),
            (LlvmValue::I16(value), Type::U64) => LlvmAlu::int_extend_signed(builder, value, 64, span),

            // I32
            (LlvmValue::I32(value), Type::I8) => {
                int_cast!(value, 8, true, true)
            }
            (LlvmValue::I32(value), Type::I16) => {
                int_cast!(value, 16, true, true)
            }
            (LlvmValue::I32(value), Type::I32) => Ok(LlvmValue::I32(value)),
            (LlvmValue::I32(value), Type::I64) => LlvmAlu::int_extend_signed(builder, value, 64, span),
            (LlvmValue::I32(value), Type::U8) => {
                int_cast!(value, 8, true, false)
            }
            (LlvmValue::I32(value), Type::U16) => {
                int_cast!(value, 16, true, false)
            }
            (LlvmValue::I32(value), Type::U32) => {
                int_cast!(value, 32, true, false)
            }
            (LlvmValue::I32(value), Type::U64) => LlvmAlu::int_extend_signed(builder, value, 64, span),

            // I64
            (LlvmValue::I64(value), Type::I8) => {
                int_cast!(value, 8, true, true)
            }
            (LlvmValue::I64(value), Type::I16) => {
                int_cast!(value, 16, true, true)
            }
            (LlvmValue::I64(value), Type::I32) => {
                int_cast!(value, 32, true, true)
            }
            (LlvmValue::I64(value), Type::I64) => Ok(LlvmValue::I64(value)),
            (LlvmValue::I64(value), Type::U8) => {
                int_cast!(value, 8, true, false)
            }
            (LlvmValue::I64(value), Type::U16) => {
                int_cast!(value, 16, true, false)
            }
            (LlvmValue::I64(value), Type::U32) => {
                int_cast!(value, 32, true, false)
            }
            (LlvmValue::I64(value), Type::U64) => {
                int_cast!(value, 64, true, false)
            }

            // U8
            (LlvmValue::U8(value), Type::I8) => {
                int_cast!(value, 8, false, true)
            }
            (LlvmValue::U8(value), Type::I16) => LlvmAlu::int_extend_unsigned(builder, value, 16, span),
            (LlvmValue::U8(value), Type::I32) => LlvmAlu::int_extend_unsigned(builder, value, 32, span),
            (LlvmValue::U8(value), Type::I64) => LlvmAlu::int_extend_unsigned(builder, value, 64, span),
            (LlvmValue::U8(value), Type::U8) => Ok(LlvmValue::U8(value)),
            (LlvmValue::U8(value), Type::U16) => LlvmAlu::int_extend_unsigned(builder, value, 16, span),
            (LlvmValue::U8(value), Type::U32) => LlvmAlu::int_extend_unsigned(builder, value, 32, span),
            (LlvmValue::U8(value), Type::U64) => LlvmAlu::int_extend_unsigned(builder, value, 64, span),

            // U16
            (LlvmValue::U16(value), Type::I8) => {
                int_cast!(value, 8, false, true)
            }
            (LlvmValue::U16(value), Type::I16) => {
                int_cast!(value, 16, false, true)
            }
            (LlvmValue::U16(value), Type::I32) => LlvmAlu::int_extend_unsigned(builder, value, 32, span),
            (LlvmValue::U16(value), Type::I64) => LlvmAlu::int_extend_unsigned(builder, value, 64, span),
            (LlvmValue::U16(value), Type::U8) => {
                int_cast!(value, 8, false, false)
            }
            (LlvmValue::U16(value), Type::U16) => Ok(LlvmValue::U16(value)),
            (LlvmValue::U16(value), Type::U32) => LlvmAlu::int_extend_unsigned(builder, value, 32, span),
            (LlvmValue::U16(value), Type::U64) => LlvmAlu::int_extend_unsigned(builder, value, 64, span),

            // U32
            (LlvmValue::U32(value), Type::I8) => {
                int_cast!(value, 8, false, true)
            }
            (LlvmValue::U32(value), Type::I16) => {
                int_cast!(value, 16, false, true)
            }
            (LlvmValue::U32(value), Type::I32) => {
                int_cast!(value, 32, false, true)
            }
            (LlvmValue::U32(value), Type::I64) => LlvmAlu::int_extend_unsigned(builder, value, 64, span),
            (LlvmValue::U32(value), Type::U8) => {
                int_cast!(value, 8, false, false)
            }
            (LlvmValue::U32(value), Type::U16) => {
                int_cast!(value, 16, false, false)
            }
            (LlvmValue::U32(value), Type::U32) => Ok(LlvmValue::U32(value)),
            (LlvmValue::U32(value), Type::U64) => LlvmAlu::int_extend_unsigned(builder, value, 64, span),

            // U64
            (LlvmValue::U64(value), Type::I8) => {
                int_cast!(value, 8, false, true)
            }
            (LlvmValue::U64(value), Type::I16) => {
                int_cast!(value, 16, false, true)
            }
            (LlvmValue::U64(value), Type::I32) => {
                int_cast!(value, 32, false, true)
            }
            (LlvmValue::U64(value), Type::I64) => {
                int_cast!(value, 64, false, true)
            }
            (LlvmValue::U64(value), Type::U8) => {
                int_cast!(value, 8, false, false)
            }
            (LlvmValue::U64(value), Type::U16) => {
                int_cast!(value, 16, false, false)
            }
            (LlvmValue::U64(value), Type::U32) => {
                int_cast!(value, 32, false, false)
            }
            (LlvmValue::U64(value), Type::U64) => Ok(LlvmValue::U64(value)),

            // ============================================================
            // Integer -> F64
            // ============================================================
            (LlvmValue::I8(value), Type::F64) => LlvmAlu::signed_int_to_float(builder, value, span),
            (LlvmValue::I16(value), Type::F64) => LlvmAlu::signed_int_to_float(builder, value, span),
            (LlvmValue::I32(value), Type::F64) => LlvmAlu::signed_int_to_float(builder, value, span),
            (LlvmValue::I64(value), Type::F64) => LlvmAlu::signed_int_to_float(builder, value, span),

            (LlvmValue::U8(value), Type::F64) => LlvmAlu::unsigned_int_to_float(builder, value, span),
            (LlvmValue::U16(value), Type::F64) => LlvmAlu::unsigned_int_to_float(builder, value, span),
            (LlvmValue::U32(value), Type::F64) => LlvmAlu::unsigned_int_to_float(builder, value, span),
            (LlvmValue::U64(value), Type::F64) => LlvmAlu::unsigned_int_to_float(builder, value, span),

            // ============================================================
            // F64 -> Integer
            // ============================================================
            (LlvmValue::F64(value), Type::I8) => LlvmAlu::float_to_signed_int(builder, value, 8, span),
            (LlvmValue::F64(value), Type::I16) => LlvmAlu::float_to_signed_int(builder, value, 16, span),
            (LlvmValue::F64(value), Type::I32) => LlvmAlu::float_to_signed_int(builder, value, 32, span),
            (LlvmValue::F64(value), Type::I64) => LlvmAlu::float_to_signed_int(builder, value, 64, span),

            (LlvmValue::F64(value), Type::U8) => LlvmAlu::float_to_unsigned_int(builder, value, 8, span),
            (LlvmValue::F64(value), Type::U16) => LlvmAlu::float_to_unsigned_int(builder, value, 16, span),
            (LlvmValue::F64(value), Type::U32) => LlvmAlu::float_to_unsigned_int(builder, value, 32, span),
            (LlvmValue::F64(value), Type::U64) => LlvmAlu::float_to_unsigned_int(builder, value, 64, span),

            // ============================================================
            // Integer -> Bool
            // ============================================================
            (LlvmValue::I8(value), Type::Bool) => LlvmAlu::int_to_bool(builder, value, true, span),
            (LlvmValue::I16(value), Type::Bool) => LlvmAlu::int_to_bool(builder, value, true, span),
            (LlvmValue::I32(value), Type::Bool) => LlvmAlu::int_to_bool(builder, value, true, span),
            (LlvmValue::I64(value), Type::Bool) => LlvmAlu::int_to_bool(builder, value, true, span),

            (LlvmValue::U8(value), Type::Bool) => LlvmAlu::int_to_bool(builder, value, false, span),
            (LlvmValue::U16(value), Type::Bool) => LlvmAlu::int_to_bool(builder, value, false, span),
            (LlvmValue::U32(value), Type::Bool) => LlvmAlu::int_to_bool(builder, value, false, span),
            (LlvmValue::U64(value), Type::Bool) => LlvmAlu::int_to_bool(builder, value, false, span),

            // ============================================================
            // F64 -> Bool
            // ============================================================
            (LlvmValue::F64(value), Type::Bool) => {
                let zero = value.get_type().const_float(0.0);

                builder
                    .build_float_compare(FloatPredicate::OGT, value, zero, "f64_to_bool")
                    .map(LlvmValue::Bool)
                    .map_err(|err| Self::map_err(err, span))
            }

            // ============================================================
            // String -> Integer
            // ============================================================
            (LlvmValue::Str(value), Type::I8) => Self::string_to_signed_int(builder, libc, value, 8, span),
            (LlvmValue::Str(value), Type::I16) => Self::string_to_signed_int(builder, libc, value, 16, span),
            (LlvmValue::Str(value), Type::I32) => Self::string_to_signed_int(builder, libc, value, 32, span),
            (LlvmValue::Str(value), Type::I64) => Self::string_to_signed_int(builder, libc, value, 64, span),

            (LlvmValue::Str(value), Type::U8) => Self::string_to_unsigned_int(builder, libc, value, 8, span),
            (LlvmValue::Str(value), Type::U16) => Self::string_to_unsigned_int(builder, libc, value, 16, span),
            (LlvmValue::Str(value), Type::U32) => Self::string_to_unsigned_int(builder, libc, value, 32, span),
            (LlvmValue::Str(value), Type::U64) => Self::string_to_unsigned_int(builder, libc, value, 64, span),

            // ============================================================
            // String -> F64
            // ============================================================
            (LlvmValue::Str(value), Type::F64) => {
                let data = Self::str_data_ptr(builder, value, span)?;

                let call = builder
                    .build_call(libc.atof_fn, &[data.into()], "atof_call")
                    .map_err(|err| Self::map_err(err, span))?;

                let value = call.try_as_basic_value().basic().expect("atof should return a value").into_float_value();

                Ok(LlvmValue::F64(value))
            }

            // ============================================================
            // String -> Bool
            // ============================================================
            (LlvmValue::Str(value), Type::Bool) => {
                let data = Self::str_data_ptr(builder, value, span)?;

                let call = builder
                    .build_call(libc.strlen_fn, &[data.into()], "strlen_call")
                    .map_err(|err| Self::map_err(err, span))?;

                let len = call.try_as_basic_value().basic().expect("strlen should return a value").into_int_value();

                let zero = len.get_type().const_zero();

                builder
                    .build_int_compare(IntPredicate::NE, len, zero, "str_to_bool")
                    .map(LlvmValue::Bool)
                    .map_err(|err| Self::map_err(err, span))
            }

            // ============================================================
            // Integer -> String
            // ============================================================
            (LlvmValue::I8(value), Type::Str) => Self::signed_int_to_str(builder, libc, value, 8, span),
            (LlvmValue::I16(value), Type::Str) => Self::signed_int_to_str(builder, libc, value, 16, span),
            (LlvmValue::I32(value), Type::Str) => Self::signed_int_to_str(builder, libc, value, 32, span),
            (LlvmValue::I64(value), Type::Str) => Self::signed_int_to_str(builder, libc, value, 64, span),

            (LlvmValue::U8(value), Type::Str) => Self::unsigned_int_to_str(builder, libc, value, 8, span),
            (LlvmValue::U16(value), Type::Str) => Self::unsigned_int_to_str(builder, libc, value, 16, span),
            (LlvmValue::U32(value), Type::Str) => Self::unsigned_int_to_str(builder, libc, value, 32, span),
            (LlvmValue::U64(value), Type::Str) => Self::unsigned_int_to_str(builder, libc, value, 64, span),

            // ============================================================
            // F64 -> String
            // ============================================================
            (LlvmValue::F64(value), Type::Str) => Self::float_to_str(builder, libc, value, span),

            // ============================================================
            // Bool -> String
            // ============================================================
            (LlvmValue::Bool(value), Type::Str) => Self::bool_to_str(builder, libc, value, span),

            // ============================================================
            // Bool -> Integer
            // ============================================================
            (LlvmValue::Bool(value), Type::I8) => Self::bool_to_int(builder, value, 8, span),
            (LlvmValue::Bool(value), Type::I16) => Self::bool_to_int(builder, value, 16, span),
            (LlvmValue::Bool(value), Type::I32) => Self::bool_to_int(builder, value, 32, span),
            (LlvmValue::Bool(value), Type::I64) => Self::bool_to_int(builder, value, 64, span),

            (LlvmValue::Bool(value), Type::U8) => Self::bool_to_int(builder, value, 8, span),
            (LlvmValue::Bool(value), Type::U16) => Self::bool_to_int(builder, value, 16, span),
            (LlvmValue::Bool(value), Type::U32) => Self::bool_to_int(builder, value, 32, span),
            (LlvmValue::Bool(value), Type::U64) => Self::bool_to_int(builder, value, 64, span),

            // ============================================================
            // Bool -> F64
            // ============================================================
            (LlvmValue::Bool(value), Type::F64) => builder
                .build_unsigned_int_to_float(value, Self::context(builder).f64_type(), "bool_to_f64")
                .map(LlvmValue::F64)
                .map_err(|err| Self::map_err(err, span)),

            // ============================================================
            // Char -> String
            // ============================================================
            (LlvmValue::Char(value), Type::Str) => Self::char_to_str(builder, libc, value, span),

            // ============================================================
            // Char <-> U8
            // ============================================================
            (LlvmValue::Char(value), Type::U8) => Ok(LlvmValue::U8(value)),
            (LlvmValue::U8(value), Type::Char) => Ok(LlvmValue::Char(value)),

            // ============================================================
            // Unsupported
            // ============================================================
            (value, target_type) => Err(Box::new(CompilerError::at(
                ErrorSeverity::HIGH,
                format!("Cannot cast '{}' to '{}'.", value.to_type(), target_type),
                span,
            ))),
        }
    }

    // ========================================================================
    // Integer casts + overflow checking
    // ========================================================================

    #[allow(clippy::too_many_arguments)]
    fn int_cast<'ctx>(
        &self,
        builder: &Builder<'ctx>,
        libc: &LibcFunctions<'ctx>,
        value: IntValue<'ctx>,
        target_bits: u32,
        source_signed: bool,
        target_signed: bool,
        span: Span,
    ) -> Result<LlvmValue<'ctx>, Box<dyn IError>> {
        let source_bits = value.get_type().get_bit_width();
        let target = Self::int_type(builder, target_bits)?;

        /*
         * Check the range before doing the actual LLVM conversion.
         *
         * Important:
         *
         *   i8  -> u8
         *   i16 -> u16
         *   i32 -> u32
         *   i64 -> u64
         *
         * are NOT safe just because the bit width is identical.
         *
         * Likewise:
         *
         *   u8  -> i8
         *   u16 -> i16
         *   u32 -> i32
         *   u64 -> i64
         *
         * can overflow even though there is no LLVM truncation instruction.
         */
        if self.overflow_policy != OverflowPolicy::Ignore && !Self::integer_range_is_subset(source_bits, source_signed, target_bits, target_signed) {
            self.emit_integer_overflow_check(builder, libc, value, source_bits, source_signed, target_bits, target_signed, span)?;
        }

        let casted = if source_bits == target_bits {
            // No LLVM instruction is necessary. The bit pattern is already
            // exactly the target width; only its semantic signedness changes.
            value
        } else if source_bits > target_bits {
            builder
                .build_int_truncate(value, target, "trunc")
                .map_err(|err| Self::map_err(err, span))?
        } else if source_signed {
            builder
                .build_int_s_extend(value, target, "sext")
                .map_err(|err| Self::map_err(err, span))?
        } else {
            builder
                .build_int_z_extend(value, target, "zext")
                .map_err(|err| Self::map_err(err, span))?
        };

        Ok(match (target_bits, target_signed) {
            (8, true) => LlvmValue::I8(casted),
            (8, false) => LlvmValue::U8(casted),
            (16, true) => LlvmValue::I16(casted),
            (16, false) => LlvmValue::U16(casted),
            (32, true) => LlvmValue::I32(casted),
            (32, false) => LlvmValue::U32(casted),
            (64, true) => LlvmValue::I64(casted),
            (64, false) => LlvmValue::U64(casted),
            _ => unreachable!(),
        })
    }

    /// Returns true when every value representable by the source type is
    /// representable by the target type.
    fn integer_range_is_subset(source_bits: u32, source_signed: bool, target_bits: u32, target_signed: bool) -> bool {
        match (source_signed, target_signed) {
            // Same signedness:
            //
            // signed N -> signed M is safe when M >= N.
            // unsigned N -> unsigned M is safe when M >= N.
            (true, true) | (false, false) => source_bits <= target_bits,

            // Signed -> unsigned:
            //
            // i8  -> u16 is safe.
            // i16 -> u32 is safe.
            //
            // But:
            //
            // i8  -> u8  is NOT safe because negative values exist.
            // i16 -> u8  is NOT safe because both negative and large
            //              positive values can be out of range.
            (true, false) => source_bits < target_bits,

            // Unsigned -> signed:
            //
            // u8  -> i16 is safe.
            // u16 -> i32 is safe.
            //
            // Same width is NOT safe:
            //
            // u8 -> i8
            // u16 -> i16
            // u32 -> i32
            // u64 -> i64
            (false, true) => source_bits < target_bits,
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn emit_integer_overflow_check<'ctx>(
        &self,
        builder: &Builder<'ctx>,
        libc: &LibcFunctions<'ctx>,
        value: IntValue<'ctx>,
        source_bits: u32,
        source_signed: bool,
        target_bits: u32,
        target_signed: bool,
        span: Span,
    ) -> Result<(), Box<dyn IError>> {
        if self.overflow_policy == OverflowPolicy::Ignore {
            return Ok(());
        }

        let context = Self::context(builder);
        let source_type = value.get_type();

        let zero = source_type.const_zero();

        /*
         * All constants are created in the SOURCE integer type.
         *
         * We deliberately do not use the existing generic
         * emit_overflow_check() here because a target such as u64 has a
         * maximum value that cannot be represented as an i64 constant when
         * the source itself is signed.
         *
         * Example:
         *
         *     i64 -> u64
         *
         * is only invalid for negative values. Trying to encode
         * u64::MAX as a signed i64 constant would turn it into -1 and make
         * the comparison incorrect.
         */
        let out_of_range = match (source_signed, target_signed) {
            // ------------------------------------------------------------
            // signed -> signed
            // ------------------------------------------------------------
            (true, true) => {
                debug_assert!(target_bits < source_bits);

                let min = -(1i128 << (target_bits - 1));
                let max = (1i128 << (target_bits - 1)) - 1;

                let min_const = source_type.const_int(min as u64, true);
                let max_const = source_type.const_int(max as u64, true);

                let lt_min = builder
                    .build_int_compare(IntPredicate::SLT, value, min_const, "overflow.lt_min")
                    .map_err(|err| Self::map_err(err, span))?;

                let gt_max = builder
                    .build_int_compare(IntPredicate::SGT, value, max_const, "overflow.gt_max")
                    .map_err(|err| Self::map_err(err, span))?;

                builder
                    .build_or(lt_min, gt_max, "overflow.out_of_range")
                    .map_err(|err| Self::map_err(err, span))?
            }

            // ------------------------------------------------------------
            // unsigned -> unsigned
            // ------------------------------------------------------------
            (false, false) => {
                debug_assert!(target_bits < source_bits);

                let max = (1u128 << target_bits) - 1;
                let max_const = source_type.const_int(max as u64, false);

                builder
                    .build_int_compare(IntPredicate::UGT, value, max_const, "overflow.gt_max")
                    .map_err(|err| Self::map_err(err, span))?
            }

            // ------------------------------------------------------------
            // signed -> unsigned
            // ------------------------------------------------------------
            (true, false) => {
                /*
                 * There are two independent failure conditions:
                 *
                 *   value < 0
                 *
                 * and, for a narrower target:
                 *
                 *   value > UINT_MAX(target)
                 *
                 * For same-width iN -> uN, the second condition is not
                 * needed because every non-negative iN already fits into uN.
                 */
                let negative = builder
                    .build_int_compare(IntPredicate::SLT, value, zero, "overflow.negative")
                    .map_err(|err| Self::map_err(err, span))?;

                let too_large = if target_bits < source_bits {
                    let max = (1u128 << target_bits) - 1;
                    let max_const = source_type.const_int(max as u64, false);

                    builder
                        .build_int_compare(IntPredicate::UGT, value, max_const, "overflow.gt_max")
                        .map_err(|err| Self::map_err(err, span))?
                } else {
                    context.bool_type().const_zero()
                };

                builder
                    .build_or(negative, too_large, "overflow.out_of_range")
                    .map_err(|err| Self::map_err(err, span))?
            }

            // ------------------------------------------------------------
            // unsigned -> signed
            // ------------------------------------------------------------
            (false, true) => {
                debug_assert!(target_bits <= source_bits);

                let max = (1i128 << (target_bits - 1)) - 1;
                let max_const = source_type.const_int(max as u64, false);

                builder
                    .build_int_compare(IntPredicate::UGT, value, max_const, "overflow.gt_max")
                    .map_err(|err| Self::map_err(err, span))?
            }
        };

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
                "Integer overflow in cast ({} -> {}:{})",
                value.get_type().get_bit_width(),
                target_bits,
                if target_signed { "signed" } else { "unsigned" }
            ),
            span,
        );

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
            builder.build_unconditional_branch(merge_block).map_err(|err| Self::map_err(err, span))?;
        }

        builder.position_at_end(merge_block);

        Ok(())
    }

    fn char_to_str<'ctx>(
        builder: &Builder<'ctx>,
        libc: &LibcFunctions<'ctx>,
        value: IntValue<'ctx>,
        span: Span,
    ) -> Result<LlvmValue<'ctx>, Box<dyn IError>> {
        let context = Self::context(builder);
        let i32_type = context.i32_type();

        let promoted = builder
            .build_int_z_extend(value, i32_type, "char_to_int")
            .map_err(|err| Self::map_err(err, span))?;

        Self::int_to_str_via_snprintf(builder, libc, promoted, "%c", span)
    }

    pub fn context<'ctx>(builder: &Builder<'ctx>) -> inkwell::context::ContextRef<'ctx> {
        builder
            .get_insert_block()
            .expect("builder should be positioned inside a block")
            .get_context()
    }

    fn int_type<'ctx>(builder: &Builder<'ctx>, bits: u32) -> Result<inkwell::types::IntType<'ctx>, Box<dyn IError>> {
        let bits = std::num::NonZeroU32::new(bits).ok_or_else(|| {
            Box::new(CompilerError::at(
                ErrorSeverity::HIGH,
                "Integer type width cannot be zero.".to_string(),
                Span::default(),
            )) as Box<dyn IError>
        })?;

        Self::context(builder).custom_width_int_type(bits).map_err(|err| {
            Box::new(CompilerError::at(
                ErrorSeverity::HIGH,
                format!("Cannot create integer type: {}.", err),
                Span::default(),
            )) as Box<dyn IError>
        })
    }

    fn int_extend_signed<'ctx>(builder: &Builder<'ctx>, value: IntValue<'ctx>, bits: u32, span: Span) -> Result<LlvmValue<'ctx>, Box<dyn IError>> {
        let target = Self::int_type(builder, bits)?;

        builder
            .build_int_s_extend(value, target, "sext")
            .map(|value| match bits {
                8 => LlvmValue::I8(value),
                16 => LlvmValue::I16(value),
                32 => LlvmValue::I32(value),
                64 => LlvmValue::I64(value),
                _ => unreachable!(),
            })
            .map_err(|err| Self::map_err(err, span))
    }

    fn int_extend_unsigned<'ctx>(builder: &Builder<'ctx>, value: IntValue<'ctx>, bits: u32, span: Span) -> Result<LlvmValue<'ctx>, Box<dyn IError>> {
        let target = Self::int_type(builder, bits)?;

        builder
            .build_int_z_extend(value, target, "zext")
            .map(|value| match bits {
                8 => LlvmValue::U8(value),
                16 => LlvmValue::U16(value),
                32 => LlvmValue::U32(value),
                64 => LlvmValue::U64(value),
                _ => unreachable!(),
            })
            .map_err(|err| Self::map_err(err, span))
    }

    fn signed_int_to_float<'ctx>(builder: &Builder<'ctx>, value: IntValue<'ctx>, span: Span) -> Result<LlvmValue<'ctx>, Box<dyn IError>> {
        builder
            .build_signed_int_to_float(value, Self::context(builder).f64_type(), "int_to_f64")
            .map(LlvmValue::F64)
            .map_err(|err| Self::map_err(err, span))
    }

    fn unsigned_int_to_float<'ctx>(builder: &Builder<'ctx>, value: IntValue<'ctx>, span: Span) -> Result<LlvmValue<'ctx>, Box<dyn IError>> {
        builder
            .build_unsigned_int_to_float(value, Self::context(builder).f64_type(), "uint_to_f64")
            .map(LlvmValue::F64)
            .map_err(|err| Self::map_err(err, span))
    }

    fn float_to_signed_int<'ctx>(
        builder: &Builder<'ctx>,
        value: FloatValue<'ctx>,
        bits: u32,
        span: Span,
    ) -> Result<LlvmValue<'ctx>, Box<dyn IError>> {
        let target = Self::int_type(builder, bits)?;

        builder
            .build_float_to_signed_int(value, target, "f64_to_int")
            .map(|value| match bits {
                8 => LlvmValue::I8(value),
                16 => LlvmValue::I16(value),
                32 => LlvmValue::I32(value),
                64 => LlvmValue::I64(value),
                _ => unreachable!(),
            })
            .map_err(|err| Self::map_err(err, span))
    }

    fn float_to_unsigned_int<'ctx>(
        builder: &Builder<'ctx>,
        value: FloatValue<'ctx>,
        bits: u32,
        span: Span,
    ) -> Result<LlvmValue<'ctx>, Box<dyn IError>> {
        let target = Self::int_type(builder, bits)?;

        builder
            .build_float_to_unsigned_int(value, target, "f64_to_uint")
            .map(|value| match bits {
                8 => LlvmValue::U8(value),
                16 => LlvmValue::U16(value),
                32 => LlvmValue::U32(value),
                64 => LlvmValue::U64(value),
                _ => unreachable!(),
            })
            .map_err(|err| Self::map_err(err, span))
    }

    fn int_to_bool<'ctx>(builder: &Builder<'ctx>, value: IntValue<'ctx>, signed: bool, span: Span) -> Result<LlvmValue<'ctx>, Box<dyn IError>> {
        let zero = value.get_type().const_zero();

        let predicate = if signed { IntPredicate::SGT } else { IntPredicate::UGT };

        builder
            .build_int_compare(predicate, value, zero, "int_to_bool")
            .map(LlvmValue::Bool)
            .map_err(|err| Self::map_err(err, span))
    }

    fn bool_to_int<'ctx>(builder: &Builder<'ctx>, value: IntValue<'ctx>, bits: u32, span: Span) -> Result<LlvmValue<'ctx>, Box<dyn IError>> {
        let target = Self::int_type(builder, bits)?;

        builder
            .build_int_z_extend(value, target, "bool_to_int")
            .map(|value| match bits {
                8 => LlvmValue::I8(value),
                16 => LlvmValue::I16(value),
                32 => LlvmValue::I32(value),
                64 => LlvmValue::I64(value),
                _ => unreachable!(),
            })
            .map_err(|err| Self::map_err(err, span))
    }

    fn string_to_signed_int<'ctx>(
        builder: &Builder<'ctx>,
        libc: &LibcFunctions<'ctx>,
        value: inkwell::values::PointerValue<'ctx>,
        bits: u32,
        span: Span,
    ) -> Result<LlvmValue<'ctx>, Box<dyn IError>> {
        let data = Self::str_data_ptr(builder, value, span)?;

        let call = builder
            .build_call(libc.atoll_fn, &[data.into()], "string_to_int")
            .map_err(|err| Self::map_err(err, span))?;

        let parsed = call.try_as_basic_value().basic().expect("atoll should return a value").into_int_value();

        if bits == 64 {
            return Ok(LlvmValue::I64(parsed));
        }

        Self::int_cast_string_signed(builder, parsed, bits, span)
    }

    fn string_to_unsigned_int<'ctx>(
        builder: &Builder<'ctx>,
        libc: &LibcFunctions<'ctx>,
        value: inkwell::values::PointerValue<'ctx>,
        bits: u32,
        span: Span,
    ) -> Result<LlvmValue<'ctx>, Box<dyn IError>> {
        let data = Self::str_data_ptr(builder, value, span)?;

        let call = builder
            .build_call(libc.atoll_fn, &[data.into()], "string_to_uint")
            .map_err(|err| Self::map_err(err, span))?;

        let parsed = call.try_as_basic_value().basic().expect("atoll should return a value").into_int_value();

        if bits == 64 {
            return Ok(LlvmValue::U64(parsed));
        }

        Self::int_cast_string_unsigned(builder, parsed, bits, span)
    }

    fn int_cast_string_signed<'ctx>(
        builder: &Builder<'ctx>,
        value: IntValue<'ctx>,
        bits: u32,
        span: Span,
    ) -> Result<LlvmValue<'ctx>, Box<dyn IError>> {
        let target = Self::int_type(builder, bits)?;

        let casted = if value.get_type().get_bit_width() > bits {
            builder
                .build_int_truncate(value, target, "int_trunc")
                .map_err(|err| Self::map_err(err, span))?
        } else {
            builder
                .build_int_s_extend(value, target, "int_sext")
                .map_err(|err| Self::map_err(err, span))?
        };

        Ok(match bits {
            8 => LlvmValue::I8(casted),
            16 => LlvmValue::I16(casted),
            32 => LlvmValue::I32(casted),
            64 => LlvmValue::I64(casted),
            _ => unreachable!(),
        })
    }

    fn int_cast_string_unsigned<'ctx>(
        builder: &Builder<'ctx>,
        value: IntValue<'ctx>,
        bits: u32,
        span: Span,
    ) -> Result<LlvmValue<'ctx>, Box<dyn IError>> {
        let target = Self::int_type(builder, bits)?;

        let casted = if value.get_type().get_bit_width() > bits {
            builder
                .build_int_truncate(value, target, "uint_trunc")
                .map_err(|err| Self::map_err(err, span))?
        } else {
            builder
                .build_int_z_extend(value, target, "uint_zext")
                .map_err(|err| Self::map_err(err, span))?
        };

        Ok(match bits {
            8 => LlvmValue::U8(casted),
            16 => LlvmValue::U16(casted),
            32 => LlvmValue::U32(casted),
            64 => LlvmValue::U64(casted),
            _ => unreachable!(),
        })
    }

    pub(in crate::backend::llvm::llvm_alu) fn bool_to_str<'ctx>(
        builder: &Builder<'ctx>,
        libc: &LibcFunctions<'ctx>,
        value: IntValue<'ctx>,
        span: Span,
    ) -> Result<LlvmValue<'ctx>, Box<dyn IError>> {
        let true_str = builder
            .build_global_string_ptr("true", "true_str")
            .map_err(|err| Self::map_err(err, span))?;

        let false_str = builder
            .build_global_string_ptr("false", "false_str")
            .map_err(|err| Self::map_err(err, span))?;

        let picked = builder
            .build_select(value, true_str.as_pointer_value(), false_str.as_pointer_value(), "bool_to_str")
            .map_err(|err| Self::map_err(err, span))?
            .into_pointer_value();

        // `picked` points at a read-only global constant - heap-duplicate
        // it before wrapping in a header, since `release_str` will
        // eventually `free()` whatever the header's `data` field points to.
        let len = builder
            .build_call(libc.strlen_fn, &[picked.into()], "bool_to_str.len")
            .map_err(|err| Self::map_err(err, span))?
            .try_as_basic_value()
            .basic()
            .expect("strlen should return a value")
            .into_int_value();

        let size = builder
            .build_int_add(len, len.get_type().const_int(1, false), "bool_to_str.size")
            .map_err(|err| Self::map_err(err, span))?;

        let dup = builder
            .build_call(libc.malloc_fn, &[size.into()], "bool_to_str.malloc")
            .map_err(|err| Self::map_err(err, span))?
            .try_as_basic_value()
            .basic()
            .expect("malloc should return a value")
            .into_pointer_value();

        builder
            .build_call(libc.strcpy_fn, &[dup.into(), picked.into()], "bool_to_str.strcpy")
            .map_err(|err| Self::map_err(err, span))?;

        Self::wrap_str_header(builder, libc, dup, span).map(LlvmValue::Str)
    }

    fn signed_int_to_str<'ctx>(
        builder: &Builder<'ctx>,
        libc: &LibcFunctions<'ctx>,
        value: IntValue<'ctx>,
        bits: u32,
        span: Span,
    ) -> Result<LlvmValue<'ctx>, Box<dyn IError>> {
        let context = Self::context(builder);

        let (promoted, fmt) = if bits < 32 {
            let i32_type = context.i32_type();

            let extended = builder
                .build_int_s_extend(value, i32_type, "vararg_sext")
                .map_err(|err| Self::map_err(err, span))?;

            (extended, "%d")
        } else if bits == 32 {
            (value, "%d")
        } else {
            (value, "%lld")
        };

        Self::int_to_str_via_snprintf(builder, libc, promoted, fmt, span)
    }

    fn unsigned_int_to_str<'ctx>(
        builder: &Builder<'ctx>,
        libc: &LibcFunctions<'ctx>,
        value: IntValue<'ctx>,
        bits: u32,
        span: Span,
    ) -> Result<LlvmValue<'ctx>, Box<dyn IError>> {
        let context = Self::context(builder);

        let (promoted, fmt) = if bits < 32 {
            let i32_type = context.i32_type();

            let extended = builder
                .build_int_z_extend(value, i32_type, "vararg_zext")
                .map_err(|err| Self::map_err(err, span))?;

            (extended, "%u")
        } else if bits == 32 {
            (value, "%u")
        } else {
            (value, "%llu")
        };

        Self::int_to_str_via_snprintf(builder, libc, promoted, fmt, span)
    }

    fn int_to_str_via_snprintf<'ctx>(
        builder: &Builder<'ctx>,
        libc: &LibcFunctions<'ctx>,
        value: IntValue<'ctx>,
        fmt: &str,
        span: Span,
    ) -> Result<LlvmValue<'ctx>, Box<dyn IError>> {
        let context = Self::context(builder);
        let i64_type = context.i64_type();

        // 21 bytes comfortably fits any 64-bit integer, including sign and NUL.
        let buffer_size = i64_type.const_int(21, false);

        let buffer = builder
            .build_call(libc.malloc_fn, &[buffer_size.into()], "int_to_str_buf")
            .map_err(|err| Self::map_err(err, span))?
            .try_as_basic_value()
            .basic()
            .expect("malloc should return a value")
            .into_pointer_value();

        let format = builder.build_global_string_ptr(fmt, "int_fmt").map_err(|err| Self::map_err(err, span))?;

        builder
            .build_call(
                libc.snprintf_fn,
                &[buffer.into(), buffer_size.into(), format.as_pointer_value().into(), value.into()],
                "snprintf_call",
            )
            .map_err(|err| Self::map_err(err, span))?;

        Self::wrap_str_header(builder, libc, buffer, span).map(LlvmValue::Str)
    }

    fn float_to_str<'ctx>(
        builder: &Builder<'ctx>,
        libc: &LibcFunctions<'ctx>,
        value: FloatValue<'ctx>,
        span: Span,
    ) -> Result<LlvmValue<'ctx>, Box<dyn IError>> {
        let context = Self::context(builder);
        let i64_type = context.i64_type();

        // 64 bytes comfortably fits a %g-formatted f64.
        let buffer_size = i64_type.const_int(64, false);

        let buffer = builder
            .build_call(libc.malloc_fn, &[buffer_size.into()], "f64_to_str_buf")
            .map_err(|err| Self::map_err(err, span))?
            .try_as_basic_value()
            .basic()
            .expect("malloc should return a value")
            .into_pointer_value();

        let format = builder.build_global_string_ptr("%g", "f64_fmt").map_err(|err| Self::map_err(err, span))?;

        builder
            .build_call(
                libc.snprintf_fn,
                &[buffer.into(), buffer_size.into(), format.as_pointer_value().into(), value.into()],
                "snprintf_call",
            )
            .map_err(|err| Self::map_err(err, span))?;

        Self::wrap_str_header(builder, libc, buffer, span).map(LlvmValue::Str)
    }
}
