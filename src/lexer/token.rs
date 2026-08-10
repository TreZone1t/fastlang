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
    Log,      // log
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
    TypeInt,   // int
    TypeFloat, // float
    TypeStr,   // str
    TypeArray, // array
    TypeChar,  // char
    TypeBool,  // bool
    TypeVoid,  // void

    // Context Types
    TypeScope,     // scope
    TypeStruct,    // struct
    TypeString,    // string
    TypeBlock,     // block
    TypeError,     // error
    TypeLength,    // length   //* with  list and string  and str
    TypeSize,      // size     //* with  list and string  and str
    TypeParam,     // param    //*  with  scope and fn and custom and init
    TypeInit,      // init for getting constructor   //* with  class and struct and custom
    TypeBluePrint, // blueprint  //* with objects
    TypeGeneric,   // generic    //* for custom generics
    TypeObject,    // object    //* with class and struct and custom
    TypeFlag,      // flag      //* with  scope and fn and looped and block and custom
    TypeStatic,    // static    //* with  class and struct and custom
    TypePublic,    // public    //* with  class and struct and  custom and scope
    TypePrivate,   // private   //* with  class and struct and  custom and scope
    TypeType,      // type      //* with all type declaration
    TypeEvent,     // event     //* with all meta-block: event.call -> { ... }
    TypeHandle,    // handle    //* with all meta-block: handle.<flag> -> { ... }
    TypeName,      // name      //*with all
    TypeStatement, // statement //* with all meta-block: statement -> { ... }
    TypeCustom,    // custom

    TypeClass, // class
    TypeEnum,  // enum

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
