use std::collections::HashMap;

use crate::frontend::lexer::token::TokenKind::{ self };

#[derive(Debug, Clone, PartialEq)]
pub enum Visibility {
    Public,
    Private,
    Static,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Editability {
    Editable,
    NotEditable,
}
#[derive(Debug, Clone, PartialEq)]
pub enum Size {
    S8,
    S16,
    S32,
    S64,
    S128,
}
impl Size {
    pub fn as_str(&self) -> String {
        match self {
            Self::S8 => "8".to_string(),
            Self::S16 => "16".to_string(),
            Self::S32 => "32".to_string(),
            Self::S64 => "64".to_string(),
            Self::S128 => "128".to_string(),
            // _ => "unreachable".to_string(),
        }
    }
}
#[derive(Debug, Clone, PartialEq)]
pub enum BaseType {
    Int(Size),
    UInt(Size),
    USize,
    ISize,
    Float(Size),
    Char,
    Str,
    Bool,
    Void,
    //It will contain a Generic BaseType
    Name(Box<BaseType>), // name<T,T2,T3> //T , T2 , T3 are the expected types for the name but after
    Modify(Box<BaseType>), // modify<T> we for now only support modify<T> for name but we may add the pointer also
    Copy(Box<BaseType>), // copy<T> we for now only support copy<T> for name but we may add all the other types also
    Pointer(Box<BaseType>),
    Type(Box<BaseType>),
    Array {
        base_type: Box<BaseType>,
        size: Box<Option<Expr>>,
    },
    Struct {
        name: String,
        fields: Box<HashMap<String, BaseType>>,
        methods: Box<HashMap<String, FnType>>,
        generics: Vec<BaseType>,
    },
    Class {
        name: String,
        fields: Box<HashMap<String, BaseType>>,
        methods: Box<HashMap<String, FnType>>,
        constructor: Option<Vec<ConstructorType>>,
        generics: Vec<BaseType>,
    },
    Enum {
        name: String,
        variants: Vec<EnumVariant>,
        methods: Box<HashMap<String, FnType>>,
        generics: Vec<BaseType>,
    },
    Blueprint {
        name: String,
        fields: Box<HashMap<String, BaseType>>,
        methods: Box<HashMap<String, FnType>>,
        generics: Vec<BaseType>,
    },
    Block {
        name: String,
        fields: Box<HashMap<String, BaseType>>,
        methods: Box<HashMap<String, FnType>>,
    },
    Label {
        name: String,
        fields: Box<HashMap<String, BaseType>>,
    },
    Machine {
        name: String,
        fields: Box<HashMap<String, BaseType>>,
        methods: Box<HashMap<String, FnType>>,
        labels: Box<HashMap<String, BaseType>>,
    },

    Method {
        name: Option<String>,
        params: Vec<BaseType>,
        return_type: Box<BaseType>,
    },
    Fn {
        name: Option<String>,
        params: Vec<BaseType>,
        return_type: Box<BaseType>,
    },
    Micro {
        name: Option<String>,
        params: Vec<BaseType>,
        return_type: Box<BaseType>,
    },
    Macro {
        name: Option<String>,
        params: Vec<BaseType>,
        return_type: Box<BaseType>,
    },
    Lambda {
        name: Option<String>,
        params: Vec<BaseType>,
        return_type: Box<BaseType>,
    },
    Flag,
    Generic(Vec<BaseType>),
    GenericParam(String),

    Unknown,
    Error,
    New(String),
}
impl BaseType {
    pub fn get_name(&self) -> String {
        match self {
            BaseType::Blueprint { name, .. } => name.clone(),
            BaseType::Struct { name, .. } => name.clone(),
            BaseType::Class { name, .. } => name.clone(),
            BaseType::Enum { name, .. } => name.clone(),
            BaseType::Block { name, .. } => name.clone(),
            BaseType::Machine { name, .. } => name.clone(),
            BaseType::Label { name, .. } => name.clone(),
            _ => self.as_str(),
        }
    }

    pub fn as_str(&self) -> String {
        match self {
            BaseType::Int(s) => format!("int{}", s.as_str()),
            BaseType::UInt(s) => format!("uint{}", s.as_str()),
            BaseType::USize => "usize".to_string(),
            BaseType::ISize => "isize".to_string(),
            BaseType::Float(s) => format!("float{}", s.as_str()),
            BaseType::Char => "char".to_string(),
            BaseType::Str => "str".to_string(),
            BaseType::Bool => "bool".to_string(),
            BaseType::Flag => "flag".to_string(),
            BaseType::Block { name, .. } => format!("block<{}>", name),
            BaseType::Machine { name, .. } => format!("machine<{}>", name),
            BaseType::Label { name, .. } => format!("label<{}>", name),
            BaseType::Void => "void".to_string(),
            BaseType::Modify(t) => format!("modify<{}>", t.as_str()),
            BaseType::Copy(t) => format!("copy<{}>", t.as_str()),
            BaseType::Name(t) => format!("name<{}>", t.as_str()),
            BaseType::Pointer(t) => format!("pointer<{}>", t.as_str()),
            BaseType::Type(t) => format!("type<{}>", t.as_str()),
            BaseType::Array { base_type, size } => {
                if let Some(s) = size.as_ref() {
                    format!("array<{}[{}]>", base_type.as_str(), s.as_str())
                } else {
                    format!("array<{}>", base_type.as_str())
                }
            }
            BaseType::Struct { name, generics, .. } => {
                if generics.is_empty() {
                    format!("struct<{}>", name)
                } else {
                    let g_strs: Vec<String> = generics
                        .iter()
                        .map(|g| g.as_str())
                        .collect();
                    format!("struct<{}<{}> >", name, g_strs.join(", "))
                }
            }
            BaseType::Class { name, generics, .. } => {
                if generics.is_empty() {
                    format!("class<{}>", name)
                } else {
                    let g_strs: Vec<String> = generics
                        .iter()
                        .map(|g| g.as_str())
                        .collect();
                    format!("class<{}<{}> >", name, g_strs.join(", "))
                }
            }
            BaseType::Enum { name, generics, .. } => {
                if generics.is_empty() {
                    format!("enum<{}>", name)
                } else {
                    let g_strs: Vec<String> = generics
                        .iter()
                        .map(|g| g.as_str())
                        .collect();
                    format!("enum<{}<{}> >", name, g_strs.join(", "))
                }
            }
            BaseType::Blueprint { name, generics, .. } => {
                if generics.is_empty() {
                    format!("blueprint<{}>", name)
                } else {
                    let g_strs: Vec<String> = generics
                        .iter()
                        .map(|g| g.as_str())
                        .collect();
                    format!("blueprint<{}<{}> >", name, g_strs.join(", "))
                }
            }
            BaseType::Method { name, params, return_type } => {
                let p_strs: Vec<String> = params
                    .iter()
                    .map(|p| p.as_str())
                    .collect();
                if name.clone().is_none() {
                    format!("Method<({}), {}>", p_strs.join(", "), return_type.as_str())
                } else {
                    format!(
                        "Method::{}<({}), {}>",
                        name.as_ref().unwrap().as_str(),
                        p_strs.join(", "),
                        return_type.as_str()
                    )
                }
            }
            BaseType::Micro { name, params, return_type } => {
                let p_strs: Vec<String> = params
                    .iter()
                    .map(|p| p.as_str())
                    .collect();
                if name.clone().is_none() {
                    format!("Micro<({}), {}>", p_strs.join(", "), return_type.as_str())
                } else {
                    format!(
                        "Micro::{}<({}), {}>",
                        name.as_ref().unwrap().as_str(),
                        p_strs.join(", "),
                        return_type.as_str()
                    )
                }
            }
            BaseType::Macro { name, params, return_type } => {
                let p_strs: Vec<String> = params
                    .iter()
                    .map(|p| p.as_str())
                    .collect();
                if name.is_none() {
                    format!("Macro<({}), {}>", p_strs.join(", "), return_type.as_str())
                } else {
                    format!(
                        "Macro::{}<({}), {}>",
                        name.as_ref().unwrap().as_str(),
                        p_strs.join(", "),
                        return_type.as_str()
                    )
                }
            }
            BaseType::Lambda { name, params, return_type } => {
                let p_strs: Vec<String> = params
                    .iter()
                    .map(|p| p.as_str())
                    .collect();
                if name.clone().is_none() {
                    format!("Lambda<({}), {}>", p_strs.join(", "), return_type.as_str())
                } else {
                    format!(
                        "Lambda::{}<({}), {}>",
                        name.as_ref().unwrap().as_str(),
                        p_strs.join(", "),
                        return_type.as_str()
                    )
                }
            }
            BaseType::Fn { name, params, return_type } => {
                let p_strs: Vec<String> = params
                    .iter()
                    .map(|p| p.as_str())
                    .collect();
                if name.is_none() {
                    format!("Fn<({}), {}>", p_strs.join(", "), return_type.as_str())
                } else {
                    format!(
                        "Fn::{}<({}), {}>",
                        name.as_ref().unwrap().as_str(),
                        p_strs.join(", "),
                        return_type.as_str()
                    )
                }
            }
            BaseType::GenericParam(name) => name.clone(),
            BaseType::Generic(inner_vec) => {
                let strs: Vec<String> = inner_vec
                    .iter()
                    .map(|t| t.as_str())
                    .collect();
                strs.join(", ")
            }
            BaseType::Unknown => "unknown".to_string(),
            BaseType::Error => "error".to_string(),
            BaseType::New(t) => format!("new<{}>", t),
        }
    }
    pub fn from_str(s: &str) -> Self {
        match s {
            "int8" => BaseType::Int(Size::S8),
            "int16" => BaseType::Int(Size::S16),
            "int32" | "int" => BaseType::Int(Size::S32),
            "int64" => BaseType::Int(Size::S64),
            "int128" => BaseType::Int(Size::S128),
            "uint8" | "byte" => BaseType::UInt(Size::S8),
            "uint16" => BaseType::UInt(Size::S16),
            "uint32" | "uint" => BaseType::UInt(Size::S32),
            "uint64" => BaseType::UInt(Size::S64),
            "uint128" => BaseType::UInt(Size::S128),
            "usize" => BaseType::USize,
            "isize" => BaseType::ISize,
            "float32" | "float" => BaseType::Float(Size::S32),
            "float64" => BaseType::Float(Size::S64),
            "float128" => BaseType::Float(Size::S128),
            "char" => BaseType::Char,
            "str" => BaseType::Str,
            "bool" => BaseType::Bool,
            "void" => BaseType::Void,
            "name" => BaseType::Name(Box::new(BaseType::Generic(Vec::new()))), // name<T,T2,T3> //T , T2 , T3 are the expected types for the name but after
            "modify" => BaseType::Modify(Box::new(BaseType::Unknown)),
            "copy" => BaseType::Copy(Box::new(BaseType::Unknown)),
            "pointer" => BaseType::Pointer(Box::new(BaseType::Unknown)),
            "type" => BaseType::Type(Box::new(BaseType::Unknown)),
            "macro" => BaseType::Macro { name: None, params: vec![], return_type: Box::new(BaseType::Void) },
            "unknown" => BaseType::Unknown,
            "error" => BaseType::Error,
            _ => BaseType::Unknown,
        }
    }

    pub fn substitute_generics(
        &self,
        map: &std::collections::HashMap<String, BaseType>
    ) -> BaseType {
        match self {
            BaseType::GenericParam(name) => {
                if let Some(concrete) = map.get(name) {
                    concrete.clone()
                } else {
                    BaseType::GenericParam(name.clone())
                }
            }
            BaseType::Name(inner) => BaseType::Name(Box::new(inner.substitute_generics(map))),
            BaseType::Modify(inner) => BaseType::Modify(Box::new(inner.substitute_generics(map))),
            BaseType::Copy(inner) => BaseType::Copy(Box::new(inner.substitute_generics(map))),
            BaseType::Pointer(inner) => BaseType::Pointer(Box::new(inner.substitute_generics(map))),
            BaseType::Type(inner) => BaseType::Type(Box::new(inner.substitute_generics(map))),
            BaseType::Array { base_type, size } =>
                BaseType::Array {
                    base_type: Box::new(base_type.substitute_generics(map)),
                    size: size.clone(),
                },
            BaseType::Generic(vec) =>
                BaseType::Generic(
                    vec
                        .iter()
                        .map(|t| t.substitute_generics(map))
                        .collect()
                ),
            BaseType::Fn { name, params, return_type } =>
                BaseType::Fn {
                    name: name.clone(),
                    params: params
                        .iter()
                        .map(|p| p.substitute_generics(map))
                        .collect(),
                    return_type: Box::new(return_type.substitute_generics(map)),
                },
            BaseType::Micro { name, params, return_type } =>
                BaseType::Micro {
                    name: name.clone(),
                    params: params
                        .iter()
                        .map(|p| p.substitute_generics(map))
                        .collect(),
                    return_type: Box::new(return_type.substitute_generics(map)),
                },
            BaseType::Macro { name, params, return_type } =>
                BaseType::Macro {
                    name: name.clone(),
                    params: params
                        .iter()
                        .map(|p| p.substitute_generics(map))
                        .collect(),
                    return_type: Box::new(return_type.substitute_generics(map)),
                },
            BaseType::Method { name, params, return_type } =>
                BaseType::Method {
                    name: name.clone(),
                    params: params
                        .iter()
                        .map(|p| p.substitute_generics(map))
                        .collect(),
                    return_type: Box::new(return_type.substitute_generics(map)),
                },
            BaseType::Class { name, fields, methods, constructor, generics } => {
                let substituted_generics = generics
                    .iter()
                    .map(|g| g.substitute_generics(map))
                    .collect();
                BaseType::Class {
                    name: name.clone(),
                    fields: fields.clone(),
                    methods: methods.clone(),
                    constructor: constructor.clone(),
                    generics: substituted_generics,
                }
            }
            BaseType::Struct { name, fields, methods, generics } => {
                let substituted_generics = generics
                    .iter()
                    .map(|g| g.substitute_generics(map))
                    .collect();
                BaseType::Struct {
                    name: name.clone(),
                    fields: fields.clone(),
                    methods: methods.clone(),
                    generics: substituted_generics,
                }
            }
            BaseType::Enum { name, variants, methods, generics } => {
                let substituted_generics = generics
                    .iter()
                    .map(|g| g.substitute_generics(map))
                    .collect();
                BaseType::Enum {
                    name: name.clone(),
                    variants: variants.clone(),
                    methods: methods.clone(),
                    generics: substituted_generics,
                }
            }
            BaseType::Blueprint { name, fields, methods, generics } => {
                let substituted_generics = generics
                    .iter()
                    .map(|g| g.substitute_generics(map))
                    .collect();
                BaseType::Blueprint {
                    name: name.clone(),
                    fields: fields.clone(),
                    methods: methods.clone(),
                    generics: substituted_generics,
                }
            }
            _ => self.clone(),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct FnType {
    pub name: String,
    pub params: Vec<Param>,
    pub return_type: BaseType,
}
#[derive(Debug, Clone, PartialEq)]
pub struct MacroType {
    pub name: String,
    pub params: Vec<Param>,
    pub return_type: BaseType,
}
#[derive(Debug, Clone, PartialEq)]
pub struct ConstructorType {
    pub name: String,
    pub params: Vec<Param>,
}
#[derive(Debug, Clone, PartialEq)]
pub struct VarMetadata {
    pub name: String,
    pub type_node: BaseType,
    pub visibility: Visibility,
    pub editability: Editability,
    pub scope: ScopeType,
    pub is_array: bool,
}
#[derive(Debug, Clone, PartialEq)]
pub struct TypeMetadata {
    pub name: String, // "Node"
    pub ty: BaseType,
    pub fields: HashMap<String, BaseType>, // {"data": Int, "next": UserType("Node")}
    pub constructor: Option<Vec<ConstructorType>>,
    pub methods: HashMap<String, FnType>, // {"set_next": Node.set_next -> void}
    pub handles: Vec<HandleMethods>,
    pub vars: HashMap<String, VarMetadata>,
    pub variants: Option<Vec<EnumVariant>>,
}
#[derive(Debug, Clone, PartialEq)]
pub enum ScopeType {
    //todo : improve it or remove it
    Fn,
    Block,
    Class,
    Struct,
    Blueprint,
    Impl,
    Enum,
    Case,
    Switch,
    Loop,
    Global,
    Label,
    Handle,
    Method,
    Machine,
    Micro,
}
#[derive(Debug, Clone, PartialEq)]
pub struct Param {
    pub name: String,
    pub type_node: BaseType,
    pub default_value: Option<Expr>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Flag {
    // Traits & Type capabilities (Compile-time and Runtime access)
    Compilable,
    Printable,
    Throwable,

    // Scope capabilities
    HasReturn,
    HasYield,
    HasBreak,
    HasContinue,
    HasThrow,
    HasLeave,
    HasExit,
    HasCall,
    HasError,
    HasSwitch,

    // Runtime execution states
    Yielded,
    IsDone,
    Broken,
    Continued,
    Returned,
    Leaved,

    Custom(String),
}

impl Flag {
    pub fn from_str(s: &str) -> Self {
        match s {
            "compilable" | "is_compilable" => Flag::Compilable,
            "printable" | "is_printable" => Flag::Printable,
            "throwable" | "is_throwable" => Flag::Throwable,
            "has_return" => Flag::HasReturn,
            "has_yield" => Flag::HasYield,
            "has_break" => Flag::HasBreak,
            "has_continue" => Flag::HasContinue,
            "has_throw" => Flag::HasThrow,
            "has_leave" => Flag::HasLeave,
            "has_exit" => Flag::HasExit,
            "has_call" => Flag::HasCall,
            "has_error" => Flag::HasError,
            "has_switch" => Flag::HasSwitch,
            "yielded" => Flag::Yielded,
            "is_done" => Flag::IsDone,
            "broken" => Flag::Broken,
            "continued" => Flag::Continued,
            "returned" => Flag::Returned,
            "leaved" => Flag::Leaved,
            _ => Flag::Custom(s.to_string()),
        }
    }

    pub fn as_str(&self) -> String {
        match self {
            Flag::Compilable => "compilable".to_string(),
            Flag::Printable => "printable".to_string(),
            Flag::Throwable => "throwable".to_string(),
            Flag::HasReturn => "has_return".to_string(),
            Flag::HasYield => "has_yield".to_string(),
            Flag::HasBreak => "has_break".to_string(),
            Flag::HasContinue => "has_continue".to_string(),
            Flag::HasThrow => "has_throw".to_string(),
            Flag::HasLeave => "has_leave".to_string(),
            Flag::HasExit => "has_exit".to_string(),
            Flag::HasCall => "has_call".to_string(),
            Flag::HasError => "has_error".to_string(),
            Flag::HasSwitch => "has_switch".to_string(),
            Flag::Yielded => "yielded".to_string(),
            Flag::IsDone => "is_done".to_string(),
            Flag::Broken => "broken".to_string(),
            Flag::Continued => "continued".to_string(),
            Flag::Returned => "returned".to_string(),
            Flag::Leaved => "leaved".to_string(),
            Flag::Custom(s) => s.clone(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ScopeSection {
    Constructor,
    Public,
    Private,
    Static,
    Extends,
    Handle,
    Data,
    Label,
    NotFound,
}

pub type Setting = ScopeSection;

impl ScopeSection {
    pub fn from_str(s: &str) -> Self {
        match s {
            "constructor" | "init" => ScopeSection::Constructor,
            "private" => ScopeSection::Private,
            "public" => ScopeSection::Public,
            "static" => ScopeSection::Static,
            "extends" => ScopeSection::Extends,
            "handle" => ScopeSection::Handle,
            "data" => ScopeSection::Data,
            "label" => ScopeSection::Label,
            _ => ScopeSection::NotFound,
        }
    }

    pub fn as_str(&self) -> String {
        match self {
            ScopeSection::Constructor => "constructor".to_string(),
            ScopeSection::Private => "private".to_string(),
            ScopeSection::Public => "public".to_string(),
            ScopeSection::Static => "static".to_string(),
            ScopeSection::Extends => "extends".to_string(),
            ScopeSection::Handle => "handle".to_string(),
            ScopeSection::Data => "data".to_string(),
            ScopeSection::Label => "label".to_string(),
            ScopeSection::NotFound => "not_found".to_string(),
        }
    }

    pub fn from_token(t: TokenKind) -> Self {
        let s = TokenKind::as_str(&t);
        ScopeSection::from_str(s)
    }
}
#[derive(Debug, Clone, PartialEq, Copy, Eq, Hash)]
pub enum HandleMethods {
    IndexAccess,
    Display, // display() to make it printable

    Increment,
    Decrement,
    PreIncrement,
    PreDecrement,

    Add, // add
    Sub, // sub
    Mul, // mul
    Div, // div
    Mod, // mod

    Not, // !
    Negate, //-

    And, // and &&
    Or, // or ||

    Arrow,
    ArrowAssign,
    FatArrow,
    Equal,

    PartialEqual, // ==
    NotEqual, // !=
    GreaterThan, // >
    LessThan, // <
    GreaterThanEqual, // >=
    LessThanEqual, // <=

    Iterator, // iter() or iterator()
    Next, // next()

    Copy, // copy()
    Call,
    Leave,
    Yield,
    Break, // break
    Continue, // continue
    Return, // return
    Error, // handle error(e: Error) (receiver/catcher)
    Throw, // handle throw() -> Error (throwable capability)
    //Exit,
    Drop,
    IsDone,
    Default,
    NotFound,
}
impl HandleMethods {
    pub fn from_str(s: &str) -> Self {
        match s {
            "index_access" => HandleMethods::IndexAccess,
            "display" => HandleMethods::Display,
            "add" => HandleMethods::Add,
            "increment" => HandleMethods::Increment,
            "decrement" => HandleMethods::Decrement,
            "pre_increment" => HandleMethods::PreIncrement,
            "pre_decrement" => HandleMethods::PreDecrement,
            "sub" => HandleMethods::Sub,
            "mul" => HandleMethods::Mul,
            "div" => HandleMethods::Div,
            "mod" => HandleMethods::Mod,
            "not" => HandleMethods::Not,
            "negate" => HandleMethods::Negate,
            "arrow" => HandleMethods::Arrow,
            "arrow_assign" => HandleMethods::ArrowAssign,
            "fat_arrow" => HandleMethods::FatArrow,
            "equal" => HandleMethods::Equal,
            "partial_equal" => HandleMethods::PartialEqual,
            "not_equal" => HandleMethods::NotEqual,
            "greater_than" => HandleMethods::GreaterThan,
            "less_than" => HandleMethods::LessThan,
            "greater_than_equal" => HandleMethods::GreaterThanEqual,
            "less_than_equal" => HandleMethods::LessThanEqual,
            "and" => HandleMethods::And,
            "or" => HandleMethods::Or,
            "iterator" | "iter" => HandleMethods::Iterator,
            "next" => HandleMethods::Next,
            "copy" => HandleMethods::Copy,
            "call" => HandleMethods::Call,
            "leave" => HandleMethods::Leave,
            "yield" => HandleMethods::Yield,
            "error" => HandleMethods::Error,
            "throw" => HandleMethods::Throw,
            "break" => HandleMethods::Break,
            "continue" => HandleMethods::Continue,
            "return" => HandleMethods::Return,
            "drop" => HandleMethods::Drop,
            "is_done" => HandleMethods::IsDone,
            // "exit" => HandleMethods::Exit,
            "default" => HandleMethods::Default,
            _ => HandleMethods::NotFound,
        }
    }
    pub fn as_str(&self) -> &str {
        match self {
            HandleMethods::IndexAccess => "index_access",
            HandleMethods::Display => "display",
            HandleMethods::Iterator => "iter",
            HandleMethods::Next => "next",
            HandleMethods::Copy => "copy",
            HandleMethods::Add => "add",
            HandleMethods::Increment => "increment",
            HandleMethods::Decrement => "decrement",
            HandleMethods::PreIncrement => "pre_increment",
            HandleMethods::PreDecrement => "pre_decrement",

            HandleMethods::Sub => "sub",
            HandleMethods::Mul => "mul",
            HandleMethods::Div => "div",
            HandleMethods::Mod => "mod",
            HandleMethods::Not => "not",
            HandleMethods::Negate => "negate",
            HandleMethods::Arrow => "arrow",
            HandleMethods::ArrowAssign => "arrow_assign",
            HandleMethods::FatArrow => "fat_arrow",
            HandleMethods::Equal => "equal",
            HandleMethods::PartialEqual => "partial_equal",
            HandleMethods::NotEqual => "not_equal",
            HandleMethods::GreaterThan => "greater_than",
            HandleMethods::LessThan => "less_than",
            HandleMethods::GreaterThanEqual => "greater_than_equal",
            HandleMethods::LessThanEqual => "less_than_equal",
            HandleMethods::And => "and",
            HandleMethods::Or => "or",
            HandleMethods::Drop => "drop",
            HandleMethods::IsDone => "is_done",
            //HandleMethods::Exit => "exit",
            HandleMethods::Break => "break",
            HandleMethods::Continue => "continue",
            HandleMethods::Return => "return",
            HandleMethods::Leave => "leave",
            HandleMethods::Error => "error",
            HandleMethods::Throw => "throw",
            HandleMethods::Yield => "yield",
            HandleMethods::Call => "call",
            HandleMethods::Default => "default",
            HandleMethods::NotFound => "not_found",
        }
    }
}
#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    LiteralInt(i64),
    LiteralFloat(f64),
    LiteralString(String),
    LiteralBool(bool),
    LiteralChar(char),
    LiteralVoid,
    LiteralUndefined,
    ArrayLiteral(Vec<Expr>),
    Spread(Box<Expr>),
    Identifier(String),
    Super,
    This,
    Global,

    ObjectLiteral(Vec<Stmt>),

    Instantiate {
        target: Box<Expr>,
        args: Vec<Expr>,
    },
    ArrayAllocate {
        type_node: BaseType,
        size: Box<Expr>,
        length: Option<Box<Expr>>,
    },
    New {
        type_node: BaseType,
        target: Box<Expr>,
    },
    TypeOf {
        target: Box<Expr>,
    },
    SizeOf {
        target: Box<Expr>,
    },
    ToString {
        target: Box<Expr>,
    },
    UnaryOp {
        operator: String,
        operand: Box<Expr>,
    },
    Default(Option<BaseType>),

    IndexAccess {
        object: Box<Expr>,
        indices: Vec<Expr>,
    },

    Call {
        callee: Box<Expr>,
        args: Vec<Expr>,
    },

    MacroCall {
        callee: Box<Expr>,
        args: Vec<Expr>,
    },

    PropertyAccess {
        object: Box<Expr>,
        property: String,
    },

    NamespaceAccess {
        namespace: String,
        property: Box<Expr>,
    },

    BinaryOp {
        left: Box<Expr>,
        operator: String,
        right: Box<Expr>,
    },

    PrefixUpdate {
        operator: String, // "++" or "--"
        right: Box<Expr>,
    },
    PostfixUpdate {
        left: Box<Expr>,
        operator: String,
    },
    Lambda {
        params: Vec<Param>,
        return_type: Option<BaseType>,
        body: Vec<Stmt>,
    },
}
impl Expr {
    pub fn as_str(&self) -> String {
        match self {
            Expr::LiteralInt(i) => i.to_string(),
            Expr::LiteralFloat(f) => f.to_string(),
            Expr::LiteralString(s) => format!("\"{}\"", s),
            Expr::LiteralChar(c) => format!("'{}'", c),
            Expr::LiteralBool(val) => {
                if *val { "true".to_string() } else { "false".to_string() }
            }
            Expr::ArrayLiteral(elements) => {
                let mut elems_code = Vec::new();
                for el in elements {
                    elems_code.push(el.as_str());
                }
                format!("[{}]", elems_code.join(", "))
            }
            Expr::Spread(operand) => format!("...{}", operand.as_str()),
            Expr::ObjectLiteral(_stmts) => "unimplemented".to_string(),
            Expr::Identifier(name) => format!("{}", name),
            Expr::This => "this".to_string(),
            Expr::Super => "super".to_string(), // will be handled in PropertyAccess
            Expr::Global => "global".to_string(),
            Expr::BinaryOp { left: _, operator: _, right: _ } => "unimplemented".to_string(),
            Expr::PostfixUpdate { left, operator } => format!("{}{}", left.as_str(), operator),
            Expr::PrefixUpdate { right, operator } => format!("{}{}", operator, right.as_str()),
            Expr::Lambda { .. } => "lambda".to_string(),
            Expr::UnaryOp { operator, operand } => format!("{}{}", operator, operand.as_str()),
            Expr::IndexAccess { object, indices } => {
                let idxs: Vec<String> = indices
                    .iter()
                    .map(|i| i.as_str())
                    .collect();
                format!("{}[{}]", object.as_str(), idxs.join(", "))
            }
            Expr::Call { callee, args } =>
                format!(
                    "{}({})",
                    callee.as_str(),
                    args
                        .iter()
                        .map(|a| a.as_str())
                        .collect::<Vec<String>>()
                        .join(", ")
                ),
            Expr::Instantiate { target, args } =>
                format!(
                    "{}({})",
                    target.as_str(),
                    args
                        .iter()
                        .map(|a| a.as_str())
                        .collect::<Vec<String>>()
                        .join(", ")
                ),

            Expr::PropertyAccess { object, property } => {
                format!("{}.{}", object.as_str(), property.as_str())
            }
            Expr::NamespaceAccess { namespace, property } =>
                format!("{}::{}", namespace, property.as_str()),
            Expr::ArrayAllocate { type_node, size, length: _ } =>
                format!("new {}[{}]", type_node.as_str(), size.as_str()),
            Expr::New { type_node, target } => {
                format!("new {}[{}]", type_node.as_str(), target.as_str())
            }
            _ => unreachable!(),
        }
    }
}

pub fn type_from_expr(expr: &Expr) -> BaseType {
    match expr {
        Expr::LiteralInt(_) => BaseType::Int(Size::S32),
        Expr::LiteralFloat(_) => BaseType::Float(Size::S64),
        Expr::LiteralBool(_) => BaseType::Bool,
        Expr::LiteralString(_) => BaseType::Str,
        Expr::LiteralChar(_) => BaseType::Char,
        Expr::New { type_node, .. } => type_node.clone(),
        Expr::ArrayAllocate { type_node, .. } => BaseType::Array {
            base_type: Box::new(type_node.clone()),
            size: Box::new(None),
        },
        _ => BaseType::Unknown,
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Place {
    Local,
    Heap,
}
#[derive(Debug, Clone, PartialEq)]
pub enum Pattern {
    Identifier(String),
    Tuple(Vec<Pattern>),
    Struct {
        name: Option<String>,
        fields: Vec<String>,
    },
}
#[derive(Debug, Clone, PartialEq)]
pub enum Decl {
    VarDecl {
        visibility: Visibility,
        editability: Editability,
        type_node: BaseType,
        place: Place,
        assign_op: String,
        name: String,
        value: Expr,
    },
    DestructureDecl {
        visibility: Visibility,
        editability: Editability,
        type_node: BaseType,
        pattern: Pattern,
        assignments: Vec<(String, Expr)>,
        assign_op: String,
    },
    ObjectDestructureDecl {
        visibility: Visibility,
        editability: Editability,
        type_name: Option<String>,
        fields: Vec<(BaseType, String)>,
        rhs: Expr,
    },
    ArrayDecl {
        visibility: Visibility,
        editability: Editability,
        type_node: BaseType,
        assign_op: String,
        name: String,
        length: Expr,
        value: Expr,
    },
    BlockDecl {
        visibility: Visibility,
        name: String,
        return_type: Option<BaseType>,
        statements: Vec<Stmt>,
    },
    MicroDecl {
        visibility: Visibility,
        name: String,
        generics: Option<Vec<String>>,
        params: Vec<Param>,
        return_type: Option<BaseType>,
        body: Vec<Stmt>,
    },
    ObjectDecl {
        visibility: Visibility,
        name: String,
        fields: Vec<ObjectField>,
    },
    ClassDecl {
        visibility: Visibility,
        name: String,
        extends: Option<String>,
        handles: Vec<HandleMethods>,
        public_block: Vec<Decl>,
        private_block: Vec<Decl>,
        static_block: Vec<Decl>,
        generics: Vec<BaseType>,
        handle_block: Vec<Decl>,
        constructor: Option<Vec<ConstructorDecl>>,
    },
    BlueprintDecl {
        visibility: Visibility,
        name: String,
        generics: Vec<BaseType>,
        definition: BlueprintDef,
    },
    ImplDecl {
        target: String,
        target_generics: Vec<BaseType>,
        is_handle_impl: bool,
        methods: Vec<Decl>,
        handle_block: Vec<Decl>,
    },

    StructDecl {
        visibility: Visibility,
        name: String,
        handles: Vec<HandleMethods>,
        public_block: Vec<Decl>,
        private_block: Vec<Decl>,
        handle_block: Vec<Decl>,
        static_block: Vec<Decl>,
        constructor: Option<Vec<ConstructorDecl>>,
    },

    EnumDecl {
        visibility: Visibility,
        name: String,
        generics: Vec<BaseType>,
        handles: Vec<HandleMethods>,
        handle_block: Vec<Decl>,
        variants: Vec<EnumVariant>,
    },
    FnDecl {
        visibility: Visibility,
        is_virtual: bool,
        is_abstract: bool,
        name: String,
        params: Vec<Param>,
        return_type: BaseType,
        body: Vec<Stmt>,
    },
    ExternFnDecl {
        abi: String,
        name: String,
        params: Vec<Param>,
        return_type: BaseType,
        alias: Option<String>,
    },
    ExternBlockDecl {
        abi: String,
        decls: Vec<Decl>,
    },
    LabelDecl {
        name: String,
        body: Vec<Stmt>,
    },
    Import {
        module_path: Vec<String>,
        imports: Option<Vec<String>>,
        abi: Option<String>,
        alias: Option<String>,
    },

    /// `machine Name -> { @label -> { ... } ... handle -> { fn call() ... fn leave() ... } }`
    /// Pure state machine: @init label runs once, other labels are states,
    /// data is stored as `this.field` assignments within labels.
    MachineDecl {
        visibility: Visibility,
        name: String,
        return_type: BaseType,
        labels: Vec<Decl>,
        handle_block: Vec<Decl>,
    },
    DefineDecl {
        visibility: Visibility,
        name: String,
        type_alias: Option<BaseType>,
        value: Option<Expr>,
    },
    MacroDecl {
        visibility: Visibility,
        name: String,
        params: Vec<Param>,
        body: Vec<Stmt>,
    },
    CompileDecl {
        name: String,
        decls: Vec<Decl>,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub enum Stmt {
    Declaration(Decl),
    Block(Vec<Stmt>),
    ThisBlock(Vec<Stmt>),

    CaseStmt {
        option: Expr,
        set: Expr,
        body: Vec<Stmt>,
    },
    SwitchStmt {
        name: String,
        condition: Expr,
        cases: Vec<Stmt>,
    },
    ReturnStmt(Option<Expr>),
    ForIn {
        item_decl: Box<Stmt>,
        iterable: Expr,
        body: Vec<Stmt>,
    },
    BreakStmt,
    ContinueStmt,
    LeaveStmt,
    YieldStmt(Option<Expr>),
    CallStmt(Expr),
    ExpressionStmt(Expr),

    ThrowStmt(Expr),
    TryCatchStmt {
        try_block: Vec<Stmt>,
        catch_param: String,
        catch_block: Vec<Stmt>,
    },

    // ── Control Flow ──────────────────────────────────────────
    // set name -> value;
    // set obj.field -> value;   (property chain reassignment)
    /// Reassignment statement — `set <target> = <value>;`
    /// target can be a simple identifier or property chain (obj.field)
    /// target can be a simple identifier or property chain (obj.field)
    ReassignStmt {
        target: Expr,
        op: String,
        value: Expr,
    },
    AddPropertyStmt {
        kind_name: String, // "label" or "flag"
        value: Expr,
    },
    GotoStmt(Expr),
    IfStmt {
        condition: Expr,
        then_block: Vec<Stmt>,
        /// Optional else block
        else_block: Option<Vec<Stmt>>,
    },

    /// `loop N -> { ... }` or `loop -> { ... }` (infinite)
    /// or `loop N -> scope_name(args)` (scope call)
    LoopStmt {
        /// Iteration count — None = infinite loop
        count: Option<Expr>,
        body: EitherBlock,
    },

    /// `while (cond) -> { ... }`
    /// or `while (cond) -> scope_name(args)` (scope call)
    WhileStmt {
        condition: Expr,
        body: EitherBlock,
    },

    /// `do -> { ... } while (cond);`
    DoWhileStmt {
        body: EitherBlock,
        condition: Expr,
    },

    /// `for (init; cond; inc) -> { ... }`
    ForStmt {
        init: Option<Box<Stmt>>,
        condition: Option<Expr>,
        increment: Option<Box<Stmt>>,
        body: EitherBlock,
    },

    /// `for (item in iterable) -> { ... }`
    ForInStmt {
        item: Box<Stmt>,
        iterable: Expr,
        body: EitherBlock,
    },
    DelStmt {
        target: Expr,
        is_array: bool,
    },
    UsingStmt(String),
}

/// Loop/while body representation:
///   Inline: standard block `{ ... }`
///   ScopeCall: invocation of a custom or looped scope
///     e.g. `while (cond) -> my_looped_scope()`
#[derive(Debug, Clone, PartialEq)]
pub enum EitherBlock {
    /// `{ statements... }` — inline block
    Inline(Vec<Stmt>),
    /// `scope_name(args)`
    External(Expr),
}

#[derive(Debug, Clone, PartialEq)]
pub struct ConstructorDecl {
    pub params: Vec<Param>,
    pub expected_types: Vec<String>,
    pub body: Vec<Stmt>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct HandleDecl {
    pub target_flag: String,
    pub body: Vec<Stmt>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum EnumVariantPayload {
    None,
    Tuple(Vec<BaseType>),
    Struct(Vec<Param>),
}

#[derive(Debug, Clone, PartialEq)]
pub struct EnumVariant {
    pub name: String,
    pub payload: EnumVariantPayload,
    pub data_type: Option<BaseType>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum BlueprintDef {
    Explicit(Vec<BlueprintField>),
    FromExistingObject(String),
    FromTemporaryObject(Vec<ObjectField>),
}

#[derive(Debug, Clone, PartialEq)]
pub struct BlueprintField {
    pub is_static: bool,
    pub name: String,
    pub type_node: BaseType,
    pub default_value: Option<Expr>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ObjectField {
    pub name: String,
    pub value: Expr,
}
#[derive(Debug, Clone, PartialEq)]
pub struct Program {
    pub statements: Vec<Stmt>,
}
/*
the scope is all of what inside a file or a {};
the "this" is only for class and struct and fn and custom and block not for statement scope
only fn has a params
all of the function or custom or block or clas or struct has a error "throw new error("test error");"
 also i want to add a new keyword "enable" to enable a certin flag is already exists like (is_return , is_exit , is_break , is_throw). that all the default flags that we have
  but i want in a custom one to be able to enable and disable flags as you like
only fn and block has a return and its flag
only struct and class has static
only blocks and custom typed scopes can add new flags
only global and what is in has exit (all have exit)
only the scope of a loop and what is in has break and its flag
only array has size
only array and custom types and str can length
only array and custom types and str can data
 while con -> fn(); we was able to do that in the cpp version so to make it still doable
  i think that is better to add a new scope type ( looped) that can have break in it .

  i think we need a way to add a new component to a scope ( like length and size ) for example

  scope x -> {
    add int(16) length;
  }
i thick that will fix our current problem and also will make the code more editable

old refrance
scope : {
params : {names || vars}, //  name is like a ref
public : {vars , fns},
private : {vars , fns},
static : {vars , fns},
settings : {custom_index_access, custom_constructor, custom_keyword, custom_param_body,param,private,public,static,length,size,data,error...},
flags : {isReturn , isExit, isBreak ,isThrow}
return : type() ,
statement : statement,
size : type() , // for array and custom types
length : type() , // for array , str and custom types
data : array<type,size,length> // for array and str and custom types but l will make list and str use linked list  to be more efficient so i will implement for them
error : e() ,
name : "id",
type : {struct || class || object (instance) || block || global || Fn || looped || custom || array || str } ,
}

*/
/*
what the custom sys does (all the setting start with custom_ ) :
1.index_access :
 it enable the cusom scope to be accessed like that custom_scope_name[index];
 but of course  you need to decare a fn with the name "index_access" in handle will handle the index_access
 for ex if i have a list custom scope with this setting and a fn called access take a index and return a name (ref)
 what i can do in handle is will be like that
 handle -> {
  fn index_access (index : int(32)) -> name {
    let name  ele= this.access(index);
    return ele;
  }
 }
 2.constructor :
  that will enable us to change the way we deal with what come after -> in constrcuting that way
  list(int(32)) li -> [1,2,3]
  it will treat the [1,2,3] as a params to the constructor
  and it will not need a handle for now at least
  in defining that way
  list(int(32)) li = new list([1,2,3]);
  that will work like the frist one
  so what is the constructor will lock
  _( arr : name) -> {
    this.extend(arr);
}
3.keyword:
 is the easy one it basically make you change the keyword you will use to use the custom scope
 so for ex
 scope List -> {
     type -> custom;
     ! no more keyword
   * keyword "my_list";
 }
 my_list(int(32)) li -> [1,2,3];
 List(int(32)) li -> [1,2,3]; // will not work now
 4.param_body:
 it allow to use a params after the name of the scope or the keyword
 like this
 list(int(32)) li -> [1,2,3]; // the (int(32)) is the param we can use that only with the param_body setting on
 it also will not use handle so the param part in the scope declaration will handle that actually i think we will use it in anther thing and make will you enable param do that insted
 and make the param body use the <> and have a custom handle for it or a new block called custom param
 so
 array<int(32)>(3) arr -> [1,2,3] so now the custom param take the type and size and the normal one take the length
 ! update we now have the array like c : int(32) arr[3] = [1,2,3];
 * i think that is all of we have for now you can enhance it and add more
*/
/*

TypeNode {
    type_name: "List".to_string(),
    base_type: BaseType::Generic(vec![BaseType::Int]), // Generic parameters
    size: None
}

TypeNode {
    type_name: "List".to_string(),
    base_type: BaseType::Custom(Box::new(Hashmap{"T" => Generic(vec![BaseType::Unknown]) , "head"=> BaseType::Custom(Box::new(Hashmap{"data" => BaseType::Unknown, "next" => BaseType::Unknown})), "tail" => BaseType::Custom(Box::new(Hashmap{"data" => BaseType::Unknown, "next" => BaseType::Unknown}))})),
    size: None
}
List<int32> li -> [1,2,3];
//or
List<int32> li = new List([1,2,3]);

*/
