use std::collections::HashMap;

use crate::lexer::token::TokenKind;

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
    //todo : use it instead of string
    Int,
    Float,
    Char,
    Bool,
    Name(Box<BaseType>), //it will contain other type inside it
    Type(Box<BaseType>), //it will contain other type inside it
    //Str,
    Array(String),  //will contain the type of the array elements
    Object(String), //will contain the name of the Struct or Class or the Blueprint it came from
    //Block,
    Custom(String), //will contain the name of the Custom Scope it came from
    Unknown,        //it is the state when we don't know the type yet
    Error,
    Void,
    New(String), //it is will be used when we are creating a new type
}
impl BaseType {
    pub fn as_str(&self) -> String {
        match self {
            BaseType::Int => "int".to_string(),
            BaseType::Float => "float".to_string(),
            BaseType::Char => "char".to_string(),
            BaseType::Bool => "bool".to_string(),
            BaseType::Name(t) => format!("name<{}>", t.as_str()),
            BaseType::Type(t) => format!("type<{}>", t.as_str()),
            //Str,
            BaseType::Array(t) => format!("array<{}>", t),
            BaseType::Object(t) => format!("object<{}>", t),
            //Block,
            BaseType::Custom(t) => format!("custom<{}>", t),
            BaseType::Unknown => "unknown".to_string(),
            BaseType::Error => "error".to_string(),
            BaseType::Void => "void".to_string(),
            BaseType::New(t) => format!("new<{}>", t),
        }
    }
    pub fn from_str(s: &str) -> Self {
        match s {
            "int" => BaseType::Int,
            "float" => BaseType::Float,
            "char" => BaseType::Char,
            "bool" => BaseType::Bool,
            "name" => BaseType::Name(Box::new(BaseType::Unknown)),
            "type" => BaseType::Type(Box::new(BaseType::Unknown)),
            //"str" => BaseType::Str,
            "array" => BaseType::Array("".to_string()),
            "object" => BaseType::Object("".to_string()),
            //"block" => BaseType::Block,
            "custom" => BaseType::Custom("".to_string()),
            "unknown" => BaseType::Unknown,
            "error" => BaseType::Error,
            "void" => BaseType::Void,
            "new" => BaseType::New("".to_string()),
            _ => BaseType::Unknown,
        }
    }
}
#[derive(Debug, Clone, PartialEq)]
pub struct TypeRef {
    pub base_type: BaseType,
    pub size: Option<i64>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TypeNode {
    Simple(TypeRef),
    Generic(Generic),
}
#[derive(Debug, Clone, PartialEq)]
pub struct FnType {
    pub name: String,
    pub params: Vec<Param>,
    pub return_type: TypeNode,
}
#[derive(Debug, Clone, PartialEq)]
pub struct ConstructorType {
    pub name: String,
    pub params: Vec<Param>,
}
#[derive(Debug, Clone, PartialEq)]
pub struct VarMetadata {
    pub name: String,
    pub type_node: TypeNode,
    pub visibility: Visibility,
    pub editability: Editability,
    pub is_array: bool,
}
#[derive(Debug, Clone, PartialEq)]
pub struct TypeMetadata {
    pub name: String,                      // "Node"
    pub fields: HashMap<String, TypeNode>, // {"data": Int, "next": UserType("Node")}
    pub constructor: Option<Vec<ConstructorType>>,
    pub params: Vec<Param>,               // {"value": Int}
    pub generics: Vec<TypeNode>,          // {"T": UserType("Type")}
    pub methods: HashMap<String, FnType>, // {"set_next": Node.set_next -> void}
    pub handles: Vec<HandleMethods>,
    pub vars: HashMap<String, VarMetadata>,
    pub is_enum: bool,
    pub variants: Option<Vec<EnumVariant>>,
}
#[derive(Debug, Clone, PartialEq)]
pub struct Generic {
    pub base_type: BaseType,
    pub generics: Vec<TypeNode>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FieldDecl {
    pub visibility: Visibility,
    pub editability: Editability,
    pub type_node: Option<TypeNode>,
    pub name: String,
    pub value: Option<Expr>,
}
#[derive(Debug, Clone, PartialEq)]
pub enum AccessMode {
    ReadOnly,  // default (e.g., let name x = y;)
    ReadWrite, // when using modify (e.g., let name x = modify y;)
}

#[derive(Debug, Clone, PartialEq)]
pub enum ReferenceKind {
    Name,
    Length,
    Size,
    Data,
}
#[derive(Debug, Clone, PartialEq)]
pub enum ScopeType {
    Fn,
    Block,
    Class,
    Struct,
    Custom,
    //todo : add looped  so we will add setting for that instead
    // Case, //todo : the same
    Switch,
    Enum,
}
#[derive(Debug, Clone, PartialEq)]
pub struct Param {
    pub name: String,
    pub type_node: Option<TypeNode>,
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
    All,      // all of the settings
    OOP,      // private , public , static , extends , constructor
    Function, // param , statement , return
    Debug,    // error , break , throw , exit , return
    State,    // leave , yield , goto , label , call
    Custom, // custom_index_access , custom_constructor , custom_generic , custom_iterator , custom_display , custom_operators
    //custom
    CustomIndexAccess,
    CustomConstructor,

    CustomGeneric,
    CustomIterator,
    CustomDisplay,
    CustomOperators,
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
    Leave,
    Yield,
    Goto,
    Label,
    Call,
    // array and str
    Length,
    Data,
    Size,
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
            "custom" => Setting::Custom,
            //custom -------
            "index_access" | "custom_index_access" => Setting::CustomIndexAccess,
            "constructor" | "custom_constructor" => Setting::CustomConstructor,
            "generic" | "custom_generic" => Setting::CustomGeneric,
            "iterator" | "custom_iterator" => Setting::CustomIterator,
            "display" | "custom_display" => Setting::CustomDisplay,
            "operators" | "custom_operators" => Setting::CustomOperators,
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
            "leave" => Setting::Leave,
            "yield" => Setting::Yield,
            "goto" => Setting::Goto,
            "label" => Setting::Label,
            "call" => Setting::Call,
            //enum --
            "variants" => Setting::Variants,
            //array and str -------
            "length" => Setting::Length,
            "size" => Setting::Size,
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
            Setting::Custom => "custom".to_string(),
            //custom -----
            Setting::CustomIndexAccess => "custom_index_access".to_string(),
            Setting::CustomConstructor => "custom_constructor".to_string(),
            Setting::CustomGeneric => "custom_generic".to_string(),
            Setting::CustomIterator => "custom_iterator".to_string(),
            Setting::CustomDisplay => "custom_display".to_string(),
            Setting::CustomOperators => "custom_operators".to_string(),
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
            Setting::Goto => "goto".to_string(),
            Setting::Label => "label".to_string(),
            Setting::Call => "call".to_string(),
            //enum -------
            Setting::Variants => "variants".to_string(),
            //array and str -------
            Setting::Length => "length".to_string(),
            Setting::Size => "size".to_string(),
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
    IndexAccess,
    Display,
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    Iterator,
    Next,
    Length,
    Size,
    Call,
    Label,
    Goto,
    Leave,
    Yield,
    Data,
    Break,
    Error,
    Exit,
    Drop, //todo:  make it essentially the same as exit and the user has to implement it specifically in custom scope
    NotFound,
}
impl HandleMethods {
    pub fn from_str(s: &str) -> Self {
        match s {
            "index_access" => HandleMethods::IndexAccess,
            "display" => HandleMethods::Display,
            "add" => HandleMethods::Add,
            "sub" => HandleMethods::Sub,
            "mul" => HandleMethods::Mul,
            "div" => HandleMethods::Div,
            "mod" => HandleMethods::Mod,
            "iterator" => HandleMethods::Iterator,
            "next" => HandleMethods::Next,
            "length" => HandleMethods::Length,
            "size" => HandleMethods::Size,
            "call" => HandleMethods::Call,
            "label" => HandleMethods::Label,
            "goto" => HandleMethods::Goto,
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
}
#[derive(Debug, Clone, PartialEq)]
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

    ObjectLiteral(Vec<Stmt>),

    Instantiate {
        target: Box<Expr>,
        args: Vec<Expr>,
    },
    Modify {
        target: Box<Expr>,
    },
    Copy {
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
    MagicReference {
        target: Box<Expr>,
        kind: ReferenceKind,     // (Name, Length, Size, Data)
        access_mode: AccessMode, // (ReadOnly, ReadWrite)
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
#[derive(Debug, Clone, PartialEq)]
pub enum Stmt {
    VarDecl {
        visibility: Visibility,
        editability: Editability,
        type_node: Option<TypeNode>,
        name: String,
        value: Expr,
    },
    ArrayDecl {
        visibility: Visibility,
        editability: Editability,
        type_node: Option<TypeNode>,
        name: String,
        length: Expr,
        value: Expr,
    },
    Reassign {
        name: String,
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
        events: Option<Vec<EventDecl>>,
        fields: Option<Vec<FieldDecl>>,
        length: i64,
        data: Option<Expr>,
        extends: String,
        return_type: Option<TypeNode>,
        public_block: Option<Vec<Stmt>>,
        private_block: Option<Vec<Stmt>>,
        static_block: Option<Vec<Stmt>>,
        statements: Option<Vec<Stmt>>,
        variant_block: Option<Vec<EnumVariant>>,
        generics: Option<Vec<TypeNode>>,
        handle_block: Option<Vec<Stmt>>,
        constructor: Option<Vec<ConstructorDecl>>,
    },

    ClassDecl {
        is_exported: bool,
        name: String,
        extends: Option<String>,
        handles: Vec<HandleMethods>,
        settings: Vec<Setting>,
        public_block: Vec<Stmt>,
        private_block: Vec<Stmt>,
        static_block: Vec<Stmt>,
        generics: Option<Vec<TypeNode>>,
        handle_block: Vec<Stmt>,
        length: i64,
        constructor: Option<Vec<ConstructorDecl>>,
    },
    BlueprintDecl {
        is_exported: bool,
        name: String,
        definition: BlueprintDef,
    },
    ImplDecl {
        target: String,
        methods: Vec<Stmt>,
    },

    StructDecl {
        is_exported: bool,
        name: String,
        handles: Vec<HandleMethods>,
        settings: Vec<Setting>,
        public_block: Vec<Stmt>,
        private_block: Vec<Stmt>,
        handle_block: Vec<Stmt>,
        static_block: Vec<Stmt>,
        constructor: Option<Vec<ConstructorDecl>>,
    },

    EnumDecl {
        is_exported: bool,
        name: String,
        handles: Vec<HandleMethods>,
        settings: Vec<Setting>,
        handle_block: Vec<Stmt>,
        length: i64,
        variants: Vec<EnumVariant>,
    },
    FnDecl {
        is_exported: bool,
        name: String,
        params: Vec<Param>,
        return_type: TypeNode,
        body: Vec<Stmt>,
    },
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
    AddPropertyStmt {
        kind_name: String, // "label" or "flag"
        value: Expr,
    },
    LabelDecl {
        name: String,
        body: Vec<Stmt>,
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

    Import {
        module_path: Vec<String>,
        imports: Option<Vec<String>>,
    },

    DelStmt(Expr),
}

/// جسم الـ loop/while — ممكن يكون:
///   Inline: block عادي `{ ... }`
///   ScopeCall: استدعاء scope من نوع looped/custom
///     e.g. `while (cond) -> my_looped_scope()`
#[derive(Debug, Clone, PartialEq)]
pub enum EitherBlock {
    /// `{ statements... }` — inline block
    Inline(Vec<Stmt>),
    /// `scope_name(args)` — استدعاء scope
    External(Expr),
}

#[derive(Debug, Clone, PartialEq)]
pub struct ConstructorDecl {
    pub params: Vec<Param>,
    pub expected_types: Vec<String>,
    pub body: Vec<Stmt>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct EventDecl {
    pub trigger_name: String,
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
    pub data_types: Option<Vec<TypeNode>>, // e.g. Success(int) -> vec!["int"]
}
// 1. طرق تعريف الـ Blueprint (الـ 3 سيناريوهات اللي صممناها سوا)
#[derive(Debug, Clone, PartialEq)]
pub enum BlueprintDef {
    Explicit(Vec<BlueprintField>), // طريقة البلوك الصريح: { int(32) x; }
    FromExistingObject(String),    // طريقة النسخ: blueprint P = existing_obj;
    FromTemporaryObject(Vec<ObjectField>), // طريقة الاستنتاج: blueprint P = {x: 3, y: 6};
}

#[derive(Debug, Clone, PartialEq)]
pub struct BlueprintField {
    pub name: String,
    pub type_node: TypeNode,
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
   keyword "my_list";
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

 * i think that is all of we have for now you can enhance it and add more
*/
