use crate::frontend::lexer::token::TokenKind;
use crate::frontend::parser::ast::*;
use crate::frontend::parser::parser::Parser;

impl Parser {
    /// Parses directive namespace blocks such as:
    /// `@compile :: { ... }` or `@compile Name :: { ... }`
    /// `@sys :: { ... }` or `@sys Name :: { ... }`
    pub(crate) fn parse_directive_decl(&mut self) -> Result<Decl, String> {
        let directive_name = match &self.peek().kind {
            TokenKind::LabelName(n) => n.clone(),
            _ => "@compile".to_string(),
        };
        self.advance(); // consume directive token ('@compile', '@sys', etc.)

        let mut name = directive_name.clone();
        if let TokenKind::Identifier(id) = &self.peek().kind {
            name = format!("{}::{}", directive_name, id);
            self.advance();
        }

        if self.peek().kind == TokenKind::DoubleColon {
            self.advance(); // consume '::'
        } else {
            return Err(format!(
                "Syntax Error: Expected '::' after '{}' (e.g. '{} :: {{ ... }}')",
                directive_name, directive_name
            ));
        }

        self.consume(
            TokenKind::LBrace,
            &format!("Expected '{{' to open {} body", directive_name),
        )?;

        let mut decls: Vec<Decl> = Vec::new();

        while !self.is_at_end() && self.peek().kind != TokenKind::RBrace {
            // Check for 'set <ident>(<params>) -> <ret_ty>;' (Built-in Intrinsic Signature)
            if self.peek().kind == TokenKind::Set {
                self.advance(); // consume 'set'
                let fn_name = self.get_identifier("Expected identifier after 'set'")?;
                let mut generics = Vec::new();
                if self.peek().kind == TokenKind::Less {
                    self.parse_generics(&mut generics)?;
                }
                self.consume(TokenKind::LParen, "Expected '(' after intrinsic name")?;
                let mut params = Vec::new();
                if self.peek().kind != TokenKind::RParen {
                    loop {
                        let p = self.parse_single_param()?;
                        params.push(p);
                        if self.peek().kind == TokenKind::Comma {
                            self.advance();
                            continue;
                        }
                        break;
                    }
                }
                self.consume(TokenKind::RParen, "Expected ')' after parameters")?;
                let return_type = if self.peek().kind == TokenKind::Arrow {
                    self.advance();
                    self.parse_type()?
                } else {
                    BaseType::Void
                };
                self.consume(
                    TokenKind::SemiColon,
                    "Expected ';' after intrinsic declaration",
                )?;

                decls.push(Decl::FnDecl {
                    visibility: Visibility::Public,
                    name: fn_name,
                    generics,
                    params,
                    return_type,
                    body: Vec::new(),
                    is_virtual: false,
                    is_abstract: false,
                });
                continue;
            }

            let start_pos = self.current;
            match self.parse_statement(ScopeType::Block)? {
                Some(stmt) => match stmt {
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
                },
                None => {
                    if self.current == start_pos
                        && !self.is_at_end()
                        && self.peek().kind != TokenKind::RBrace
                    {
                        self.advance();
                    }
                }
            }
        }

        self.consume(
            TokenKind::RBrace,
            &format!("Expected '}}' to close {} body", directive_name),
        )?;

        if self.peek().kind == TokenKind::SemiColon {
            self.advance();
        }

        Ok(Decl::CompileDecl { name, decls })
    }
}
