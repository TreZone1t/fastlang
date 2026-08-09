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
pub struct TypeRef {
    pub base_type: String,
    pub size: Option<i64>,
}

#[derive(Debug, Clone)]
pub struct FieldDecl {
    pub visibility: Visibility,
    pub editability: Editability,
    pub type_sized: Option<TypeRef>,
    pub name: String,
    pub value: Option<Expr>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ScopeType {
    Fn,
    Block,
    Class,
    Struct,
    Custom,
    Looped,
    Case,
    Array,
    String,
}

#[derive(Debug, Clone)]
pub struct Param {
    pub name: String,
    pub base_type: String,
    pub size: Option<i64>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Flag {
    IsReturn,
    IsBreak,
    IsThrow,
    IsSwitch,
    IsExit,
    Custom(String),
}

impl Flag {
    pub fn from_str(s: &str) -> Self {
        match s {
            "is_return" => Flag::IsReturn,
            "is_break" => Flag::IsBreak,
            "is_throw" => Flag::IsThrow,
            "is_switch" => Flag::IsSwitch,
            "is_exit" => Flag::IsExit,
            _ => Flag::Custom(s.to_string()),
        }
    }

    pub fn as_str(&self) -> String {
        match self {
            Flag::IsReturn => "is_return".to_string(),
            Flag::IsBreak => "is_break".to_string(),
            Flag::IsThrow => "is_throw".to_string(),
            Flag::IsSwitch => "is_switch".to_string(),
            Flag::IsExit => "is_exit".to_string(),
            Flag::Custom(s) => s.clone(),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Setting {
    IndexAccess,
    CustomInitBody,
    Keyword,
    CustomParamBody,
    Param,
    Private,
    Public,
    Static,
    Length,
    Size,
    Data,
    Error,
    Custom(String),
}

impl Setting {
    pub fn from_str(s: &str) -> Self {
        match s {
            "index_access" => Setting::IndexAccess,
            "init" => Setting::CustomInitBody,
            "keyword" => Setting::Keyword,
            "param" => Setting::Param,
            "private" => Setting::Private,
            "public" => Setting::Public,
            "static" => Setting::Static,
            "length" => Setting::Length,
            "size" => Setting::Size,
            "data" => Setting::Data,
            "error" => Setting::Error,
            _ => Setting::Custom(s.to_string()),
        }
    }

    pub fn as_str(&self) -> String {
        match self {
            Setting::IndexAccess => "index_access".to_string(),
            Setting::CustomInitBody => "init".to_string(),
            Setting::Keyword => "keyword".to_string(),
            Setting::CustomParamBody => "param".to_string(),
            Setting::Param => "param".to_string(),
            Setting::Private => "private".to_string(),
            Setting::Public => "public".to_string(),
            Setting::Static => "static".to_string(),
            Setting::Length => "length".to_string(),
            Setting::Size => "size".to_string(),
            Setting::Data => "data".to_string(),
            Setting::Error => "error".to_string(),
            Setting::Custom(s) => s.clone(),
        }
    }
}

#[derive(Debug, Clone)]
pub enum Expr {
    LiteralInt(i64),
    LiteralFloat(f64),
    LiteralString(String),
    LiteralBool(bool),
    LiteralChar(char),
    ArrayLiteral(Vec<Expr>),
    Identifier(String),
    Super,
    This,
    Global,

    ListLiteral(Vec<Expr>),
    ObjectLiteral(Vec<Stmt>),

    Instantiate {
        op: String,
        target: Box<Expr>,
        args: Vec<Expr>,
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

    PostfixUpdate {
        left: Box<Expr>,
        operator: String,
    },
}

#[derive(Debug, Clone)]
pub enum Stmt {
    VarDecl {
        visibility: Visibility,
        editability: Editability,
        type_sized: Option<TypeRef>,
        name: String,
        value: Expr,
    },

    Reassign {
        name: String,
        value: Expr,
    },

    ScopeDecl {
        is_exported: bool,
        is_const: bool,
        name: String,
        scope_type: ScopeType,
        params: Vec<Param>,
        return_type: Option<TypeRef>,
        flags: Vec<Flag>,
        settings: Vec<Setting>,
        events: Vec<EventDecl>,
        handles: Vec<HandleDecl>,
        statements: Vec<Stmt>,
        public_block: Vec<Stmt>,
        fields: Vec<FieldDecl>,
        private_block: Vec<Stmt>,
        return_value: Option<Expr>,
        constructor: Option<ConstructorDecl>,
    },

    ClassDecl {
        is_exported: bool,
        name: String,
        extends: Option<String>,
        public_block: Vec<Stmt>,
        private_block: Vec<Stmt>,
        static_block: Vec<Stmt>,
        constructor: Option<ConstructorDecl>,
    },

    StructDecl {
        is_exported: bool,
        name: String,
        public_block: Vec<Stmt>,
        private_block: Vec<Stmt>,
        static_block: Vec<Stmt>,
        constructor: Option<ConstructorDecl>,
    },

    EnumDecl {
        is_exported: bool,
        name: String,
        variants: Vec<EnumVariant>,
    },

    FnDecl {
        is_exported: bool,
        name: String,
        params: Vec<Param>,
        return_type: String,
        body: Vec<Stmt>,
    },

    ReturnStmt(Expr),
    BreakStmt,
    ContinueStmt,
    ExpressionStmt(Expr),

    ThrowStmt(Expr),
    TryCatchStmt {
        try_block: Vec<Stmt>,
        catch_param: String,
        catch_block: Vec<Stmt>,
    },
    EnableStmt(String),  // enable <flag> or enable all
    DisableStmt(String), // disable <flag> or disable all

    // ── Control Flow ──────────────────────────────────────────
    // set name -> value;
    // set obj.field -> value;   (property chain reassignment)
    /// إعادة تعيين قيمة — `set <target> -> <value>;`
    /// target ممكن يكون identifier بسيط أو property chain (obj.field)
    ReassignStmt {
        target: Expr,
        value: Expr,
    },

    /// `if (cond) { ... } else { ... }`
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
        increment: Option<Expr>,
        body: EitherBlock,
    },

    Use {
        module_path: Vec<String>,
        imports: Option<Vec<String>>,
    },

    SwitchStmt {
        condition: Expr,
        cases: EitherBlock,
    },

    DelStmt(Expr),
}

/// جسم الـ loop/while — ممكن يكون:
///   Inline: block عادي `{ ... }`
///   ScopeCall: استدعاء scope من نوع looped/custom
///     e.g. `while (cond) -> my_looped_scope()`
#[derive(Debug, Clone)]
pub enum EitherBlock {
    /// `{ statements... }` — inline block
    Inline(Vec<Stmt>),
    /// `scope_name(args)` — استدعاء scope
    External(Expr),
}

#[derive(Debug, Clone)]
pub struct ConstructorDecl {
    pub params: Vec<Param>,
    pub expected_types: Vec<String>,
    pub body: Vec<Stmt>,
}

#[derive(Debug, Clone)]
pub struct EventDecl {
    pub trigger_name: String,
    pub body: Vec<Stmt>,
}

#[derive(Debug, Clone)]
pub struct HandleDecl {
    pub target_flag: String,
    pub body: Vec<Stmt>,
}

#[derive(Debug, Clone)]
pub struct EnumVariant {
    pub name: String,
    pub data_types: Vec<String>, // e.g. Success(int) -> vec!["int"]
}

#[derive(Debug, Clone)]
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
settings : {index_access, custom_init_body ,keyword, custom_param_body,param,private,public,static,length,size,data,error...},
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
