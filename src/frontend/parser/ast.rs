use std::collections::HashMap;

use crate::frontend::lexer::token::TokenKind::{ self, For };

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
pub enum BaseType {
    Int8,
    Int16,
    Int32,
    Int64,
    Int128,
    Float32,
    Float64,
    Char,
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

    Custom {
        name: String,
        fields: Box<HashMap<String, BaseType>>,
        methods: Box<HashMap<String, FnType>>,
        generics: Vec<BaseType>,
        params: Vec<Param>,
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

    Method {
        params: Vec<BaseType>,
        return_type: Box<BaseType>,
    },
    Generic(Vec<BaseType>),

    Unknown,
    Error,
    New(String),
}
impl BaseType {
    pub fn as_str(&self) -> String {
        match self {
            BaseType::Int8 => "int8".to_string(),
            BaseType::Int16 => "int16".to_string(),
            BaseType::Int32 => "int32".to_string(),
            BaseType::Int64 => "int64".to_string(),
            BaseType::Int128 => "int128".to_string(),
            BaseType::Float32 => "float32".to_string(),
            BaseType::Float64 => "float64".to_string(),
            BaseType::Char => "char".to_string(),
            BaseType::Bool => "bool".to_string(),
            BaseType::Void => "void".to_string(),
            BaseType::Modify(t) => format!("modify<{}>", t.as_str()),
            BaseType::Copy(t) => format!("copy<{}>", t.as_str()),
            BaseType::Name(t) => format!("name<{}>", t.as_str()),
            BaseType::Pointer(t) => format!("pointer<{}>", t.as_str()),
            BaseType::Type(t) => format!("type<{}>", t.as_str()),
            BaseType::Array { base_type, size } => {
                format!("array<{}[{}]>", base_type.as_str(), size.clone().unwrap().as_str())
            }
            BaseType::Custom { name, .. } => format!("custom<{}>", name),
            BaseType::Struct { name, .. } => format!("struct<{}>", name),
            BaseType::Class { name, .. } => format!("class<{}>", name),
            BaseType::Enum { name, .. } => format!("enum<{}>", name),
            BaseType::Blueprint { name, .. } => format!("blueprint<{}>", name),
            BaseType::Method { .. } => "method".to_string(),
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
            "int8" => BaseType::Int8,
            "int16" => BaseType::Int16,
            "int32" | "int" => BaseType::Int32,
            "int64" => BaseType::Int64,
            "int128" => BaseType::Int128,
            "float32" | "float" => BaseType::Float32,
            "float64" => BaseType::Float64,
            "char" => BaseType::Char,
            "bool" => BaseType::Bool,
            "void" => BaseType::Void,
            "name" => BaseType::Name(Box::new(BaseType::Generic(Vec::new()))), // name<T,T2,T3> //T , T2 , T3 are the expected types for the name but after
            "modify" => BaseType::Modify(Box::new(BaseType::Unknown)),
            "copy" => BaseType::Copy(Box::new(BaseType::Unknown)),
            "pointer" => BaseType::Pointer(Box::new(BaseType::Unknown)),
            "type" => BaseType::Type(Box::new(BaseType::Unknown)),
            "unknown" => BaseType::Unknown,
            "error" => BaseType::Error,
            _ => BaseType::Unknown,
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
    pub fields: HashMap<String, BaseType>, // {"data": Int, "next": UserType("Node")}
    pub constructor: Option<Vec<ConstructorType>>,
    pub params: Vec<Param>, // {"value": Int}
    pub generics: Vec<BaseType>, // {"T": UserType("Type")}
    pub methods: HashMap<String, FnType>, // {"set_next": Node.set_next -> void}
    pub handles: Vec<HandleMethods>,
    pub vars: HashMap<String, VarMetadata>,
    pub is_enum: bool,
    pub variants: Option<Vec<EnumVariant>>,
}
#[derive(Debug, Clone, PartialEq)]
pub enum ScopeType {
    Fn,
    Block,
    Class,
    Struct,
    Custom,
    Impl,
    Enum,
    Case,
    Switch,
    Loop,
    Global,
    Label,
    Handle,
}
#[derive(Debug, Clone, PartialEq)]
pub struct Param {
    pub name: String,
    pub type_node: BaseType,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Flag {
    HasReturn,
    HasBreak,
    HasThrow,
    HasError,
    HasSwitch,
    HasExit,
    Custom(String),
}

impl Flag {
    pub fn from_str(s: &str) -> Self {
        match s {
            "has_return" => Flag::HasReturn,
            "has_break" => Flag::HasBreak,
            "has_throw" => Flag::HasThrow,
            "has_switch" => Flag::HasSwitch,
            "has_error" => Flag::HasError,
            "has_exit" => Flag::HasExit,
            _ => Flag::Custom(s.to_string()),
        }
    }

    pub fn as_str(&self) -> String {
        match self {
            Flag::HasReturn => "has_return".to_string(),
            Flag::HasBreak => "has_break".to_string(),
            Flag::HasThrow => "has_throw".to_string(),
            Flag::HasError => "has_error".to_string(),
            Flag::HasSwitch => "has_switch".to_string(),
            Flag::HasExit => "has_exit".to_string(),
            Flag::Custom(s) => s.clone(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Setting {
    //global to add a list of settings
    All, // all of the settings
    OOP, // private , public , static , extends , constructor
    Function, // param , statement , return
    Debug, // error , break , throw , exit , return
    State, // leave , yield , goto , label , call
    Call,
    //fn
    Param,
    Statement,
    Return,
    //switch
    Case,
    Break,
    //oop
    Private,
    Public,
    Static,
    Extends,
    Constructor,
    // enum
    Variants,
    // scope settings
    Label, //goto will be only supported in label and the call handle method
    Yield,
    Leave,
    // array and str
    Data,
    //all
    Throw,
    Error,
    Exit,
    Handle,
    NotFound,
}

impl Setting {
    pub fn from_str(s: &str) -> Self {
        match s {
            //global -------
            "all" => Setting::All,
            "oop" => Setting::OOP,
            "function" => Setting::Function,
            "debug" => Setting::Debug,
            "state" => Setting::State,
            //fn ---
            "param" => Setting::Param,
            "statement" => Setting::Statement,
            "return" => Setting::Return,
            //switch -------
            "case" => Setting::Case,
            "break" => Setting::Break,
            //oop -------
            "init" => Setting::Constructor,
            "private" => Setting::Private,
            "public" => Setting::Public,
            "static" => Setting::Static,
            "extends" => Setting::Extends,
            "label" => Setting::Label,
            "yield" => Setting::Yield,
            "leave" => Setting::Leave,
            "call" => Setting::Call,
            //enum --
            "variants" => Setting::Variants,
            "data" => Setting::Data,
            "error" => Setting::Error,
            "handle" => Setting::Handle,
            _ => Setting::NotFound,
        }
    }

    pub fn as_str(&self) -> String {
        match self {
            //global -----
            Setting::All => "all".to_string(),
            Setting::OOP => "oop".to_string(),
            Setting::Function => "function".to_string(),
            Setting::Debug => "debug".to_string(),
            Setting::State => "state".to_string(),
            //fn -------
            Setting::Param => "param".to_string(),
            Setting::Statement => "statement".to_string(),
            Setting::Return => "return".to_string(),
            //switch -------
            Setting::Case => "case".to_string(),
            Setting::Break => "break".to_string(),
            //oop -------
            Setting::Private => "private".to_string(),
            Setting::Public => "public".to_string(),
            Setting::Static => "static".to_string(),
            Setting::Extends => "extends".to_string(),
            Setting::Constructor => "constructor".to_string(),
            Setting::Leave => "leave".to_string(),
            Setting::Yield => "yield".to_string(),
            Setting::Label => "label".to_string(),
            Setting::Call => "call".to_string(),
            //enum -------
            Setting::Variants => "variants".to_string(),
            Setting::Data => "data".to_string(),
            //all ------
            Setting::Error => "error".to_string(),
            Setting::Exit => "exit".to_string(),
            Self::Throw => "throw".to_string(),
            Setting::Handle => "handle".to_string(),
            Setting::NotFound => "not_found".to_string(),
        }
    }
    pub fn from_token(t: TokenKind) -> Self {
        let s = TokenKind::as_str(&t);
        return Setting::from_str(s);
    }
}
#[derive(Debug, Clone, PartialEq, Copy, Eq, Hash)]
pub enum HandleMethods {
    IndexAccess, // scope[i]
    IndexAssign, // scope[i] = value;
    IndexIncrement, //scope[i]++
    IndexDecrement, //scope[i]--
    IndexPreIncrement, //++scope[i]
    IndexPreDecrement, //--scope[i]
    IndexAdd, //scope[i] += value;
    IndexSub, //scope[i] -= value;
    IndexMul, //scope[i] *= value;
    IndexDiv, //scope[i] /= value;
    IndexMod, //scope[i] %= value;
    Display, // log() or to_string()
    Add, //+ operator
    Increment, //++ operator
    Decrement, //-- operator
    PreIncrement, //++ operator
    PreDecrement, //-- operator
    Sub, //- operator
    Mul, //* operator
    Div,
    /// operator
    Mod, //% operator
    Not, // ! operator
    Negate, //- operator
    Arrow, //-> operator
    ArrowAssign, //-> operator
    FatArrow, //=> operator
    Equal, //= operator
    PartialEqual, //== operator
    NotEqual, // != operator
    GreaterThan, //> operator
    LessThan, //< operator
    GreaterThanEqual, //>= operator
    LessThanEqual, //<= operator
    And, // && operator
    Or, // || operator
    Iterator, // for in loop
    Next, // for in loop
    Length, // length()
    Size, // sizeof()
    Call, // handle the call of the scope : scope();
    Leave, // leave
    Yield, // yield
    Data, //todo : remove this
    Break, // break
    Error, // error
    Exit, // exit
    Drop, //todo:  make it essentially the same as exit and the user has to implement it specifically in custom scope
    NotFound,
}
impl HandleMethods {
    pub fn from_str(s: &str) -> Self {
        match s {
            "index_access" => HandleMethods::IndexAccess,
            "index_assign" => HandleMethods::IndexAssign,
            "index_increment" => HandleMethods::IndexIncrement,
            "index_decrement" => HandleMethods::IndexDecrement,
            "index_pre_increment" => HandleMethods::IndexPreIncrement,
            "index_pre_decrement" => HandleMethods::IndexPreDecrement,
            "index_add" => HandleMethods::IndexAdd,
            "index_sub" => HandleMethods::IndexSub,
            "index_mul" => HandleMethods::IndexMul,
            "index_div" => HandleMethods::IndexDiv,
            "index_mod" => HandleMethods::IndexMod,
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
            "iterator" => HandleMethods::Iterator,
            "next" => HandleMethods::Next,
            "length" => HandleMethods::Length,
            "size" => HandleMethods::Size,
            "call" => HandleMethods::Call,
            "leave" => HandleMethods::Leave,
            "yield" => HandleMethods::Yield,
            "data" => HandleMethods::Data,
            "has_error" => HandleMethods::Error,
            "exit" => HandleMethods::Exit,
            _ => {
                println!("DEBUG: Handle method not found: {}", s);
                HandleMethods::NotFound
            }
        }
    }
    pub fn as_str(&self) -> &str {
        match self {
            HandleMethods::IndexAccess => "index_access",
            HandleMethods::IndexAssign => "index_assign",
            HandleMethods::IndexIncrement => "index_increment",
            HandleMethods::IndexDecrement => "index_decrement",
            HandleMethods::IndexPreIncrement => "index_pre_increment",
            HandleMethods::IndexPreDecrement => "index_pre_decrement",
            HandleMethods::IndexAdd => "index_add",
            HandleMethods::IndexSub => "index_sub",
            HandleMethods::IndexMul => "index_mul",
            HandleMethods::IndexDiv => "index_div",
            HandleMethods::IndexMod => "index_mod",
            HandleMethods::Display => "display",
            HandleMethods::Iterator => "iterator",
            HandleMethods::Next => "next",
            HandleMethods::Length => "length",
            HandleMethods::Size => "size",
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
            HandleMethods::Exit => "exit",
            HandleMethods::Break => "break",
            HandleMethods::Leave => "leave",
            HandleMethods::Error => "error",
            HandleMethods::Yield => "yield",
            HandleMethods::Data => "data",
            HandleMethods::Call => "call",
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
    ArrayLiteral(Vec<Expr>),
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

    IndexAccess {
        object: Box<Expr>,
        index: Box<Expr>,
    },

    Call {
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
            Expr::ObjectLiteral(stmts) => "unimplemented".to_string(),
            Expr::Identifier(name) => format!("{}", name),
            Expr::This => "this".to_string(),
            Expr::Super => "super".to_string(), // will be handled in PropertyAccess
            Expr::Global => "global".to_string(),
            Expr::BinaryOp { left, operator, right } => "unimplemented".to_string(),
            Expr::PostfixUpdate { left, operator } => format!("{}{}", left.as_str(), operator),
            Expr::PrefixUpdate { right, operator } => format!("{}{}", operator, right.as_str()),
            Expr::UnaryOp { operator, operand } => format!("{}{}", operator, operand.as_str()),
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
            Expr::ArrayAllocate { type_node, size, length } =>
                format!("new {}[{}]", type_node.as_str(), size.as_str()),
            Expr::New { type_node, target } => {
                format!("new {}[{}]", type_node.as_str(), target.as_str())
            }
            _ => unreachable!(),
        }
    }
}
#[derive(Debug, Clone, PartialEq)]
pub enum Place {
    Local,
    Heap,
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
        is_exported: bool,
        name: String,
        statements: Vec<Stmt>,
    },
    ObjectDecl {
        is_exported: bool,
        name: String,
        fields: Vec<ObjectField>,
    },
    CustomDecl {
        is_exported: bool,
        name: String,
        settings: Option<Vec<Setting>>,
        handles: Option<Vec<HandleMethods>>,
        params: Option<Vec<Param>>,
        flags: Option<Vec<Flag>>,
        labels: Option<Vec<String>>,
        data: Option<Expr>,
        extends: String,
        return_type: Option<BaseType>,
        public_block: Option<Vec<Decl>>,
        private_block: Option<Vec<Decl>>,
        static_block: Option<Vec<Decl>>,
        statements: Option<Vec<Stmt>>,
        label_blocks: Option<Vec<Decl>>,
        variant_block: Option<Vec<EnumVariant>>,
        generics: Option<Vec<BaseType>>,
        handle_block: Option<Vec<Decl>>,
        constructor: Option<Vec<ConstructorDecl>>,
    },

    ClassDecl {
        is_exported: bool,
        name: String,
        extends: Option<String>,
        handles: Vec<HandleMethods>,
        settings: Vec<Setting>,
        public_block: Vec<Decl>,
        private_block: Vec<Decl>,
        static_block: Vec<Decl>,
        generics: Vec<BaseType>,
        handle_block: Vec<Decl>,
        constructor: Option<Vec<ConstructorDecl>>,
    },
    BlueprintDecl {
        is_exported: bool,
        name: String,
        generics: Vec<BaseType>,
        definition: BlueprintDef,
    },
    ImplDecl {
        target: String,
        methods: Vec<Decl>,
    },

    StructDecl {
        is_exported: bool,
        name: String,
        handles: Vec<HandleMethods>,
        settings: Vec<Setting>,
        public_block: Vec<Decl>,
        private_block: Vec<Decl>,
        handle_block: Vec<Decl>,
        static_block: Vec<Decl>,
        constructor: Option<Vec<ConstructorDecl>>,
    },

    EnumDecl {
        is_exported: bool,
        name: String,
        generics: Vec<BaseType>,
        handles: Vec<HandleMethods>,
        settings: Vec<Setting>,
        handle_block: Vec<Decl>,
        variants: Vec<EnumVariant>,
    },
    FnDecl {
        is_exported: bool,
        name: String,
        params: Vec<Param>,
        return_type: BaseType,
        body: Vec<Stmt>,
    },
    LabelDecl {
        name: String,
        body: Vec<Stmt>,
    },
    Import {
        module_path: Vec<String>,
        imports: Option<Vec<String>>,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub enum Stmt {
    Declaration(Decl),

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
    ReturnStmt(Expr),
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
    EnableStmt(String), // enable <flag> or enable all
    DisableStmt(String), // disable <flag> or disable all

    // ── Control Flow ──────────────────────────────────────────
    // set name -> value;
    // set obj.field -> value;   (property chain reassignment)
    /// إعادة تعيين قيمة — `set <target> -> <value>;`
    /// target ممكن يكون identifier بسيط أو property chain (obj.field)
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
        /// else block — اختياري
        else_block: Option<Vec<Stmt>>,
    },

    /// `loop N -> { ... }`  أو  `loop -> { ... }` (infinite)
    /// أو  `loop N -> scope_name(args)` / `loop -> scope_name(args)` (scope call)
    LoopStmt {
        /// عدد التكرارات — None = infinite loop
        count: Option<Expr>,
        body: EitherBlock,
    },

    /// `while (cond) -> { ... }`
    /// أو  `while (cond) -> scope_name(args)` (scope call)
    WhileStmt {
        condition: Expr,
        body: EitherBlock,
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
}

/// جسم الـ loop/while — ممكن يكون:
///   Inline: block عادي `{ ... }`
///   ScopeCall: استدعاء scope من نوع looped/custom
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
pub struct EnumVariant {
    pub name: String,
    pub data_type: Option<BaseType>, // e.g. Success(int) -> vec!["int"]
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
    base_type: BaseType::Generic(vec![BaseType::Int]), // يحتوي على المعاملات (Parameters)
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
