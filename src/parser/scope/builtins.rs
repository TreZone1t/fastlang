use crate::lexer::token::{Token, TokenKind};
use crate::parser::ast::*;
use crate::parser::parser::Parser;

impl Parser {
    pub(crate) fn parse_array_decl(&mut self, name: String) -> Result<Stmt, String> {
        // we are in the array scope already so we don't need to consme any thing
        let mut settings: Vec<crate::parser::ast::Setting> = Vec::new();
        let mut constructor: Option<crate::parser::ast::ConstructorDecl> = None;
        let mut handles: Vec<crate::parser::ast::HandleMethods> = Vec::new();
        let mut handle_block: Vec<Stmt> = Vec::new();
        let mut generic_block: Vec<String> = Vec::new();
        let mut public_block_ast: Vec<Stmt> = Vec::new();

        let mut private_block_ast: Vec<Stmt> = Vec::new();
        let mut length: i64 = 0;
        let mut data = String::new(); // will be a ptr for a list of data
        let mut keyword = name.clone();
        //adding the default settings to the array c
        settings.push(crate::parser::ast::Setting::CustomIndexAccess);
        settings.push(crate::parser::ast::Setting::CustomConstructor);
        settings.push(crate::parser::ast::Setting::CustomGeneric);
        settings.push(crate::parser::ast::Setting::CustomIterator);
        settings.push(crate::parser::ast::Setting::Private);
        settings.push(crate::parser::ast::Setting::Public);
        settings.push(crate::parser::ast::Setting::Length);
        settings.push(crate::parser::ast::Setting::Data);
        settings.push(crate::parser::ast::Setting::Size);
        // adding allowed handles
        //we have display , iterator , next , length , size
        handles.push(crate::parser::ast::HandleMethods::IndexAccess);
        handles.push(crate::parser::ast::HandleMethods::Display);
        handles.push(crate::parser::ast::HandleMethods::Iterator);
        handles.push(crate::parser::ast::HandleMethods::Next);
        handles.push(crate::parser::ast::HandleMethods::Length);
        handles.push(crate::parser::ast::HandleMethods::Size);
        // we now had our settings and handles we need to parse the body
        while !self.is_at_end() && self.peek().kind != TokenKind::RBrace {
            // we need to check if the token is valid for the setting
            let t = self.peek().kind.clone();
            let con = self.is_valid_setting(t.clone());
            if con {
                // now need to know what is this section
                //====================================================================
                // constructor    _ () -> { ... }
                //====================================================================
                if t == TokenKind::Init {
                    match self.parse_constructor_decl() {
                        Ok(c) => constructor = c,
                        Err(e) => {
                            eprintln!("Syntax Error in scope constructor: {}", e);
                            self.synchronize();
                        }
                    }
                    continue;
                }
                //====================================================================
                // generic -> { ... }
                //====================================================================
                if t == TokenKind::Generic {
                    generic_block = self.parse_generic_block()?;
                    continue;
                }
                //====================================================================
                //====================================================================
                // handle -> { fn1 , fn2 , ... }
                //====================================================================
                if t == TokenKind::Handle {
                    handle_block = self.parse_handle_block(handles.clone())?;
                    continue;
                }
                //====================================================================
                // public -> { ... }
                //====================================================================
                if self.peek().kind == TokenKind::Public {
                    public_block_ast = self.parse_field_block()?;
                    self.advance();
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
                // length -> <value>; data -> <name>;  // it will be a ptr for a list of data
                //====================================================================
                if t == TokenKind::TypeLength {
                    self.advance(); // consume 'length'
                    self.consume(TokenKind::Arrow, "Expected '->' after 'length'")?;
                    let value = self.parse_expression()?;
                    self.consume(TokenKind::SemiColon, "Expected ';' after length value")?;
                    length = match value {
                        Expr::LiteralInt(i) => i,
                        _ => {
                            return Err(
                                "Syntax Error: Expected integer value for length".to_string()
                            )
                        }
                    };
                    continue;
                }
                if t == TokenKind::TypeData {
                    self.advance(); // consume 'data'
                    self.consume(TokenKind::Arrow, "Expected '->' after 'data'")?;
                    data = self.get_identifier("Expected data name")?;
                    self.consume(TokenKind::SemiColon, "Expected ';' after data name")?;
                    continue;
                }
            } else {
                let s_token = t.as_str();
                print!("DEBUG: Invalid field found : {} , that is not allow in the array typed scope to use it \n\t - use custom typed scope with enable some setting it will work if it valid" , s_token);
                return Err(
                    ("Syntax Error: Invalid field  declaration at line {}, column {}").to_string(),
                );
            }
        }
        return Ok(Stmt::ArrayDecl {
            is_exported: false,
            name,
            length,
            handles,
            settings,
            data,
            public_block: public_block_ast,
            private_block: private_block_ast,
            generic_block: generic_block,
            handle_block,
            constructor,
        });
    }

    pub(crate) fn parse_str_decl(&mut self, name: String) -> Result<Stmt, String> {
        // we are in the array scope already so we don't need to consme any thing
        let mut settings: Vec<crate::parser::ast::Setting> = Vec::new();
        let mut constructor: Option<crate::parser::ast::ConstructorDecl> = None;
        let mut handles: Vec<crate::parser::ast::HandleMethods> = Vec::new();
        let mut handle_block: Vec<Stmt> = Vec::new();
        let mut public_block_ast: Vec<Stmt> = Vec::new();
        let mut private_block_ast: Vec<Stmt> = Vec::new();
        let mut length: i64 = 0;
        let mut data = String::new(); // will be a ptr for a list of data
        let mut name = name.clone();
        let mut keyword = name.clone();
        //adding the default settings to the array c
        settings.push(crate::parser::ast::Setting::CustomIndexAccess);
        settings.push(crate::parser::ast::Setting::CustomConstructor);
        settings.push(crate::parser::ast::Setting::CustomIterator);
        settings.push(crate::parser::ast::Setting::Private);
        settings.push(crate::parser::ast::Setting::Public);
        settings.push(crate::parser::ast::Setting::Length);
        settings.push(crate::parser::ast::Setting::Data);
        settings.push(crate::parser::ast::Setting::Size);
        // adding allowed handles
        //we have display , iterator , next , length , size
        handles.push(crate::parser::ast::HandleMethods::IndexAccess);
        handles.push(crate::parser::ast::HandleMethods::Display);
        handles.push(crate::parser::ast::HandleMethods::Iterator);
        handles.push(crate::parser::ast::HandleMethods::Next);
        handles.push(crate::parser::ast::HandleMethods::Length);
        handles.push(crate::parser::ast::HandleMethods::Size);
        if name != "" {
            //we not been redirect by the scope parsing fn
            name = self.get_identifier("Expected array name")?;
            self.consume(TokenKind::Arrow, "Expected '->' to open array body")?;
            self.consume(TokenKind::LBrace, "Expected '{' to open array body")?;
        }
        while !self.is_at_end() && self.peek().kind != TokenKind::RBrace {
            // we need to check if the token is valid for the setting
            let t = self.peek().kind.clone();
            if self.is_valid_setting(t.clone()) {
                // now need to know what is this section
                //====================================================================
                // constructor    _ () -> { ... }
                //====================================================================
                if t == TokenKind::Init {
                    match self.parse_constructor_decl() {
                        Ok(c) => constructor = c,
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
                    handle_block = self.parse_handle_block(handles.clone())?;
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
                // length -> <value>; data -> <name>;  // it will be a ptr for a list of data
                //====================================================================
                if t == TokenKind::TypeLength {
                    self.advance(); // consume 'length'
                    self.consume(TokenKind::Arrow, "Expected '->' after 'length'")?;
                    let value = self.parse_expression()?;
                    self.consume(TokenKind::SemiColon, "Expected ';' after length value")?;
                    length = match value {
                        Expr::LiteralInt(i) => i,
                        _ => {
                            return Err(
                                "Syntax Error: Expected integer value for length".to_string()
                            )
                        }
                    };
                    continue;
                }
                if t == TokenKind::TypeData {
                    self.advance(); // consume 'data'
                    self.consume(TokenKind::Arrow, "Expected '->' after 'data'")?;
                    data = self.get_identifier("Expected data name")?;
                    self.consume(TokenKind::SemiColon, "Expected ';' after data name")?;
                    continue;
                }
            } else {
                print!("DEBUG: Invalid field found : {} , that is not allow in the array typed scope to use it \n\t - use custom typed scope with enable some setting it will work if it valid" , t.as_str());
                return Err(
                    ("Syntax Error: Invalid field  declaration at line {}, column {}").to_string(),
                );
            }
        }
        return Ok(Stmt::StrDecl {
            is_exported: false,
            name,
            handles,
            settings,
            length,
            data,
            public_block: public_block_ast,
            private_block: private_block_ast,
            handle_block,
            constructor,
        });
    }

    pub(crate) fn parse_fn_decl(&mut self, name: String) -> Result<Stmt, String> {
        let mut settings: Vec<crate::parser::ast::Setting> = Vec::new();
        //todo: handle methods for future updates
        let mut handles: Vec<crate::parser::ast::HandleMethods> = Vec::new();
        let mut handle_block: Vec<Stmt> = Vec::new();

        let mut statement_block: Vec<Stmt> = Vec::new();
        let mut params: Vec<Param> = Vec::new();
        let mut return_type: TypeNode =
            crate::parser::ast::TypeNode::Simple(crate::parser::ast::TypeRef {
                base_type: "void".to_string(),
                size: None,
            });
        let mut name = name.clone();
        //adding the default settings to the struct scope
        settings.push(crate::parser::ast::Setting::Statement);
        settings.push(crate::parser::ast::Setting::Return);
        settings.push(crate::parser::ast::Setting::Param);
        settings.push(crate::parser::ast::Setting::Handle);
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
                            match self.parse_var_decl() {
                                Ok(crate::parser::ast::Stmt::VarDecl {
                                    name, type_node, ..
                                }) => {
                                    params.push(crate::parser::ast::Param { name, type_node });
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
        return Ok(Stmt::FnDecl {
            is_exported: false,
            name,
            params,
            return_type,
            body: statement_block,
        });
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
