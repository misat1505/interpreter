use std::{
    collections::HashMap,
    fmt::{Debug, Display},
    rc::Rc,
};

use crate::{
    backend::std_functions::std_functions::StdFunction,
    common::{span::Span, types::Type},
};

#[derive(Debug, Clone, PartialEq)]
pub struct Node<T> {
    pub value: T,
    pub span: Span,
}

type BNode<T> = Box<Node<T>>;

#[derive(Debug, Clone, PartialEq)]
pub struct StructLiteralField {
    pub identifier: Node<String>,
    pub value: Node<Expression>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct StructLiteral {
    pub identifier: Node<String>,
    pub fields: Vec<Node<StructLiteralField>>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Expression {
    // Boolean operations
    Alternative(BNode<Expression>, BNode<Expression>),
    Concatenation(BNode<Expression>, BNode<Expression>),
    // Relations
    Greater(BNode<Expression>, BNode<Expression>),
    GreaterEqual(BNode<Expression>, BNode<Expression>),
    Less(BNode<Expression>, BNode<Expression>),
    LessEqual(BNode<Expression>, BNode<Expression>),
    Equal(BNode<Expression>, BNode<Expression>),
    NotEqual(BNode<Expression>, BNode<Expression>),
    // Arithmetic operations
    Addition(BNode<Expression>, BNode<Expression>),
    Subtraction(BNode<Expression>, BNode<Expression>),
    Multiplication(BNode<Expression>, BNode<Expression>),
    Division(BNode<Expression>, BNode<Expression>),
    Modulo(BNode<Expression>, BNode<Expression>),
    // Unary operations
    BooleanNegation(BNode<Expression>),
    ArithmeticNegation(BNode<Expression>),
    // Casting
    Casting {
        value: BNode<Expression>,
        to_type: Node<Type>,
    },
    // Values
    Literal(Literal),
    Vector(Vec<BNode<Expression>>),
    Index {
        collection: BNode<Expression>,
        index: BNode<Expression>,
    },
    FieldAccess {
        instance: BNode<Expression>,
        field: Node<String>,
    },
    Variable(String),
    FunctionCall {
        identifier: Node<String>,
        arguments: Vec<BNode<Argument>>,
    },
    StructLiteral(Node<StructLiteral>),
}

#[derive(Debug, Clone, PartialEq)]
pub enum Literal {
    True,
    False,
    String(String),
    Char(char),
    I64(i64),
    F64(f64),
}

#[derive(Debug, Clone, PartialEq)]
pub enum PassedBy {
    Reference,
    Value,
}

impl Display for PassedBy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Value => write!(f, "Value"),
            Self::Reference => write!(f, "Reference"),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Argument {
    pub value: Node<Expression>,
    pub passed_by: PassedBy,
}

#[derive(Debug, Clone, PartialEq)]
pub enum VariableDeclarationKind {
    LET { var_type: Option<Node<Type>>, value: Node<Expression> },
    TYPE { var_type: Node<Type>, value: Option<Node<Expression>> },
}

#[derive(Debug, Clone, PartialEq)]
pub enum Accessor {
    Index(Node<Expression>),
    Field(Node<String>),
}

#[derive(Debug, Clone, PartialEq)]
pub enum Statement {
    FunctionCall {
        identifier: Node<String>,
        arguments: Vec<BNode<Argument>>,
    },
    Declaration {
        identifier: Node<String>,
        kind: VariableDeclarationKind,
    },
    Assignment {
        identifier: Node<String>,
        accessors: Vec<Node<Accessor>>,
        value: Node<Expression>,
    },
    Conditional {
        condition: Node<Expression>,
        if_block: Node<Block>,
        else_block: Option<Node<Block>>,
    },
    WhileLoop {
        condition: Node<Expression>,
        block: Node<Block>,
    },
    ForLoop {
        declaration: Option<Box<Node<Statement>>>,
        condition: Node<Expression>,
        assignment: Option<Box<Node<Statement>>>,
        block: Node<Block>,
    },
    Switch {
        expressions: Vec<Node<SwitchExpression>>,
        cases: Vec<Node<SwitchCase>>,
    },
    Return(Option<Node<Expression>>),
    Break,
    Continue,
    Import {
        path: Node<String>,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub struct Parameter {
    pub passed_by: PassedBy,
    pub parameter_type: Node<Type>,
    pub identifier: Node<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SwitchExpression {
    pub expression: Node<Expression>,
    pub alias: Option<Node<String>>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SwitchCase {
    pub condition: Node<Expression>,
    pub block: Node<Block>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Block(pub Vec<Node<Statement>>);

#[derive(Debug, Clone, PartialEq)]
pub struct FunctionDeclaration {
    pub identifier: Node<String>,
    pub parameters: Vec<Node<Parameter>>,
    pub return_type: Node<Type>,
    pub block: Node<Block>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ExternFunctionDeclaration {
    pub identifier: Node<String>,
    pub alias: Option<Node<String>>,
    pub parameters: Vec<Node<Parameter>>,
    pub return_type: Node<Type>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct StructMember {
    pub identifier: Node<String>,
    pub member_type: Node<Type>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct StructDeclaration {
    pub identifier: Node<String>,
    pub members: Vec<Node<StructMember>>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum DeclaredType {
    Struct(StructDeclaration),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Program {
    pub statements: Vec<Node<Statement>>,
    pub functions: HashMap<String, Rc<Node<FunctionDeclaration>>>,
    pub std_functions: HashMap<String, StdFunction>,
    pub extern_functions: HashMap<String, Rc<Node<ExternFunctionDeclaration>>>,
    pub declared_types: HashMap<String, Rc<Node<DeclaredType>>>,
    pub types: HashMap<String, Type>,
}
