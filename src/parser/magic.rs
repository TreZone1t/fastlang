use crate::lexer::token::{Token, TokenKind};
use crate::parser::ast::*;
use crate::parser::parser::Parser;

impl Parser {
    // 1. الأنواع المسموح تكون Magic Types للمتغيرات (تم تنظيفها من الأشياء الملغية)
    pub(crate) fn is_magic_token(kind: &TokenKind) -> bool {
        match kind {
            TokenKind::TypeName
            | TokenKind::TypeBluePrint
         | TokenKind::TypeType // لو عندك Token خاص بكلمة type فعلّه هنا
          | TokenKind::TypeLength
          | TokenKind::TypeScope
          | TokenKind::TypeData => true,
            _ => false,
        }
    }

    // 2. أنواع الـ Scopes الهيكلية
    pub fn is_scope_type(kind: &TokenKind) -> bool {
        match kind {
            TokenKind::TypeScope
            | TokenKind::TypeCustom
            | TokenKind::TypeStruct
            | TokenKind::TypeClass
            | TokenKind::TypeEnum
            | TokenKind::TypeBlock => true,
            // تم حذف Statement بناءً على قرار التخلص منها
            _ => false,
        }
    }

    // 3. التحقق النصي من الـ Magic Types (تم إعدام القائمة الحمراء من هنا)
    pub(crate) fn is_magic_type_str(type_name: &str) -> bool {
        matches!(
            type_name,
            "name" | "blueprint" | "type" | "length" | "size" | "data"
        )
    }
    pub(crate) fn is_scope_type_str(type_name: &str) -> bool {
        matches!(
            type_name,
            "scope" | "custom" | "struct" | "class" | "enum" | "block"
        )
    }
    // 4. التحديث الأهم: تحويل الـ Cast القديم للـ Nodes الجديدة اللي صممناها
    pub(crate) fn parse_magic_cast(
        &mut self,
        magic_type: String,
        target: Expr,
    ) -> Result<Expr, String> {
        match magic_type.as_str() {
            "name" => Ok(Expr::MagicReference {
                target: Box::new(target),
                kind: ReferenceKind::Name,
                access_mode: AccessMode::ReadOnly, // الافتراضي ReadOnly
            }),
            "length" => Ok(Expr::MagicReference {
                target: Box::new(target),
                kind: ReferenceKind::Length,
                access_mode: AccessMode::ReadOnly,
            }),
            "size" => Ok(Expr::MagicReference {
                target: Box::new(target),
                kind: ReferenceKind::Size,
                access_mode: AccessMode::ReadOnly,
            }),
            "data" => Ok(Expr::MagicReference {
                target: Box::new(target),
                kind: ReferenceKind::Data,
                access_mode: AccessMode::ReadOnly,
            }),
            "type" => Ok(Expr::TypeOf {
                target: Box::new(target),
            }),
            _ => Err(format!(
                "Semantic Error: '{}' cannot be used in a magic cast. Only name, length, size, data, and type are allowed.",
                magic_type
            )),
        }
    }

    // 5. أنواع الحقول داخل الـ Scopes (زي public -> { ... })
    // احتفظنا بـ public و private هنا لأنهم بلوكات هيكلية وليسوا متغيرات
    pub fn is_scope_field_type(kind: &TokenKind) -> bool {
        match kind {
            TokenKind::Static
            | TokenKind::Public
            | TokenKind::Private
            | TokenKind::Event
            | TokenKind::Init
            | TokenKind::Keyword
            | TokenKind::Variants
            | TokenKind::Generic
            | TokenKind::Flag
            | TokenKind::TypeData
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
                TokenKind::Event => "event",
                TokenKind::TypeLength => "length",
                TokenKind::Init => "init",
                TokenKind::Generic => "generic",
                TokenKind::TypeData => "data",
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
