#[derive(Debug, Clone, PartialEq)]
pub enum Visibility {
    Public,
    Private,
    Static,
}

#[derive(Debug, Clone)]
pub struct Param {
    pub base_type: String,
    pub size: Option<i64>,
    pub name: String,
}

/// A type as it was written in source code.  Keeping the size separate avoids
/// losing information such as `int(16)` when a declaration is lowered to a
/// `ScopeDecl`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TypeRef {
    pub base_type: String,
    pub size: Option<i64>,
}

#[derive(Debug, Clone)]
pub enum Expr {
    LiteralInt(i64),
    LiteralFloat(f64),
    LiteralString(String),
    LiteralBool(bool),
    Identifier(String),
    Super,  // reference to parent class
    This,   // reference to current instance
    Global, // reference to global scope

    ListLiteral(Vec<Expr>),
    ObjectLiteral(Vec<Stmt>),

    Instantiate {
        op: String, // "new", "copy", "modify"
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
        operator: String, // "++" or "--"
    },
}

#[derive(Debug, Clone)]
pub enum Stmt {
    VarDecl {
        is_exported: bool,
        is_static: bool,
        is_const: bool,
        base_type: Option<String>,
        size: Option<i64>,
        name: String,
        value: Expr,
    },

    Reassign {
        name: String,
        value: Expr,
    },

    ScopeDecl {
        is_exported: bool,
        name: String,

        /// نوع الـ scope (block, Fn, looped, custom, ...)
        scope_type: String,

        /// true لو النوع custom — بيعني الـ semantic analyzer هيتخطى معظم القواعد
        is_custom: bool,

        /// الـ params — مسموح بهم بس في Fn و custom
        params: Vec<Stmt>,

        /// The declared return type for function-like scopes.  `None` means
        /// that the source did not declare one.
        return_type: Option<TypeRef>,

        /// الـ flags — control flow (isBreak, isReturn, etc.)
        flags: Vec<String>,

        /// الـ settings — scope features (index_access, public, length, size, etc.)
        settings: Vec<String>,

        /// event handlers: event.<name> -> { ... }
        events: Vec<EventDecl>,

        /// handle handlers: handle.<flag> -> { ... }
        handles: Vec<HandleDecl>,

        /// Block scope statements
        statements: Vec<Stmt>,

        /// Custom scope public block
        public_block: Vec<Stmt>,

        /// Fields declared with `add <type> <name>;` in a custom scope.
        /// They are kept separate from methods in access blocks so codegen can
        /// emit them as C++ data members.
        fields: Vec<Stmt>,

        /// Custom scope private block
        private_block: Vec<Stmt>,

        /// return value — مسموح بها بس في Fn و block و custom
        return_value: Option<Expr>,

        /// دالة البناء (constructor) — مسموح بها في הـ custom/scopes
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
        body: LoopBody,
    },

    /// `while (cond) -> { ... }`
    /// أو  `while (cond) -> scope_name(args)` (scope call)
    WhileStmt {
        condition: Expr,
        body: LoopBody,
    },

    /// `for (init; cond; inc) -> { ... }`
    ForStmt {
        init: Option<Box<Stmt>>,
        condition: Option<Expr>,
        increment: Option<Expr>,
        body: LoopBody,
    },

    Use {
        module_path: Vec<String>,
        imports: Option<Vec<String>>,
    },
}

/// جسم الـ loop/while — ممكن يكون:
///   Inline: block عادي `{ ... }`
///   ScopeCall: استدعاء scope من نوع looped/custom
///     e.g. `while (cond) -> my_looped_scope()`
#[derive(Debug, Clone)]
pub enum LoopBody {
    /// `{ statements... }` — inline block
    Inline(Vec<Stmt>),
    /// `scope_name(args)` — استدعاء scope
    ScopeCall(Expr),
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
