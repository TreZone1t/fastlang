use crate::frontend::lexer::token::TokenKind;
use crate::frontend::parser::ast::*;
use crate::frontend::parser::parser::Parser;

impl Parser {
    /// Parses: `@compile [Name] [->] { decl1; decl2; ... }`
    pub(crate) fn parse_compile_decl(&mut self) -> Result<Decl, String> {
        self.advance(); // consume '@compile'

        let mut name = "@compile".to_string();
        if let TokenKind::Identifier(id) = &self.peek().kind {
            name = id.clone();
            self.advance();
        }

        if self.peek().kind == TokenKind::DoubleColon {
            self.advance(); // consume '::'
        } else if self.peek().kind == TokenKind::Arrow {
            self.advance(); // consume '->'
        }

        self.consume(TokenKind::LBrace, "Expected '{' to open @compile body")?;

        let mut decls: Vec<Decl> = Vec::new();

        while !self.is_at_end() && self.peek().kind != TokenKind::RBrace {
            let start_pos = self.current;
            match self.parse_statement(ScopeType::Block)? {
                Some(stmt) => {
                    match stmt {
                        Stmt::Declaration(d) => decls.push(d),
                        Stmt::ThrowStmt(expr) => {
                            decls.push(Decl::DefineDecl {
                                visibility: Visibility::Private,
                                name: String::new(),
                                type_alias: None,
                                value: Some(expr),
                            });
                        }
                        _ => {}
                    }
                }
                None => {
                    if self.current == start_pos && !self.is_at_end() && self.peek().kind != TokenKind::RBrace {
                        self.advance();
                    }
                }
            }
        }

        self.consume(TokenKind::RBrace, "Expected '}' to close @compile body")?;

        if self.peek().kind == TokenKind::SemiColon {
            self.advance();
        }

        Ok(Decl::CompileDecl { name, decls })
    }
}
