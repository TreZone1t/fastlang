use crate::lexer::token::{Token, TokenKind};
use crate::parser::ast::*;

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
}

impl Parser {
    pub fn new(tokens: Vec<Token>) -> Self {
        Parser { tokens, current: 0 }
    }

    pub(crate) fn peek(&self) -> &Token {
        // NOTE: EOF placeholder for out-of-range access carries dummy position 0,0.
        // Should not happen in practice since is_at_end() gates the main loops.
        self.tokens.get(self.current).unwrap_or(&EOF_TOKEN)
    }

    pub(crate) fn previous(&self) -> &Token {
        &self.tokens[self.current - 1]
    }

    pub(crate) fn is_at_end(&self) -> bool {
        self.peek().kind == TokenKind::EOF
    }

    pub(crate) fn advance(&mut self) -> &Token {
        if !self.is_at_end() {
            self.current += 1;
        }
        self.previous()
    }

    /// Keywords that can legally appear as identifiers in name positions
    /// (variable names, field names, parameter names).
    /// e.g. `let bool flag = ...` where 'flag' is a keyword we registered.
    pub(crate) fn keyword_as_identifier(kind: &TokenKind) -> Option<String> {
        match kind {
            // Context-type keywords that users can also use as names
            TokenKind::TypeFlag => Some("flag".to_string()),
            TokenKind::TypeLength => Some("length".to_string()),
            TokenKind::TypeSize => Some("size".to_string()),
            TokenKind::TypeParam => Some("param".to_string()),
            TokenKind::TypeType => Some("type".to_string()),
            TokenKind::TypeInit => Some("init".to_string()),
            TokenKind::TypeEvent => Some("event".to_string()),
            TokenKind::TypeHandle => Some("handle".to_string()),
            TokenKind::TypeName => Some("name".to_string()),
            TokenKind::TypeCustom => Some("custom".to_string()),
            TokenKind::TypePrivate => Some("private".to_string()),
            TokenKind::TypePublic => Some("public".to_string()),
            TokenKind::TypeError => Some("error".to_string()),
            TokenKind::TypeBlock => Some("block".to_string()),
            TokenKind::Fn => Some("fn".to_string()),
            TokenKind::TypeStruct => Some("struct".to_string()),
            TokenKind::Class => Some("class".to_string()),
            TokenKind::Enum => Some("enum".to_string()),
            TokenKind::Log => Some("log".to_string()),
            // booleans as identifiers
            TokenKind::Bool(b) => Some(if *b { "true" } else { "false" }.to_string()),
            _ => None,
        }
    }

    pub fn parse_program(&mut self) -> Result<Program, String> {
        let mut statements = Vec::new();

        while !self.is_at_end() {
            match self.parse_statement()? {
                Some(stmt) => statements.push(stmt),
                None => {}
            }
        }

        Ok(Program { statements })
    }

    pub(crate) fn consume(&mut self, expected: TokenKind, error_message: &str) -> Result<&Token, String> {
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
            if self.previous().kind == TokenKind::SemiColon {
                return;
            }

            match &self.peek().kind {
                TokenKind::Let
                | TokenKind::Set
                | TokenKind::If
                | TokenKind::Else
                | TokenKind::While
                | TokenKind::Loop
                | TokenKind::Break
                | TokenKind::Continue
                | TokenKind::Return
                | TokenKind::Fn
                | TokenKind::TypeScope
                | TokenKind::Class
                | TokenKind::Enum
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
