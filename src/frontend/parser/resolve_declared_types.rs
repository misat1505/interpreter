use std::{collections::HashMap, rc::Rc};

use crate::{
    common::{errors::IError, types::Type},
    frontend::ast::{DeclaredType, Node},
};

pub fn resolve_declared_types(declared_types: &HashMap<String, Rc<Node<DeclaredType>>>) -> Result<HashMap<String, Type>, Box<dyn IError>> {
    let mut types = HashMap::new();

    for (name, declared_type) in declared_types {
        let resolved_type = match &declared_type.value {
            DeclaredType::Struct(struct_declaration) => {
                let mut fields = HashMap::new();

                for member in &struct_declaration.members {
                    fields.insert(member.value.identifier.value.clone(), member.value.member_type.value.clone());
                }

                Ok(Type::Struct {
                    identifier: struct_declaration.identifier.value.clone(),
                    fields,
                })
            }
            DeclaredType::Enum(enum_declaration) => {
                let mut fields = HashMap::new();

                for member in &enum_declaration.members {
                    let field_type = match member.value.member_type {
                        Some(ref t) => Some(t.value.clone()),
                        None => None,
                    };
                    fields.insert(member.value.identifier.value.clone(), field_type.clone());
                }

                Ok(Type::Enum {
                    identifier: enum_declaration.identifier.value.clone(),
                    fields,
                })
            }
        }?;
        types.insert(name.clone(), resolved_type);
    }

    Ok(types)
}
