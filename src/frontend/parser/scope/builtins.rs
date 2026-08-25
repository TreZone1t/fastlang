
use crate::frontend::lexer::token::TokenKind;
use crate::frontend::parser::ast::*;
use crate::frontend::parser::parser::Parser;

impl Parser {
    pub(crate) fn parse_fn_decl(&mut self) -> Result<Decl, String> {
        let mut statement_block: Vec<Stmt> = Vec::new();
        let mut params: Vec<Param> = Vec::new();
        let mut return_type: BaseType = BaseType::Void;

        let mut is_virtual = false;
        let mut is_abstract = false;
        if self.peek().kind == TokenKind::Virtual {
            self.advance();
            is_virtual = true;
        } else if self.peek().kind == TokenKind::Abstract {
            self.advance();
            is_abstract = true;
        }

        self.consume(TokenKind::Fn, "Expected 'fn'")?;
        let name = self.get_identifier("Expected function name")?;
        let mut fn_meta: FnType = FnType {
            name: name.clone(),
            params: Vec::new(),
            return_type: return_type.clone(),
        };

        self.consume(TokenKind::LParen, "Expected '(' after function name")?;
        if self.peek().kind != TokenKind::RParen {
            loop {
                let param_name: String = self.get_identifier("Expected parameter name")?;
                self.consume(TokenKind::Colon, "Expected ':' after parameter name")?;
                let type_node = self.parse_type()?;
                params.push(Param {
                    name: param_name,
                    type_node,
                });
                if self.peek().kind == TokenKind::Comma {
                    self.advance();
                } else {
                    break;
                }
            }
        }
        self.consume(TokenKind::RParen, "Expected ')' after function parameters")?;
        self.consume(TokenKind::Arrow, "Expected '->' after function parameters")?;
        if !(self.peek().kind == TokenKind::LBrace || self.peek().kind == TokenKind::SemiColon) {
            return_type = self.parse_type()?;
        }

        if is_abstract {
            if self.peek().kind == TokenKind::SemiColon {
                self.advance();
            } else if self.peek().kind == TokenKind::LBrace {
                return Err("Syntax Error: Abstract function cannot have a body".to_string());
            }
        } else {
            self.consume(TokenKind::LBrace, "Expected '{' to open function body")?;
            while !self.is_at_end() && self.peek().kind != TokenKind::RBrace {
                match self.parse_statement(ScopeType::Fn) {
                    Ok(Some(stmt)) => statement_block.push(stmt),
                    Ok(None) => {
                        if !self.is_at_end() && self.peek().kind != TokenKind::RBrace {
                            self.advance();
                        }
                    }
                    Err(err) => return Err(err),
                }
            }
            self.consume(TokenKind::RBrace, "Expected '}' to close statement block")?;
            if self.peek().kind == TokenKind::SemiColon {
                self.advance(); // consume ';'
            }
        }

        fn_meta.name = name.clone();
        fn_meta.params = params.clone();
        fn_meta.return_type = return_type.clone();
        self.fn_metadata.insert(name.clone(), fn_meta);
        Ok(Decl::FnDecl {
            is_exported: false,
            is_virtual,
            is_abstract,
            name,
            params,
            return_type,
            body: statement_block,
        })
    }

    pub(crate) fn parse_block_scope_decl(&mut self) -> Result<Decl, String> {
        self.advance(); // consume 'block'
        let mut return_type = None;
        if self.peek().kind == TokenKind::Less {
            self.advance(); // consume '<'
            return_type = Some(self.parse_type()?);
            self.consume(TokenKind::Greater, "Expected '>' to close block return type")?;
        }

        let name = self.get_identifier("Expected block name")?;

        if self.peek().kind == TokenKind::Assign {
            self.advance(); // consume '='
        }

        if self.peek().kind == TokenKind::LParen {
            self.advance();
            self.consume(TokenKind::RParen, "Expected ')' after block parameters")?;
        }

        if self.peek().kind == TokenKind::Arrow {
            self.advance();
            if self.peek().kind != TokenKind::LBrace {
                return_type = Some(self.parse_type()?);
            }
        }

        self.consume(TokenKind::LBrace, "Expected '{' to open block body")?;
        let mut statements = Vec::new();
        while !self.is_at_end() && self.peek().kind != TokenKind::RBrace {
            match self.parse_statement(ScopeType::Block) {
                Ok(Some(stmt)) => statements.push(stmt),
                Ok(None) => {
                    if !self.is_at_end() && self.peek().kind != TokenKind::RBrace {
                        self.advance();
                    }
                }
                Err(err) => return Err(err),
            }
        }
        self.consume(TokenKind::RBrace, "Expected '}' to close block body")?;
        if self.peek().kind == TokenKind::SemiColon {
            self.advance();
        }
        Ok(Decl::BlockDecl {
            is_exported: false,
            name,
            return_type,
            statements,
        })
    }

    pub(crate) fn parse_micro_decl(&mut self) -> Result<Decl, String> {
        self.advance(); // consume 'micro'
        let name = self.get_identifier("Expected micro name")?;

        let mut generics = None;
        if self.peek().kind == TokenKind::Less {
            self.advance(); // consume '<'
            let mut g_list = Vec::new();
            while !self.is_at_end() && self.peek().kind != TokenKind::Greater {
                let g_name = self.get_identifier("Expected generic type parameter name")?;
                g_list.push(g_name);
                if self.peek().kind == TokenKind::Comma {
                    self.advance();
                } else {
                    break;
                }
            }
            self.consume(TokenKind::Greater, "Expected '>' to close micro generics")?;
            generics = Some(g_list);
        }

        if self.peek().kind == TokenKind::Assign {
            self.advance(); // consume '='
        }

        let mut params = Vec::new();
        if self.peek().kind == TokenKind::LParen {
            self.advance(); // consume '('
            if self.peek().kind != TokenKind::RParen {
                loop {
                    let p_name = self.get_identifier("Expected parameter name in micro")?;
                    self.consume(TokenKind::Colon, "Expected ':' after parameter name")?;
                    let p_type = self.parse_type()?;
                    params.push(Param {
                        name: p_name,
                        type_node: p_type,
                    });
                    if self.peek().kind == TokenKind::Comma {
                        self.advance();
                    } else {
                        break;
                    }
                }
            }
            self.consume(TokenKind::RParen, "Expected ')' after micro parameters")?;
        }

        let mut return_type = None;
        if self.peek().kind == TokenKind::Arrow {
            self.advance(); // consume '->'
            if self.peek().kind != TokenKind::LBrace {
                return_type = Some(self.parse_type()?);
            }
        }

        let body = if self.peek().kind == TokenKind::LBrace {
            self.advance(); // consume '{'
            let mut stmts = Vec::new();
            while !self.is_at_end() && self.peek().kind != TokenKind::RBrace {
                match self.parse_statement(ScopeType::Block) {
                    Ok(Some(stmt)) => stmts.push(stmt),
                    Ok(None) => {
                        if !self.is_at_end() && self.peek().kind != TokenKind::RBrace {
                            self.advance();
                        }
                    }
                    Err(err) => return Err(err),
                }
            }
            self.consume(TokenKind::RBrace, "Expected '}' to close micro body")?;
            stmts
        } else {
            match self.parse_statement(ScopeType::Block)? {
                Some(stmt) => vec![stmt],
                None => vec![],
            }
        };

        if self.peek().kind == TokenKind::SemiColon {
            self.advance();
        }

        Ok(Decl::MicroDecl {
            is_exported: false,
            name,
            generics,
            params,
            return_type,
            body,
        })
    }
}
