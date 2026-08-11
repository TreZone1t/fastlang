use crate::lexer::token::{Token, TokenKind};
use crate::parser::ast::*;
use crate::parser::parser::Parser;
//there is three ways to use it
//1. let <Magic> name = name; //name is the name of a scope so we will need a tool to know is the type came after let
// is realy a Magic Type so it will call parse_magic_cast
//2. <scope_type> <id> -> { ... }
//3. <scope_type> -> {
//name -> "";
//};
//4. <feild_type> <type> <id> -> <date> ;
//5. <feild_type> -> {...};

impl Parser {
    pub(crate) fn is_magic_token(kind: &TokenKind) -> bool {
        match kind {
            TokenKind::TypeName
            | TokenKind::TypeLength
            | TokenKind::TypeSize
            | TokenKind::TypeScope
            | TokenKind::TypeFlag
            | TokenKind::TypeParam
            | TokenKind::TypeBluePrint
            | TokenKind::TypeInit
            | TokenKind::TypeStatic
            | TokenKind::TypeArray
            | TokenKind::TypePublic
            | TokenKind::TypePrivate
            | TokenKind::TypeEvent
            | TokenKind::TypeHandle
            | TokenKind::TypeCustom
            | TokenKind::TypeStruct
            | TokenKind::TypeClass
            | TokenKind::TypeEnum
            | TokenKind::TypeStatement
            | TokenKind::TypeStr
            | TokenKind::TypeBlock
            | TokenKind::TypeObject => true,
            _ => false,
        }
    }
    //types we have  scope  or block or custom or
    //2. <scope_type> <name> -> { ... }
    pub fn is_scope_type(kind: &TokenKind) -> bool {
        match kind {
            TokenKind::TypeScope
            | TokenKind::TypeCustom
            | TokenKind::TypeStatement
            | TokenKind::TypeArray
            | TokenKind::TypeStruct
            | TokenKind::TypeClass
            | TokenKind::TypeEnum
            | TokenKind::TypeStr
            | TokenKind::TypeBlock => true,
            _ => false,
        }
    }

    pub(crate) fn is_magic_type_str(type_name: &str) -> bool {
        matches!(
            type_name,
            "name"
                | "scope"
                | "flag"
                | "length"
                | "size"
                | "param"
                | "init"
                | "blueprint"
                | "type"
                | "event"
                | "handle"
                | "statement"
                | "custom"
                | "struct"
                | "class"
                | "enum"
                | "string"
                | "block"
                | "object"
        )
    }

    pub(crate) fn parse_magic_cast(
        &mut self,
        magic_type: String,
        target: Expr,
    ) -> Result<Expr, String> {
        Ok(Expr::MagicCast {
            magic_type,
            target: Box::new(target),
        })
    }

    //1. let <Magic> name = <name> ;
    pub fn is_scope_feild_type(kind: &TokenKind) -> bool {
        match kind {
            TokenKind::TypeStatic
            | TokenKind::TypePublic
            | TokenKind::TypePrivate
            | TokenKind::TypeEvent
            | TokenKind::TypeLength
            | TokenKind::TypeInit
            | TokenKind::TypeName
            | TokenKind::TypeSize
            | TokenKind::TypeFlag
            | TokenKind::TypeParam
            | TokenKind::TypeHandle => true,
            _ => false,
        }
    }
    pub fn parse_scope_type(&mut self) -> Result<Expr, String> {
        let kind = self.peek().kind.clone();
        if Self::is_scope_type(&kind) {
            let name = match kind {
                TokenKind::TypeScope => "scope",
                TokenKind::TypeCustom => "custom",
                TokenKind::TypeStatement => "statement",
                TokenKind::TypeArray => "array",
                TokenKind::TypeStruct => "struct",
                TokenKind::TypeClass => "class",
                TokenKind::TypeEnum => "enum",
                TokenKind::TypeStr => "str",
                TokenKind::TypeBlock => "block",
                _ => unreachable!(),
            };
            self.advance();
            Ok(Expr::Identifier(name.to_string()))
        } else {
            Err(format!("Expected scope type, got {:?}", kind))
        }
    }
    pub fn parse_scope_feild_type(&mut self) -> Result<Expr, String> {
        let kind = self.peek().kind.clone();
        if Self::is_scope_feild_type(&kind) {
            let name = match kind {
                TokenKind::TypeStatic => "static",
                TokenKind::TypePublic => "public",
                TokenKind::TypePrivate => "private",
                TokenKind::TypeEvent => "event",
                TokenKind::TypeLength => "length",
                TokenKind::TypeInit => "init",
                TokenKind::TypeName => "name",
                TokenKind::TypeSize => "size",
                TokenKind::TypeFlag => "flag",
                TokenKind::TypeParam => "param",
                TokenKind::TypeHandle => "handle",
                _ => unreachable!(),
            };
            self.advance();
            // we now after the
            Ok(Expr::Identifier(name.to_string()))
        } else {
            Err(format!("Expected scope field type, got {:?}", kind))
        }
    }
}
