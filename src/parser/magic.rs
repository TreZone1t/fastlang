use crate::lexer::token::TokenKind;
use crate::parser::ast::*;
use crate::parser::parser::Parser;

impl Parser {
    pub fn is_scope_type(kind: &TokenKind) -> bool {
        match kind {
            TokenKind::TypeScope
            | TokenKind::TypeCustom
            | TokenKind::TypeStruct
            | TokenKind::TypeClass
            | TokenKind::TypeEnum
            | TokenKind::TypeBlock => true,
            _ => false,
        }
    }
    pub fn is_scope_field_type(kind: &TokenKind) -> bool {
        match kind {
            TokenKind::Static
            | TokenKind::Public
            | TokenKind::Private
            | TokenKind::Init
            | TokenKind::Variants
            | TokenKind::Generic
            | TokenKind::Flag
            | TokenKind::TypeData
            | TokenKind::Label
            | TokenKind::Statement
            | TokenKind::TypeLength
            | TokenKind::Handle => true,
            _ => false,
        }
    }

    pub fn parse_scope_type(&mut self) -> Result<Expr, String> {
        let kind = self.peek().kind.clone();
        if Self::is_scope_type(&kind) {
            let name = match kind {
                TokenKind::TypeScope => "scope",
                TokenKind::TypeCustom => "custom",
                TokenKind::TypeStruct => "struct",
                TokenKind::TypeClass => "class",
                TokenKind::TypeEnum => "enum",
                TokenKind::TypeBlock => "block",
                _ => unreachable!(),
            };
            self.advance();
            Ok(Expr::Identifier(name.to_string()))
        } else {
            Err(format!("Expected scope type, got {:?}", kind))
        }
    }

    pub fn parse_scope_field_type(&mut self) -> Result<Expr, String> {
        let kind = self.peek().kind.clone();
        if Self::is_scope_field_type(&kind) {
            let name = match kind {
                TokenKind::Static => "static",
                TokenKind::Public => "public",
                TokenKind::Private => "private",
                TokenKind::TypeLength => "length",
                TokenKind::Init => "init",
                TokenKind::Generic => "generic",
                TokenKind::TypeData => "data",
                TokenKind::Label => "label",
                TokenKind::Variants => "variants",
                TokenKind::Flag => "flag",
                TokenKind::Statement => "statement",
                TokenKind::Handle => "handle",
                _ => unreachable!(),
            };
            self.advance();
            Ok(Expr::Identifier(name.to_string()))
        } else {
            Err(format!("Expected scope field type, got {:?}", kind))
        }
    }
}
