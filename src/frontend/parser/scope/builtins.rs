use std::collections::HashMap;

use crate::frontend::lexer::token::TokenKind;
use crate::frontend::parser::ast::*;
use crate::frontend::parser::parser::Parser;

impl Parser {
    pub(crate) fn parse_fn_decl(&mut self) -> Result<Decl, String> {
        let mut settings: Vec<Setting> = Vec::new();
        //todo: handle methods for future updates
        let mut handles: Vec<HandleMethods> = Vec::new();
        let mut handle_block: Vec<Stmt> = Vec::new();

        let mut statement_block: Vec<Stmt> = Vec::new();
        let mut params: Vec<Param> = Vec::new();
        let mut return_type: BaseType = BaseType::Void;
        self.advance(); // consume 'fn''
        let name = self.get_identifier("Expected function name")?;
        let mut fn_meta: FnType = FnType {
            name: name.clone(),
            params: Vec::new(),
            return_type: return_type.clone(),
        };
        //adding the default settings to the struct scope
        settings.push(Setting::Statement);
        settings.push(Setting::Return);
        settings.push(Setting::Param);
        settings.push(Setting::Handle);

        self.consume(TokenKind::LParen, "Expected '(' after function name")?;
        if self.peek().kind != TokenKind::RParen {
            // we expect a list of params
            // (a : int(32), b : int(32)) -> void
            loop {
                let param_name: String = self.get_identifier("Expected parameter name")?;
                self.consume(TokenKind::Colon, "Expected ':' after parameter name")?;
                let type_node = self.parse_type()?;
                params.push(Param {
                    name: param_name,
                    type_node: type_node,
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
        if !(self.peek().kind == TokenKind::LBrace) {
            return_type = self.parse_type()?;
        }
        self.consume(TokenKind::LBrace, "Expected '{' to open function body")?;
        while !self.is_at_end() && self.peek().kind != TokenKind::RBrace {
            match self.parse_statement("function".to_string()) {
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
        fn_meta.name = name.clone();
        fn_meta.params = params.clone();
        fn_meta.return_type = return_type.clone();
        self.fn_metadata.insert(name.clone(), fn_meta);
        Ok(Decl::FnDecl {
            is_exported: false,
            name,
            params,
            return_type,
            body: statement_block,
        })
    }

    pub(crate) fn parse_switch(&mut self, name: String) -> Result<Stmt, String> {
        Err(format!("Switch scope '{}' is not implemented yet", name))
    }
    pub(crate) fn parse_block_decl(&mut self) -> Result<Decl, String> {
        // block have statements only
        let mut statements: Vec<Stmt> = Vec::new();
        self.advance(); // consume 'block'
        let name = self.get_identifier("Expected block name")?;
        self.consume(TokenKind::Arrow, "Expected '->' to open block body")?;
        self.consume(TokenKind::LBrace, "Expected '{' to open block body")?;
        while !self.is_at_end() && self.peek().kind != TokenKind::RBrace {
            match self.parse_statement("block".to_string()) {
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
        Ok(Decl::BlockDecl {
            is_exported: false,
            name,
            statements,
        })
    }
}
