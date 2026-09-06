use std::{
    cell::RefCell,
    collections::HashMap,
    fmt::{self, Display},
    rc::Rc,
    vec,
};

use crate::common::{
    errors::{ComputationError, ErrorSeverity},
    span::Span,
    types::Type,
};

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    I8(i8),
    I16(i16),
    I32(i32),
    I64(i64),

    U8(u8),
    U16(u16),
    U32(u32),
    U64(u64),

    F64(f64),
    String(String),
    Char(char),
    Bool(bool),

    Vector {
        kind: Box<Type>,
        values: Rc<RefCell<Vec<Rc<RefCell<Value>>>>>,
    },

    Struct {
        identifier: String,
        fields_types: Rc<HashMap<String, Type>>,
        fields: Rc<RefCell<HashMap<String, Rc<RefCell<Value>>>>>,
    },
}

impl Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Value::I8(value) => write!(f, "{value}"),
            Value::I16(value) => write!(f, "{value}"),
            Value::I32(value) => write!(f, "{value}"),
            Value::I64(value) => write!(f, "{value}"),
            Value::U8(value) => write!(f, "{value}"),
            Value::U16(value) => write!(f, "{value}"),
            Value::U32(value) => write!(f, "{value}"),
            Value::U64(value) => write!(f, "{value}"),
            Value::F64(value) => write!(f, "{value}"),
            Value::String(value) => write!(f, "{value}"),
            Value::Char(value) => write!(f, "{value}"),
            Value::Bool(value) => write!(f, "{value}"),
            Value::Vector { values, .. } => {
                let values = values.borrow();
                write!(f, "[")?;
                for (index, value) in values.iter().enumerate() {
                    if index > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", value.borrow())?;
                }
                write!(f, "]")
            }
            Value::Struct { identifier, fields, .. } => {
                let fields = fields.borrow();
                write!(f, "{identifier} {{")?;
                let mut entries: Vec<_> = fields.iter().collect();
                entries.sort_by_key(|(name, _)| *name);

                for (index, (name, value)) in entries.iter().enumerate() {
                    if index > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{name}: {}", value.borrow())?;
                }
                write!(f, "}}")
            }
        }
    }
}

impl Value {
    pub(in crate::backend) fn default_value(var_type: &Type, span: Span) -> Result<Value, ComputationError> {
        match var_type {
            Type::Bool => Ok(Value::Bool(false)),

            Type::I8 => Ok(Value::I8(0)),
            Type::I16 => Ok(Value::I16(0)),
            Type::I32 => Ok(Value::I32(0)),
            Type::I64 => Ok(Value::I64(0)),

            Type::U8 => Ok(Value::U8(0)),
            Type::U16 => Ok(Value::U16(0)),
            Type::U32 => Ok(Value::U32(0)),
            Type::U64 => Ok(Value::U64(0)),

            Type::F64 => Ok(Value::F64(0.0)),
            Type::Str => Ok(Value::String("".to_owned())),

            Type::Vector(inner) => Ok(Value::Vector {
                kind: Box::new(Type::Vector(inner.clone())),
                values: Rc::new(RefCell::new(vec![])),
            }),

            Type::Struct { identifier, fields } => {
                let mut default_fields = HashMap::new();

                for (field_name, field_type) in fields {
                    let default_field_value = Value::default_value(field_type, span)?;
                    default_fields.insert(field_name.clone(), Rc::new(RefCell::new(default_field_value)));
                }

                Ok(Value::Struct {
                    identifier: identifier.clone(),
                    fields_types: Rc::new(fields.clone()),
                    fields: Rc::new(RefCell::new(default_fields)),
                })
            }

            other => Err(ComputationError::new(
                ErrorSeverity::HIGH,
                format!("Cannot create default value for type '{}'.", other),
                span,
            )),
        }
    }

    pub(in crate::backend) fn to_type(&self) -> Type {
        match self {
            Value::Bool(_) => Type::Bool,

            Value::I8(_) => Type::I8,
            Value::I16(_) => Type::I16,
            Value::I32(_) => Type::I32,
            Value::I64(_) => Type::I64,

            Value::U8(_) => Type::U8,
            Value::U16(_) => Type::U16,
            Value::U32(_) => Type::U32,
            Value::U64(_) => Type::U64,

            Value::F64(_) => Type::F64,
            Value::String(_) => Type::Str,
            Value::Char(_) => Type::Char,

            Value::Vector { kind, .. } => kind.as_ref().clone(),

            Value::Struct {
                identifier, fields_types, ..
            } => Type::Struct {
                identifier: identifier.clone(),
                fields: (**fields_types).clone(),
            },
        }
    }

    pub(in crate::backend) fn try_into_bool(&self, span: Span) -> Result<bool, ComputationError> {
        match self {
            Value::Bool(bool) => Ok(*bool),

            _ => Err(ComputationError::new(
                ErrorSeverity::HIGH,
                String::from("Given value is not a boolean."),
                span,
            )),
        }
    }
}
