use crate::lexer::token::TokenKind;
use crate::parser::ast::*;
use crate::parser::parser::Parser;

impl Parser {
    pub(crate) fn parse_struct_decl(&mut self, name: String) -> Result<Stmt, String> {
        let mut settings: Vec<crate::parser::ast::Setting> = Vec::new();
        let mut constructor: Option<crate::parser::ast::ConstructorDecl> = None;
        let mut handles: Vec<crate::parser::ast::HandleMethods> = Vec::new();
        let mut handle_block_ast: Vec<Stmt> = Vec::new();
        let mut public_block_ast: Vec<Stmt> = Vec::new();
        let mut private_block_ast: Vec<Stmt> = Vec::new();
        let mut static_block_ast: Vec<Stmt> = Vec::new();
        let mut name = name.clone();
        let mut keyword = name.clone();
        //adding the default settings to the struct scope
        settings.push(crate::parser::ast::Setting::CustomKeyword);
        settings.push(crate::parser::ast::Setting::Private);
        settings.push(crate::parser::ast::Setting::Public);
        settings.push(crate::parser::ast::Setting::Static);
        // adding allowed handles
        //we have display only
        handles.push(crate::parser::ast::HandleMethods::Display);
        if name.is_empty() {
            // جاي مباشر من parse_statement → لازم نستهلك 'struct' + الاسم + '{'
            self.advance(); // consume 'struct'
            name = self.get_identifier("Expected struct name")?;
            keyword = name.clone();
            // الـ struct syntax المباشر: `struct Node { ... }` بدون `->`
            if self.peek().kind == TokenKind::Arrow {
                self.advance(); // consume optional '->'
            }
            self.consume(TokenKind::LBrace, "Expected '{' to open struct body")?;
        }
        // now we are the same as the one being redirected by the scope parsing fn

        while !self.is_at_end() && self.peek().kind != TokenKind::RBrace {
            // we need to check if the token is valid for the setting
            let t = self.peek().kind.clone();
            //todo : check if the settings are in the settings vec
            if self.is_valid_setting(t.clone()) {
                // now need to know what is this section
                //====================================================================
                // constructor    _ () -> { ... }
                //====================================================================
                if t == TokenKind::Init {
                    match self.parse_constructor_decl() {
                        Ok(c) => constructor = Some(c),
                        Err(e) => {
                            eprintln!("Syntax Error in scope constructor: {}", e);
                            self.synchronize();
                        }
                    }
                    continue;
                }
                //====================================================================
                // handle -> { fn1 , fn2 , ... }
                //====================================================================
                if t == TokenKind::Handle {
                    self.advance(); // 'handle'
                    self.consume(TokenKind::Arrow, "Expected '->' after 'handle'")?;
                    self.consume(TokenKind::LBrace, "Expected '{' to open handle block")?;

                    while !self.is_at_end() && self.peek().kind != TokenKind::RBrace {
                        // first we need to check if the function is a valid handle function and there is no other function with the same name
                        // we need to check if it a fn in the first place no other thing is allowed
                        if self.peek().kind == TokenKind::Fn {
                            self.advance();
                            let mut method_params: Vec<Param> = Vec::new();
                            let return_type: TypeNode;
                            if self.is_valid_handle(handles.clone(), self.peek().kind.clone()) {
                                self.advance();
                                if self.peek().kind == TokenKind::LParen {
                                    self.advance();
                                    if self.peek().kind != TokenKind::RParen {
                                        // we expect a list of params
                                        // (a : int(32), b : int(32)) -> void
                                        loop {
                                            let name: String =
                                                self.get_identifier("Expected parameter name")?;
                                            self.consume(
                                                TokenKind::Colon,
                                                "Expected ':' after parameter name",
                                            )?;
                                            let type_node = self.parse_type()?;
                                            method_params.push(Param {
                                                name,
                                                type_node: Some(type_node),
                                            });
                                            if self.peek().kind == TokenKind::Comma {
                                                self.advance();
                                            } else {
                                                break;
                                            }
                                        }
                                    }
                                    self.consume(
                                        TokenKind::RParen,
                                        "Expected ')' after handle method parameters",
                                    )?;
                                    self.consume(
                                        TokenKind::Arrow,
                                        "Expected '->' after handle method parameters",
                                    )?;
                                    return_type = self.parse_type()?;
                                    let body = self.parse_block()?;
                                    self.consume(
                                        TokenKind::SemiColon,
                                        "Expected ';' after handle method body",
                                    )?;
                                    handle_block_ast.push(Stmt::FnDecl {
                                        is_exported: false,
                                        name: name.as_str().to_string(),
                                        params: method_params,
                                        return_type: return_type,
                                        body,
                                    });
                                }
                                self.consume(
                                    TokenKind::RBrace,
                                    "Expected '}' to close handle block",
                                )?;
                            }
                        } else {
                            return Err("Syntax Error: this name is not a valid allowed handle method in this scope type (array) at line {}, column {}".to_string());
                        }
                    }
                    self.consume(TokenKind::RBrace, "Expected '}' to close handle block")?;
                    if self.peek().kind == TokenKind::SemiColon {
                        self.advance();
                    }
                    continue;
                }
                //====================================================================
                // public -> { ... }
                //====================================================================
                if self.peek().kind == TokenKind::Public {
                    public_block_ast = self.parse_field_block()?;
                    continue;
                }
                //====================================================================
                // private -> { ... }
                //====================================================================
                if t == TokenKind::Private {
                    private_block_ast = self.parse_field_block()?;
                    continue;
                }
                //====================================================================
                // static -> { ... }
                //====================================================================
                if t == TokenKind::Static {
                    static_block_ast = self.parse_field_block()?;
                    continue;
                }
                //====================================================================
                // keyword -> <str>;
                //====================================================================
                if t == TokenKind::Keyword {
                    self.advance(); // 'keyword'
                    self.consume(TokenKind::Arrow, "Expected '->' after 'keyword'")?;
                    keyword = self.get_identifier("Expected keyword name")?;
                    self.custom_keywords.insert(keyword.clone(), name.clone());
                    self.consume(TokenKind::SemiColon, "Expected ';' after keyword name")?;
                    continue;
                }
            } else {
                print!("DEBUG: Invalid field found : {} , that is not allow in the array typed scope to use it \n\t - use custom typed scope with enable some setting it will work if it valid" , t.as_str());
                return Err(
                    ("Syntax Error: Invalid field  declaration at line {}, column {}").to_string(),
                );
            }
        }
        return Ok(Stmt::StructDecl {
            is_exported: false,
            name,
            keyword,
            handles,
            settings,
            public_block: public_block_ast,
            private_block: private_block_ast,
            handle_block: handle_block_ast,
            static_block: static_block_ast,
            constructor,
        });
    }
}
