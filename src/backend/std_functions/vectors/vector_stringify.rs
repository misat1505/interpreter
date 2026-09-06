use std::{cell::RefCell, rc::Rc, vec};

use crate::{
    backend::{
        interpreter::Value,
        llvm::{compiler::Compiler, LlvmValue},
        std_functions::std_functions::{build_usage_error, LlvmCompileFn, StdFunction},
    },
    common::{
        errors::{CompilerError, ErrorSeverity, IError, StdFunctionError},
        span::Span,
        types::Type,
        visitor::Visitor,
    },
    frontend::ast::PassedBy,
};

fn stringify_value(value: &Value) -> String {
    match value {
        Value::I8(v) => v.to_string(),
        Value::I16(v) => v.to_string(),
        Value::I32(v) => v.to_string(),
        Value::I64(v) => v.to_string(),

        Value::U8(v) => v.to_string(),
        Value::U16(v) => v.to_string(),
        Value::U32(v) => v.to_string(),
        Value::U64(v) => v.to_string(),

        Value::F64(v) => v.to_string(),
        Value::String(v) => format!("\"{}\"", v),
        Value::Char(v) => format!("\'{}\'", v),
        Value::Bool(v) => v.to_string(),

        Value::Vector { values, .. } => {
            let values = values.borrow().iter().map(|v| stringify_value(&v.borrow())).collect::<Vec<String>>();

            format!("[{}]", values.join(", "))
        }
        Value::Struct { identifier, fields, .. } => {
            let fields = fields.borrow();

            let mut field_names: Vec<&String> = fields.keys().collect();
            field_names.sort();

            let fields_str = field_names
                .into_iter()
                .map(|name| format!("{}: {}", name, stringify_value(&fields.get(name).unwrap().borrow())))
                .collect::<Vec<String>>()
                .join(", ");

            if fields_str.is_empty() {
                format!("{} {{}}", identifier)
            } else {
                format!("{} {{{}}}", identifier, fields_str)
            }
        }
    }
}

pub fn vector_stringify() -> StdFunction {
    let params = vec![Type::Vector(Box::new(Type::Void))];

    let execute = |params: &Vec<Rc<RefCell<Value>>>, span: Span| -> Result<Option<Value>, StdFunctionError> {
        let fn_name = "vector_stringify";
        let expected_types = vec![Type::Vector(Box::new(Type::Void))];
        let mut actual_types: Vec<Type> = vec![];

        if let Some(vector) = params.first() {
            actual_types.push(vector.borrow().to_type());

            let vector = vector.borrow();

            match &*vector {
                Value::Vector { .. } => Ok(Some(Value::String(stringify_value(&vector)))),
                _ => Err(build_usage_error(fn_name, expected_types, actual_types, span)),
            }
        } else {
            Err(build_usage_error(fn_name, expected_types, actual_types, span))
        }
    };

    let type_check: fn(&[Type]) -> Result<Type, String> = |arg_types: &[Type]| match arg_types {
        [Type::Vector(_)] => Ok(Type::Str),
        [other] => Err(format!("vector_stringify expected a vector, but got '{}'.", other)),
        _ => Err(String::from("vector_stringify expects exactly 1 argument.")),
    };

    let compile: LlvmCompileFn = |compiler, arguments, span| {
        let vector_arg = arguments.first().ok_or_else(|| {
            Box::new(CompilerError::at(
                ErrorSeverity::HIGH,
                String::from("'vector_stringify' expects exactly one argument."),
                span,
            )) as Box<dyn IError>
        })?;

        compiler.visit_expression(&vector_arg.value.value)?;
        let vector_value = compiler.read_last_value()?;

        let (vector_ptr, inner_type) = match &vector_value {
            LlvmValue::Vector(ptr, inner) => (*ptr, (**inner).clone()),
            other => {
                return Err(Box::new(CompilerError::at(
                    ErrorSeverity::HIGH,
                    format!("'vector_stringify' expects a vector, got '{}'.", other.to_type()),
                    span,
                )))
            }
        };

        let result = compiler.build_vector_to_string(vector_ptr, &inner_type, span)?;

        if Compiler::expr_needs_release(&vector_arg.value.value.value) {
            compiler.release_value(&vector_value, span)?;
        }

        compiler.set_last_value(LlvmValue::Str(result));

        Ok(())
    };

    StdFunction {
        params,
        passed_by: vec![PassedBy::Value],
        return_type: Type::Str,
        type_check: Some(type_check),
        compile,
        execute,
    }
}
