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
    /// Custom scope keywords registered at parse-time (e.g. "array", "some").
    /// Populated when a scope with `keyword -> "...";` is parsed.
    pub(crate) custom_keywords: Vec<String>,
    /// Built-in scope type tokens that are "activated" when a scope declares
    /// `type -> array;` or `type -> str;`. Only these tokens are then valid
    /// as types in variable declarations (e.g. `array<int(32)> x -> ...;`).
    /// This avoids hardcoding std library names and keeps the system dynamic.
    pub(crate) registered_builtin_type_tokens: Vec<TokenKind>,
}

impl Parser {
    pub fn new(tokens: Vec<Token>) -> Self {
        Parser {
            tokens,
            current: 0,
            custom_keywords: Vec::new(),
            registered_builtin_type_tokens: Vec::new(),
        }
    }

    pub(crate) fn peek(&self) -> &Token {
        // NOTE: EOF placeholder for out-of-range access carries dummy position 0,0.
        // Should not happen in practice since is_at_end() gates the main loops.
        self.tokens.get(self.current).unwrap_or(&EOF_TOKEN)
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
            match self.parse_statement()? {
                Some(stmt) => statements.push(stmt),
                None => {}
            }
        }

        Ok(Program { statements })
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
