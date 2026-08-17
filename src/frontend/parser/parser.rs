use std::collections::HashMap;

use crate::frontend::lexer::token::{Token, TokenKind};
use crate::frontend::parser::ast::*;

// Fallback token returned when `current` runs past the end of the stream.
// Explicit `const` instead of an inline `&Token { .. }` literal so we don't
// depend on rvalue static-promotion rules to make the borrow-check work.
const EOF_TOKEN: Token = Token {
    kind: TokenKind::EOF,
    line: 0,
    column: 0,
};

pub struct Parser {
    pub(crate) tokens: Vec<Token>,
    pub(crate) current: usize,
    pub(crate) metadata: HashMap<String, TypeMetadata>,
    pub(crate) var_metadata: HashMap<String, VarMetadata>,
    pub(crate) fn_metadata: HashMap<String, FnType>,
}

impl Parser {
    pub fn new(tokens: Vec<Token>) -> Self {
        Parser {
            tokens,
            current: 0,
            metadata: HashMap::new(),
            var_metadata: HashMap::new(),
            fn_metadata: HashMap::new(),
        }
    }

    pub(crate) fn peek(&self) -> &Token {
        // NOTE: EOF placeholder for out-of-range access carries dummy position 0,0.
        // Should not happen in practice since is_at_end() gates the main loops.
        self.tokens.get(self.current).unwrap_or(&EOF_TOKEN)
    }
    pub(crate) fn peek_at(&self, offset: usize) -> &Token {
        // NOTE: EOF placeholder for out-of-range access carries dummy position 0,0.
        // Should not happen in practice since is_at_end() gates the main loops.
        self.tokens.get(self.current + offset).unwrap_or(&EOF_TOKEN)
    }
    pub(crate) fn previous(&self, n: Option<usize>) -> &Token {
        let n = n.unwrap_or(1);
        &self.tokens[self.current - n]
    }

    pub(crate) fn is_at_end(&self) -> bool {
        self.peek().kind == TokenKind::EOF
    }

    pub(crate) fn advance(&mut self) -> &Token {
        if !self.is_at_end() {
            self.current += 1;
        }
        self.previous(None)
    }

    /// Keywords that can legally appear as identifiers in name positions
    /// (variable names, field names, parameter names).
    /// e.g. `let bool flag = ...` where 'flag' is a keyword we registered.

    pub fn parse_program(&mut self) -> Result<Program, String> {
        let mut statements = Vec::new();

        while !self.is_at_end() {
            match self.parse_statement("global".to_string()) {
                Ok(Some(stmt)) => statements.push(stmt),

                // إضافة الاحتمال الناقص لتجاهل الجمل الفارغة
                Ok(None) => continue,

                Err(e) => {
                    // =============== إضافة الـ Debug المؤقتة ===============
                    println!("\n⚠️ ⚠️ ⚠️ ERROR OCCURRED! PRINTING AST BUILT SO FAR ⚠️ ⚠️ ⚠️");
                    println!("{:#?}", statements);
                    println!("=========================================================\n");

                    return Err(e);
                }
            }
        }
        Ok(Program {
            statements: statements,
        })
    }

    pub(crate) fn consume(
        &mut self,
        expected: TokenKind,
        error_message: &str,
    ) -> Result<&Token, String> {
        if core::mem::discriminant(&self.peek().kind) == core::mem::discriminant(&expected) {
            Ok(self.advance())
        } else {
            // هنا هنضيف مستقبلاً اللوجيك اللي بيشاور على السطر وبيطبع الـ Hints (زي IF و print)
            // NOTE: self.peek().line / .column are now available for exactly this.
            Err(format!(
                "{} (at line {}, column {})",
                error_message,
                self.peek().line,
                self.peek().column
            ))
        }
    }

    // الدالة دي بترجع البارسر لوعيه بعد ما يلاقي غلطة عشان الكومبايلر ميكراشش
    pub(crate) fn synchronize(&mut self) {
        self.advance();

        while !self.is_at_end() {
            if self.previous(None).kind == TokenKind::SemiColon {
                return;
            }

            match &self.peek().kind {
                TokenKind::Set
                | TokenKind::If
                | TokenKind::Else
                | TokenKind::While
                | TokenKind::Loop
                | TokenKind::Break
                | TokenKind::Continue
                | TokenKind::Return
                | TokenKind::Fn
                | TokenKind::TypeClass
                | TokenKind::TypeEnum
                | TokenKind::TypeStruct => {
                    return;
                }
                _ => {
                    self.advance();
                }
            }
        }
    }

    // ----------------------------------------------------
    // تحليل الجمل (Statement Parsing)
    // ----------------------------------------------------

    // الدالة دي بتحدد إحنا هنقرأ أي نوع من الأوامر
}
