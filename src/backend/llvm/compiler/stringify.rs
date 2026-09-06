use inkwell::values::{IntValue, PointerValue};
use inkwell::{AddressSpace, IntPredicate};

use super::Compiler;
use crate::{
    backend::llvm::llvm_alu::llvm_value::{LlvmValue, VEC_DATA, VEC_LENGTH},
    common::{
        errors::{CompilerError, ErrorSeverity, IError},
        span::Span,
        types::Type,
    },
};

impl<'a, 'ctx> Compiler<'a, 'ctx> {
    pub(in crate::backend::llvm::compiler) fn append_cstring_tracked(
        &mut self,
        buffer_ptr: PointerValue<'ctx>,
        current_length: IntValue<'ctx>,
        addition: PointerValue<'ctx>,
        span: Span,
    ) -> Result<IntValue<'ctx>, Box<dyn IError>> {
        let err = Self::builder_err(span);
        let ptr_type = self.context.ptr_type(AddressSpace::default());
        let i64_type = self.context.i64_type();

        let current = self
            .builder
            .build_load(ptr_type, buffer_ptr, "buf.cur")
            .map_err(&err)?
            .into_pointer_value();

        let len_add = self
            .builder
            .build_call(self.libc.strlen_fn, &[addition.into()], "len.add")
            .map_err(&err)?
            .try_as_basic_value()
            .basic()
            .unwrap()
            .into_int_value();

        let new_length = self.builder.build_int_add(current_length, len_add, "len.new").map_err(&err)?;
        let new_length_plus_nul = self
            .builder
            .build_int_add(new_length, i64_type.const_int(1, false), "len.new.nul")
            .map_err(&err)?;

        let new_buf = self
            .builder
            .build_call(self.libc.realloc_fn, &[current.into(), new_length_plus_nul.into()], "buf.realloc")
            .map_err(&err)?
            .try_as_basic_value()
            .basic()
            .unwrap()
            .into_pointer_value();

        let end_ptr = unsafe {
            self.builder
                .build_gep(self.context.i8_type(), new_buf, &[current_length], "buf.end")
                .map_err(&err)?
        };

        let len_add_plus_nul = self
            .builder
            .build_int_add(len_add, i64_type.const_int(1, false), "len.add.nul")
            .map_err(&err)?;

        self.builder
            .build_call(
                self.libc.memcpy_fn,
                &[end_ptr.into(), addition.into(), len_add_plus_nul.into()],
                "buf.memcpy",
            )
            .map_err(&err)?;

        self.builder.build_store(buffer_ptr, new_buf).map_err(&err)?;
        Ok(new_length)
    }

    /// Formats a scalar value into a raw, un-headered C string buffer.
    /// Caller is responsible for freeing the returned pointer.
    pub fn format_scalar_to_cstring(&mut self, value: LlvmValue<'ctx>, elem_type: &Type, span: Span) -> Result<PointerValue<'ctx>, Box<dyn IError>> {
        let err = Self::builder_err(span);
        let i64_type = self.context.i64_type();

        match (elem_type, &value) {
            (Type::I8, LlvmValue::I8(v)) => {
                let buf_size = i64_type.const_int(24, false);
                let buf = self
                    .builder
                    .build_array_malloc(self.context.i8_type(), buf_size, "num.buf")
                    .map_err(&err)?;
                let promoted = self.builder.build_int_s_extend(*v, self.context.i32_type(), "i8.promote").map_err(&err)?;
                let fmt = self.builder.build_global_string_ptr("%d", "fmt.i8").map_err(&err)?.as_pointer_value();
                self.builder
                    .build_call(
                        self.libc.snprintf_fn,
                        &[buf.into(), buf_size.into(), fmt.into(), promoted.into()],
                        "snprintf.i8",
                    )
                    .map_err(&err)?;
                Ok(buf)
            }
            (Type::I16, LlvmValue::I16(v)) => {
                let buf_size = i64_type.const_int(24, false);
                let buf = self
                    .builder
                    .build_array_malloc(self.context.i8_type(), buf_size, "num.buf")
                    .map_err(&err)?;
                let promoted = self
                    .builder
                    .build_int_s_extend(*v, self.context.i32_type(), "i16.promote")
                    .map_err(&err)?;
                let fmt = self.builder.build_global_string_ptr("%d", "fmt.i16").map_err(&err)?.as_pointer_value();
                self.builder
                    .build_call(
                        self.libc.snprintf_fn,
                        &[buf.into(), buf_size.into(), fmt.into(), promoted.into()],
                        "snprintf.i16",
                    )
                    .map_err(&err)?;
                Ok(buf)
            }
            (Type::I32, LlvmValue::I32(v)) => {
                let buf_size = i64_type.const_int(24, false);
                let buf = self
                    .builder
                    .build_array_malloc(self.context.i8_type(), buf_size, "num.buf")
                    .map_err(&err)?;
                let fmt = self.builder.build_global_string_ptr("%d", "fmt.i32").map_err(&err)?.as_pointer_value();
                self.builder
                    .build_call(
                        self.libc.snprintf_fn,
                        &[buf.into(), buf_size.into(), fmt.into(), (*v).into()],
                        "snprintf.i32",
                    )
                    .map_err(&err)?;
                Ok(buf)
            }
            (Type::I64, LlvmValue::I64(v)) => {
                let buf_size = i64_type.const_int(24, false);
                let buf = self
                    .builder
                    .build_array_malloc(self.context.i8_type(), buf_size, "num.buf")
                    .map_err(&err)?;
                let fmt = self.builder.build_global_string_ptr("%lld", "fmt.i64").map_err(&err)?.as_pointer_value();
                self.builder
                    .build_call(
                        self.libc.snprintf_fn,
                        &[buf.into(), buf_size.into(), fmt.into(), (*v).into()],
                        "snprintf.i64",
                    )
                    .map_err(&err)?;
                Ok(buf)
            }
            (Type::U8, LlvmValue::U8(v)) => {
                let buf_size = i64_type.const_int(24, false);
                let buf = self
                    .builder
                    .build_array_malloc(self.context.i8_type(), buf_size, "num.buf")
                    .map_err(&err)?;
                let promoted = self.builder.build_int_z_extend(*v, self.context.i32_type(), "u8.promote").map_err(&err)?;
                let fmt = self.builder.build_global_string_ptr("%u", "fmt.u8").map_err(&err)?.as_pointer_value();
                self.builder
                    .build_call(
                        self.libc.snprintf_fn,
                        &[buf.into(), buf_size.into(), fmt.into(), promoted.into()],
                        "snprintf.u8",
                    )
                    .map_err(&err)?;
                Ok(buf)
            }
            (Type::U16, LlvmValue::U16(v)) => {
                let buf_size = i64_type.const_int(24, false);
                let buf = self
                    .builder
                    .build_array_malloc(self.context.i8_type(), buf_size, "num.buf")
                    .map_err(&err)?;
                let promoted = self
                    .builder
                    .build_int_z_extend(*v, self.context.i32_type(), "u16.promote")
                    .map_err(&err)?;
                let fmt = self.builder.build_global_string_ptr("%u", "fmt.u16").map_err(&err)?.as_pointer_value();
                self.builder
                    .build_call(
                        self.libc.snprintf_fn,
                        &[buf.into(), buf_size.into(), fmt.into(), promoted.into()],
                        "snprintf.u16",
                    )
                    .map_err(&err)?;
                Ok(buf)
            }
            (Type::U32, LlvmValue::U32(v)) => {
                let buf_size = i64_type.const_int(24, false);
                let buf = self
                    .builder
                    .build_array_malloc(self.context.i8_type(), buf_size, "num.buf")
                    .map_err(&err)?;
                let fmt = self.builder.build_global_string_ptr("%u", "fmt.u32").map_err(&err)?.as_pointer_value();
                self.builder
                    .build_call(
                        self.libc.snprintf_fn,
                        &[buf.into(), buf_size.into(), fmt.into(), (*v).into()],
                        "snprintf.u32",
                    )
                    .map_err(&err)?;
                Ok(buf)
            }
            (Type::U64, LlvmValue::U64(v)) => {
                let buf_size = i64_type.const_int(24, false);
                let buf = self
                    .builder
                    .build_array_malloc(self.context.i8_type(), buf_size, "num.buf")
                    .map_err(&err)?;
                let fmt = self.builder.build_global_string_ptr("%llu", "fmt.u64").map_err(&err)?.as_pointer_value();
                self.builder
                    .build_call(
                        self.libc.snprintf_fn,
                        &[buf.into(), buf_size.into(), fmt.into(), (*v).into()],
                        "snprintf.u64",
                    )
                    .map_err(&err)?;
                Ok(buf)
            }
            (Type::F64, LlvmValue::F64(v)) => {
                let buf_size = i64_type.const_int(64, false);
                let buf = self
                    .builder
                    .build_array_malloc(self.context.i8_type(), buf_size, "num.buf")
                    .map_err(&err)?;
                let fmt = self.builder.build_global_string_ptr("%g", "fmt.f64").map_err(&err)?.as_pointer_value();
                self.builder
                    .build_call(
                        self.libc.snprintf_fn,
                        &[buf.into(), buf_size.into(), fmt.into(), (*v).into()],
                        "snprintf.f64",
                    )
                    .map_err(&err)?;
                Ok(buf)
            }
            (Type::Char, LlvmValue::Char(v)) => {
                let buf_size = i64_type.const_int(2, false);
                let buf = self
                    .builder
                    .build_array_malloc(self.context.i8_type(), buf_size, "char.buf")
                    .map_err(&err)?;
                self.builder.build_store(buf, *v).map_err(&err)?;
                let nul_ptr = unsafe {
                    self.builder
                        .build_gep(self.context.i8_type(), buf, &[i64_type.const_int(1, false)], "char.nul.ptr")
                        .map_err(&err)?
                };
                self.builder.build_store(nul_ptr, self.context.i8_type().const_zero()).map_err(&err)?;
                Ok(buf)
            }
            (Type::Bool, LlvmValue::Bool(v)) => {
                let true_str = self
                    .builder
                    .build_global_string_ptr("true", "bool.true")
                    .map_err(&err)?
                    .as_pointer_value();
                let false_str = self
                    .builder
                    .build_global_string_ptr("false", "bool.false")
                    .map_err(&err)?
                    .as_pointer_value();
                let picked = self
                    .builder
                    .build_select(*v, true_str, false_str, "bool.str")
                    .map_err(&err)?
                    .into_pointer_value();
                let len = self
                    .builder
                    .build_call(self.libc.strlen_fn, &[picked.into()], "bool.len")
                    .map_err(&err)?
                    .try_as_basic_value()
                    .basic()
                    .unwrap()
                    .into_int_value();
                let len_nul = self
                    .builder
                    .build_int_add(len, i64_type.const_int(1, false), "bool.len.nul")
                    .map_err(&err)?;
                let dup = self
                    .builder
                    .build_call(self.libc.malloc_fn, &[len_nul.into()], "bool.dup")
                    .map_err(&err)?
                    .try_as_basic_value()
                    .basic()
                    .unwrap()
                    .into_pointer_value();
                self.builder
                    .build_call(self.libc.strcpy_fn, &[dup.into(), picked.into()], "bool.strcpy")
                    .map_err(&err)?;
                Ok(dup)
            }
            (Type::Str, LlvmValue::Str(header_ptr)) => {
                let data = self.str_data_ptr(*header_ptr, span)?;
                let quote = self.builder.build_global_string_ptr("\"", "quote").map_err(&err)?.as_pointer_value();
                let len_v = self
                    .builder
                    .build_call(self.libc.strlen_fn, &[data.into()], "str.len")
                    .map_err(&err)?
                    .try_as_basic_value()
                    .basic()
                    .unwrap()
                    .into_int_value();
                let total = self
                    .builder
                    .build_int_add(len_v, i64_type.const_int(3, false), "str.total")
                    .map_err(&err)?;
                let buf = self
                    .builder
                    .build_call(self.libc.malloc_fn, &[total.into()], "str.buf")
                    .map_err(&err)?
                    .try_as_basic_value()
                    .basic()
                    .unwrap()
                    .into_pointer_value();
                let empty = self.builder.build_global_string_ptr("", "empty").map_err(&err)?.as_pointer_value();
                self.builder
                    .build_call(self.libc.strcpy_fn, &[buf.into(), empty.into()], "str.init")
                    .map_err(&err)?;
                self.builder
                    .build_call(self.libc.strcat_fn, &[buf.into(), quote.into()], "str.cat1")
                    .map_err(&err)?;
                self.builder
                    .build_call(self.libc.strcat_fn, &[buf.into(), data.into()], "str.cat2")
                    .map_err(&err)?;
                self.builder
                    .build_call(self.libc.strcat_fn, &[buf.into(), quote.into()], "str.cat3")
                    .map_err(&err)?;
                Ok(buf)
            }
            (other, _) => Err(Box::new(CompilerError::at(
                ErrorSeverity::HIGH,
                format!("Compiling vector_stringify for element type '{}' is not yet supported.", other),
                span,
            ))),
        }
    }

    /// Builds the `[...]` textual representation of a vector, returning it
    /// as a properly-headered, refcounted `Str` (refcount 1).
    pub fn build_vector_to_string(
        &mut self,
        vector_ptr: PointerValue<'ctx>,
        inner_type: &Type,
        span: Span,
    ) -> Result<PointerValue<'ctx>, Box<dyn IError>> {
        let err = Self::builder_err(span);
        let function = self.current_function();
        let struct_type = LlvmValue::vector_struct_type(self.context);
        let ptr_type = self.context.ptr_type(AddressSpace::default());
        let i64_type = self.context.i64_type();

        let data_field = self
            .builder
            .build_struct_gep(struct_type, vector_ptr, VEC_DATA, "vec.data")
            .map_err(&err)?;
        let length_field = self
            .builder
            .build_struct_gep(struct_type, vector_ptr, VEC_LENGTH, "vec.length")
            .map_err(&err)?;

        let data = self
            .builder
            .build_load(ptr_type, data_field, "vec.data.val")
            .map_err(&err)?
            .into_pointer_value();
        let length = self
            .builder
            .build_load(i64_type, length_field, "vec.length.val")
            .map_err(&err)?
            .into_int_value();

        let open_bracket = self.builder.build_global_string_ptr("[", "open").map_err(&err)?.as_pointer_value();
        let result_init = self
            .builder
            .build_call(self.libc.malloc_fn, &[i64_type.const_int(2, false).into()], "result.init")
            .map_err(&err)?
            .try_as_basic_value()
            .basic()
            .unwrap()
            .into_pointer_value();
        self.builder
            .build_call(self.libc.strcpy_fn, &[result_init.into(), open_bracket.into()], "result.strcpy")
            .map_err(&err)?;

        let result_alloca = self.builder.build_alloca(ptr_type, "result").map_err(&err)?;
        self.builder.build_store(result_alloca, result_init).map_err(&err)?;

        let length_alloca = self.builder.build_alloca(i64_type, "stringify.len").map_err(&err)?;
        self.builder.build_store(length_alloca, i64_type.const_int(1, false)).map_err(&err)?;

        let index_alloca = self.builder.build_alloca(i64_type, "stringify.i").map_err(&err)?;
        self.builder.build_store(index_alloca, i64_type.const_int(0, false)).map_err(&err)?;

        let cond_block = self.context.append_basic_block(function, "stringify.cond");
        let body_block = self.context.append_basic_block(function, "stringify.body");
        let after_block = self.context.append_basic_block(function, "stringify.after");

        self.builder.build_unconditional_branch(cond_block).map_err(&err)?;

        // condition
        self.builder.position_at_end(cond_block);
        let idx = self.builder.build_load(i64_type, index_alloca, "i.val").map_err(&err)?.into_int_value();
        let cmp = self.builder.build_int_compare(IntPredicate::SLT, idx, length, "i.cmp").map_err(&err)?;
        self.builder.build_conditional_branch(cmp, body_block, after_block).map_err(&err)?;

        // body
        self.builder.position_at_end(body_block);

        let is_first = self
            .builder
            .build_int_compare(IntPredicate::EQ, idx, i64_type.const_int(0, false), "i.is_first")
            .map_err(&err)?;
        let empty_sep = self.builder.build_global_string_ptr("", "sep.empty").map_err(&err)?.as_pointer_value();
        let comma_sep = self.builder.build_global_string_ptr(", ", "sep.comma").map_err(&err)?.as_pointer_value();
        let sep = self
            .builder
            .build_select(is_first, empty_sep, comma_sep, "sep")
            .map_err(&err)?
            .into_pointer_value();

        let len_before_sep = self
            .builder
            .build_load(i64_type, length_alloca, "len.before_sep")
            .map_err(&err)?
            .into_int_value();
        let len_after_sep = self.append_cstring_tracked(result_alloca, len_before_sep, sep, span)?;
        self.builder.build_store(length_alloca, len_after_sep).map_err(&err)?;

        let element_llvm_type = LlvmValue::type_to_basic_type_enum(inner_type, self.context).ok_or_else(|| {
            Box::new(CompilerError::at(
                ErrorSeverity::HIGH,
                format!("Compiling vectors of type '{}' is not yet supported. 3", inner_type),
                span,
            )) as Box<dyn IError>
        })?;

        let elem_ptr = unsafe { self.builder.build_gep(element_llvm_type, data, &[idx], "elem.ptr").map_err(&err)? };
        let elem_raw = self.builder.build_load(element_llvm_type, elem_ptr, "elem.val").map_err(&err)?;
        let elem_value = LlvmValue::from_basic_value_enum(elem_raw, inner_type);

        let elem_str = match &elem_value {
            LlvmValue::Vector(nested_ptr, nested_inner) => self.build_vector_to_string(*nested_ptr, nested_inner, span)?,
            _ => self.format_scalar_to_cstring(elem_value.clone(), inner_type, span)?,
        };

        // Nested vector returns a proper StrHeader → extract raw C buffer.
        // Scalar already returns a raw buffer.
        let elem_cstr = if matches!(inner_type, Type::Vector(_)) {
            self.str_data_ptr(elem_str, span)?
        } else {
            elem_str
        };

        let len_before_elem = self
            .builder
            .build_load(i64_type, length_alloca, "len.before_elem")
            .map_err(&err)?
            .into_int_value();
        let len_after_elem = self.append_cstring_tracked(result_alloca, len_before_elem, elem_cstr, span)?;
        self.builder.build_store(length_alloca, len_after_elem).map_err(&err)?;

        // === FIX: zwolnij tymczasowy bufor ===
        if matches!(inner_type, Type::Vector(_)) {
            // Nested vector → zwolnij StrHeader
            self.release_value(&LlvmValue::Str(elem_str), span)?;
        } else {
            // Skalar → zwolnij surowy malloc z format_scalar_to_cstring
            self.builder
                .build_call(self.libc.free_fn, &[elem_str.into()], "elem.str.free")
                .map_err(&err)?;
        }
        // ====================================

        let next_idx = self.builder.build_int_add(idx, i64_type.const_int(1, false), "i.next").map_err(&err)?;
        self.builder.build_store(index_alloca, next_idx).map_err(&err)?;
        self.builder.build_unconditional_branch(cond_block).map_err(&err)?;

        // after
        self.builder.position_at_end(after_block);

        let close_bracket = self.builder.build_global_string_ptr("]", "close").map_err(&err)?.as_pointer_value();
        let len_before_close = self
            .builder
            .build_load(i64_type, length_alloca, "len.before_close")
            .map_err(&err)?
            .into_int_value();
        let _len_after_close = self.append_cstring_tracked(result_alloca, len_before_close, close_bracket, span)?;

        let final_result = self
            .builder
            .build_load(ptr_type, result_alloca, "result.final")
            .map_err(&err)?
            .into_pointer_value();

        self.build_str_header(final_result, span)
    }
}
