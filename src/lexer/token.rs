#[derive(Debug, PartialEq, Clone)]

pub enum TokenKind {
    // 1. Data Types
    Int(i64),
    Float(f64),
    String(String),
    Char(char),
    Identifier(String),
    Bool(bool),
    // 2. Keywords
    Let,      // let
    Const,    // const
    Set,      // set
    If,       // if
    Else,     // else
    Switch, // switch // i don't know how to implement it but it will be using scope we will make a new scope type for it.
    Case,   // case
    For,    // for
    In,     // in
    While,  // while
    Loop,   // loop
    Break,  // break
    Continue, // continue
    Return, // return
    Fn,     // fn
    Del,    // del
    Extends, // extends
    Super,  // super

    Use,    // use
    Export, // export
    New,    // new
    Copy,   // copy
    Modify, // modify
    This,   // this
    Global, // global

    Try,     // try
    Catch,   // catch
    Throw,   // throw
    Enable,  // enable
    Disable, // disable
    All,     // all

    // comments
    MultiLineComment,
    InlineComment,

    // 3. Built-in Types

    // Primitives
    TypeInt,            // int
    TypeFloat,          // float
    TypeChar,           // char
    TypeBool,           // bool
    TypeVoid,           // void
    TypeType,           // type
    MadeUpType(String), // made up type

    //temp
    TypeError, //todo : remove this

    // built-in fn
    SizeOf,   // sizeof()
    TypeOf,   // typeof()
    ToString, // to_string()
    Log,      // log()

    // Context Types
    TypeScope,     // scope
    TypeLength,    // length   //* with  array and str
    TypeData,      // data     //* with  array and str
    TypeName,      // name
    TypeBluePrint, // blueprint  //* with objects
    //scopes types
    TypeObject, // object    //* with oop scopes and custom
    TypeCustom, // custom
    TypeStruct, // struct
    TypeBlock,  // block
    TypeClass,  // class
    TypeEnum,   // enum
    TypeStr,    // str
    TypeArray,  // array
    //scopes fields
    Param,     // param    //*  with  scope and fn and custom and init
    Init,      // init for getting constructor   //* with  oop scopes and custom
    Generic,   // generic    //* for custom generics
    Flag,      // flag      //* with  scope and fn and looped and block and custom
    Static,    // static    //* with  oop scopes and custom
    Public,    // public    //* with  class and struct and  custom and scope
    Private,   // private   //* with  class and struct and  custom and scope
    Event,     // event     //* with all meta-block: event.call -> { ... }
    Handle,    // handle    //* with all meta-block: handle.<flag> -> { ... }
    Statement, // statement //* with all meta-block: statement -> { ... }
    Variants,

    // for custom
    CustomIndexAccess, // index_access
    CustomConstructor, // constructor
    CustomKeyword,     // custom_keyword
    CustomGeneric,     // custom_generic
    CustomIterator,    // iterator
    CustomDisplay,     // display
    CustomOperators,   // operators
    // 4. Operators / punctuation
    Assign,   // =
    Arrow,    // ->
    FatArrow, // =>
    Dot,      // .
    Not,      // !
    Plus,     // +
    Minus,    // -
    Multiply, // *
    Divide,   // /

    PlusPlus,   // ++
    MinusMinus, // --
    DotDotDot,  // ...  //todo: add it
    Mod,        // %
    Underscore, // _

    // logical
    And, // && or and
    Or,  // || or or

    // Relational
    Eq,        // ==
    NotEq,     // !=
    Greater,   // >
    Less,      // <
    GreaterEq, // >=
    LessEq,    // <=

    Ampersand, // &

    // Symbols
    LParen,      // (
    RParen,      // )
    LBrace,      // {
    RBrace,      // }
    LBracket,    // [
    RBracket,    // ]
    Colon,       // :
    DoubleColon, // ::
    Comma,       // ,
    SemiColon,   // ;

    EOF,
    Default, // legacy catch-all, no longer emitted by the scanner (kept so nothing
    // downstream that matches on it breaks); prefer Error(String) instead.
    Error(String), // lexical error with a human-readable message; scanning continues
                   // afterward so the parser can still synchronize() and report more errors.
}
impl TokenKind {
    pub fn as_str(&self) -> &str {
        match self {
            TokenKind::Int(_) => "int",
            TokenKind::Float(_) => "float",
            TokenKind::String(_) => "str",
            TokenKind::Char(_) => "char",
            TokenKind::Identifier(v) => v,
            TokenKind::Bool(_) => "bool",
            TokenKind::Let => "let",
            TokenKind::Const => "const",
            TokenKind::Set => "set",
            TokenKind::Log => "log",
            TokenKind::If => "if",
            TokenKind::Else => "else",
            TokenKind::Switch => "switch",
            TokenKind::Loop => "loop",
            TokenKind::While => "while",
            TokenKind::Break => "break",
            TokenKind::Continue => "continue",
            TokenKind::Return => "return",
            TokenKind::Fn => "fn",
            TokenKind::Del => "del",
            TokenKind::Extends => "extends",
            TokenKind::Super => "super",
            TokenKind::Use => "use",
            TokenKind::Export => "export",
            TokenKind::New => "new",
            TokenKind::Copy => "copy",
            TokenKind::Modify => "modify",
            TokenKind::This => "this",
            TokenKind::Global => "global",
            TokenKind::Try => "try",
            TokenKind::Catch => "catch",
            TokenKind::Throw => "throw",
            TokenKind::Enable => "enable",
            TokenKind::Disable => "disable",
            TokenKind::All => "all",
            TokenKind::TypeClass => "class",
            TokenKind::TypeEnum => "enum",
            TokenKind::TypeError => "error",
            TokenKind::Event => "event",
            TokenKind::Handle => "handle",
            TokenKind::TypeName => "name",
            TokenKind::TypeCustom => "custom",
            TokenKind::Private => "private",
            TokenKind::Public => "public",
            TokenKind::Static => "static",
            TokenKind::TypeLength => "length",
            TokenKind::SizeOf => "sizeof",
            TokenKind::TypeData => "data",
            TokenKind::Statement => "statement",
            TokenKind::CustomConstructor => "constructor",
            TokenKind::CustomKeyword => "keyword",
            TokenKind::CustomGeneric => "generic",
            TokenKind::CustomIterator => "iterator",
            TokenKind::CustomDisplay => "display",
            TokenKind::CustomOperators => "operators",
            TokenKind::Param => "param",
            TokenKind::Init => "init",
            TokenKind::TypeBluePrint => "blueprint",
            TokenKind::Flag => "flag",
            TokenKind::TypeType => "type",
            TokenKind::Assign => "=",
            TokenKind::Arrow => "->",
            TokenKind::FatArrow => "=>",
            TokenKind::Dot => ".",
            TokenKind::Not => "!",
            TokenKind::Plus => "+",
            TokenKind::Minus => "-",
            TokenKind::Multiply => "*",
            TokenKind::Divide => "/",
            TokenKind::Mod => "%",
            TokenKind::LBrace => "{",
            TokenKind::RBrace => "}",
            TokenKind::LBracket => "[",
            TokenKind::RBracket => "]",
            TokenKind::LParen => "(",
            TokenKind::RParen => ")",
            TokenKind::Eq => "==",
            TokenKind::NotEq => "!=",
            TokenKind::Less => "<",
            TokenKind::Greater => ">",
            TokenKind::GreaterEq => ">=",
            TokenKind::LessEq => "<=",
            TokenKind::And => "and",
            TokenKind::Or => "or",
            TokenKind::Underscore => "_",
            TokenKind::Comma => ",",
            TokenKind::SemiColon => ";",
            TokenKind::Error(v) => v,
            _ => "error",
        }
    }
    /// Returns the source keyword string for type tokens.
    /// Mirrors `as_str` but covers the built-in type variants that the main
    /// `as_str` falls through to the `_ => "error"` arm.
    pub fn type_keyword(&self) -> Option<&str> {
        match self {
            TokenKind::TypeInt => Some("int"),
            TokenKind::TypeFloat => Some("float"),
            TokenKind::TypeType => Some("type"),
            TokenKind::TypeBool => Some("bool"),
            TokenKind::TypeChar => Some("char"),
            TokenKind::TypeVoid => Some("void"),
            TokenKind::TypeScope => Some("scope"),
            TokenKind::TypeName => Some("name"),
            _ => None,
        }
    }
}
/// A token plus the source position where it *starts*. Position is captured
/// before any of the token's characters are consumed, so it points at the
/// first character of the lexeme, not the character after it.
#[derive(Debug, PartialEq, Clone)]
pub struct Token {
    pub kind: TokenKind,
    pub line: usize,
    pub column: usize,
}

impl Token {
    pub fn new(kind: TokenKind, line: usize, column: usize) -> Self {
        Token { kind, line, column }
    }
}
