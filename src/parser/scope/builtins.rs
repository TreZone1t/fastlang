use std::collections::HashMap;

use crate::lexer::token::TokenKind;
use crate::parser::ast::*;
use crate::parser::parser::Parser;

impl Parser {
    pub(crate) fn parse_fn_decl(&mut self, name: String) -> Result<Stmt, String> {
        let mut settings: Vec<Setting> = Vec::new();
        //todo: handle methods for future updates
        let mut handles: Vec<HandleMethods> = Vec::new();
        let mut handle_block: Vec<Stmt> = Vec::new();

        let mut statement_block: Vec<Stmt> = Vec::new();
        let mut params: Vec<Param> = Vec::new();
        let mut return_type: TypeNode = TypeNode::Simple(TypeRef {
            base_type: BaseType::Void,
            size: None,
        });
        let mut name = name.clone();
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
        // لو جاي من scope dispatcher (name != "") → نحلل الـ statements مباشرةً
        // لو جاي مباشر (name == "") → الـ scope settings parsing (غير مستخدم حالياً)
        let normal = name.is_empty();

        if normal {
            // fn name
            println!("DEBUG: fn token: {:?}", self.peek().kind);
            self.advance(); // consume 'fn''
                            //debug
            print!("DEBUG: fn name: {:?}", self.peek().kind);
            name = self.get_identifier("Expected function name")?;
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
                        type_node: Some(type_node),
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
        }
        //now we are in the body like the scope fn
        if !normal {
            while !self.is_at_end() && self.peek().kind != TokenKind::RBrace {
                let t = self.peek().kind.clone();
                if (self.is_valid_setting(t.clone())) {
                    //====================================================================
                    // param -> { int a; int b; } ...
                    //====================================================================
                    if t == TokenKind::Param {
                        self.advance(); // 'param'
                        self.consume(TokenKind::Arrow, "Expected '->' after 'param'")?;
                        self.consume(TokenKind::LBrace, "Expected '{' to open param block")?;

                        while !self.is_at_end() && self.peek().kind != TokenKind::RBrace {
                            match self.parse_var_decl(false, false) {
                                Ok(Stmt::VarDecl {
                                    name, type_node, ..
                                }) => {
                                    params.push(Param {
                                        name: name.clone(),
                                        type_node: type_node.clone(),
                                    });
                                    fn_meta.params.push(Param { name, type_node });
                                }
                                Ok(_) => {
                                    return Err(
                                        "Syntax Error: Expected variable declaration".to_string()
                                    );
                                }
                                Err(e) => {
                                    eprintln!("Syntax Error in scope param block: {}", e);
                                    self.synchronize();
                                }
                            }
                        }
                        self.consume(TokenKind::RBrace, "Expected '}' to close param block")?;
                        if self.peek().kind == TokenKind::SemiColon {
                            self.advance();
                        }
                        continue;
                    }
                    //====================================================================
                    // return -> <type>;
                    //====================================================================
                    if t == TokenKind::Return {
                        self.advance(); // 'return'
                        self.consume(TokenKind::Arrow, "Expected '->' after 'return'")?;
                        return_type = self.parse_type()?;
                        self.consume(TokenKind::SemiColon, "Expected ';' after return type")?;
                        continue;
                    }
                    //====================================================================
                    // statement -> {  ... }
                    //====================================================================
                    if t == TokenKind::Statement {
                        self.advance(); // 'statement'
                        self.consume(TokenKind::Arrow, "Expected '->' after 'statement'")?;
                        self.consume(TokenKind::LBrace, "Expected '{' to open statement block")?;

                        while !self.is_at_end() && self.peek().kind != TokenKind::RBrace {
                            match self.parse_statement() {
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
                            continue;
                        }
                    }
                } else {
                    return Err(format!("Syntax Error: Invalid field found : {} , that is not allow in the array typed scope to use it \n\t - use custom typed scope with enable some setting it will work if it valid" , t.as_str()));
                }
            }
            self.consume(TokenKind::RBrace, "Expected '}' to close fn scope")?;
        } else {
            // we here only have a normal function
            while !self.is_at_end() && self.peek().kind != TokenKind::RBrace {
                match self.parse_statement() {
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
        Ok(Stmt::FnDecl {
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
    pub(crate) fn parse_block_decl(&mut self, name: String) -> Result<Stmt, String> {
        // block have statements only
        let mut statements: Vec<Stmt> = Vec::new();
        if name != "" {
            self.consume(TokenKind::Arrow, "Expected '->' to open block body")?;
            self.consume(TokenKind::LBrace, "Expected '{' to open block body")?;
        }
        while !self.is_at_end() && self.peek().kind != TokenKind::RBrace {
            match self.parse_statement() {
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
        return Ok(Stmt::BlockDecl {
            is_exported: false,
            name,
            statements,
        });
    }
}
