use crate::parser::parser::Parser;
use crate::lexer::token::{Token, TokenKind};
use crate::parser::ast::*;

impl Parser {
    pub(crate) fn parse_array_decl(&mut self, name: String) -> Result<Stmt, String> {
            // we are in the array scope already so we don't need to consme any thing
            let mut settings: Vec<crate::parser::ast::Setting> = Vec::new();
            let mut constructor: Option<crate::parser::ast::ConstructorDecl> = None;
            let mut handles: Vec<crate::parser::ast::HandleMethods> = Vec::new();
            let mut handle_block_ast: Vec<Stmt> = Vec::new();
            let mut generic_block: Vec<Stmt> = Vec::new();
            let mut public_block_ast: Vec<Stmt> = Vec::new();
    
            let mut private_block_ast: Vec<Stmt> = Vec::new();
            let mut length: i64 = 0;
            let mut data = String::new(); // will be a ptr for a list of data
            let mut keyword = name.clone();
            //adding the default settings to the array c
            settings.push(crate::parser::ast::Setting::CustomIndexAccess);
            settings.push(crate::parser::ast::Setting::CustomConstructor);
            settings.push(crate::parser::ast::Setting::CustomKeyword);
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
                    if t == TokenKind::Underscore {
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
                    // generic -> { ... }
                    //====================================================================
                    if t == TokenKind::TypeGeneric {
                        self.advance(); // 'generic'
                        self.consume(TokenKind::Arrow, "Expected '->' after 'generic'")?;
                        self.consume(TokenKind::LBrace, "Expected '{' to open generic block")?;
    
                        while !self.is_at_end() && self.peek().kind != TokenKind::RBrace {
                            match self.parse_statement() {
                                Ok(Some(stmt)) => generic_block.push(stmt),
                                Ok(None) => {
                                    if !self.is_at_end() && self.peek().kind != TokenKind::RBrace {
                                        self.advance();
                                    }
                                }
                                Err(err) => return Err(err),
                            }
                        }
    
                        self.consume(TokenKind::RBrace, "Expected '}' to close generic block")?;
                    }
                    //====================================================================
                    //====================================================================
                    // handle -> { fn1 , fn2 , ... }
                    //====================================================================
                    if t == TokenKind::TypeHandle {
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
                    if self.peek().kind == TokenKind::TypePublic {
                        self.advance(); // 'public'
                        self.consume(TokenKind::Arrow, "Expected '->' after 'public'")?;
                        self.consume(TokenKind::LBrace, "Expected '{' to open public block")?;
    
                        while !self.is_at_end() && self.peek().kind != TokenKind::RBrace {
                            match self.parse_statement() {
                                Ok(Some(stmt)) => public_block_ast.push(stmt),
                                Ok(None) => {
                                    if !self.is_at_end() && self.peek().kind != TokenKind::RBrace {
                                        self.advance();
                                    }
                                }
                                Err(err) => return Err(err),
                            }
                        }
    
                        self.consume(TokenKind::RBrace, "Expected '}' to close public block")?;
                        if self.peek().kind == TokenKind::SemiColon {
                            self.advance();
                        }
                        continue;
                    }
                    //====================================================================
                    // private -> { ... }
                    //====================================================================
                    if t == TokenKind::TypePrivate {
                        self.advance(); // 'private'
                        self.consume(TokenKind::Arrow, "Expected '->' after 'private'")?;
                        self.consume(TokenKind::LBrace, "Expected '{' to open private block")?;
    
                        while !self.is_at_end() && self.peek().kind != TokenKind::RBrace {
                            match self.parse_statement() {
                                Ok(Some(stmt)) => private_block_ast.push(stmt),
                                Ok(None) => {
                                    if !self.is_at_end() && self.peek().kind != TokenKind::RBrace {
                                        self.advance();
                                    }
                                }
                                Err(err) => return Err(err),
                            }
                        }
    
                        self.consume(TokenKind::RBrace, "Expected '}' to close private block")?;
                        if self.peek().kind == TokenKind::SemiColon {
                            self.advance();
                        }
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
                    //====================================================================
                    // keyword -> <name>;
                    //====================================================================
                    if t == TokenKind::TypeKeyword {
                        self.advance(); // 'keyword'
                        self.consume(TokenKind::Arrow, "Expected '->' after 'keyword'")?;
                        keyword = self.get_identifier("Expected keyword name")?;
                        self.custom_keywords.push(keyword.clone());
                        self.consume(TokenKind::SemiColon, "Expected ';' after keyword name")?;
                        continue;
                    }
                } else {
                    let s_token = t.as_str();
                    print!("DEBUG: Invalid feild found : {} , that is not allow in the array typed scope to use it \n\t - use custom typed scope with enable some setting it will work if it valid" , s_token);
                    return Err(
                        ("Syntax Error: Invalid feild  declaration at line {}, column {}").to_string(),
                    );
                }
            }
        return Ok(Stmt::ArrayDecl {
            is_exported: false,
            name,
            keyword,
            length,
            handles,
            settings,
            data,
            public_block: public_block_ast,
            private_block: private_block_ast,
            generic_block: generic_block,
            handle_block: handle_block_ast,
            constructor,
        });
    }

    pub(crate) fn parse_str_decl(&mut self, name: String) -> Result<Stmt, String> {
            // we are in the array scope already so we don't need to consme any thing
            let mut settings: Vec<crate::parser::ast::Setting> = Vec::new();
            let mut constructor: Option<crate::parser::ast::ConstructorDecl> = None;
            let mut handles: Vec<crate::parser::ast::HandleMethods> = Vec::new();
            let mut handle_block_ast: Vec<Stmt> = Vec::new();
            let mut public_block_ast: Vec<Stmt> = Vec::new();
            let mut private_block_ast: Vec<Stmt> = Vec::new();
            let mut length: i64 = 0;
            let mut data = String::new(); // will be a ptr for a list of data
            let mut name = name.clone();
            let mut keyword = name.clone();
            //adding the default settings to the array c
            settings.push(crate::parser::ast::Setting::CustomIndexAccess);
            settings.push(crate::parser::ast::Setting::CustomConstructor);
            settings.push(crate::parser::ast::Setting::CustomKeyword);
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
                    if t == TokenKind::Underscore {
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
                    if t == TokenKind::TypeHandle {
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
                    if self.peek().kind == TokenKind::TypePublic {
                        self.advance(); // 'public'
                        self.consume(TokenKind::Arrow, "Expected '->' after 'public'")?;
                        self.consume(TokenKind::LBrace, "Expected '{' to open public block")?;
    
                        while !self.is_at_end() && self.peek().kind != TokenKind::RBrace {
                            match self.parse_statement() {
                                Ok(Some(stmt)) => public_block_ast.push(stmt),
                                Ok(None) => {
                                    if !self.is_at_end() && self.peek().kind != TokenKind::RBrace {
                                        self.advance();
                                    }
                                }
                                Err(err) => return Err(err),
                            }
                        }
    
                        self.consume(TokenKind::RBrace, "Expected '}' to close public block")?;
                        if self.peek().kind == TokenKind::SemiColon {
                            self.advance();
                        }
                        continue;
                    }
                    //====================================================================
                    // private -> { ... }
                    //====================================================================
                    if t == TokenKind::TypePrivate {
                        self.advance(); // 'private'
                        self.consume(TokenKind::Arrow, "Expected '->' after 'private'")?;
                        self.consume(TokenKind::LBrace, "Expected '{' to open private block")?;
    
                        while !self.is_at_end() && self.peek().kind != TokenKind::RBrace {
                            match self.parse_statement() {
                                Ok(Some(stmt)) => private_block_ast.push(stmt),
                                Ok(None) => {
                                    if !self.is_at_end() && self.peek().kind != TokenKind::RBrace {
                                        self.advance();
                                    }
                                }
                                Err(err) => return Err(err),
                            }
                        }
    
                        self.consume(TokenKind::RBrace, "Expected '}' to close private block")?;
                        if self.peek().kind == TokenKind::SemiColon {
                            self.advance();
                        }
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
                    //====================================================================
                    // keyword -> <name>;
                    //====================================================================
                    if t == TokenKind::TypeKeyword {
                        self.advance(); // 'keyword'
                        self.consume(TokenKind::Arrow, "Expected '->' after 'keyword'")?;
                        keyword = self.get_identifier("Expected keyword name")?;
                        self.custom_keywords.push(keyword.clone());
                        self.consume(TokenKind::SemiColon, "Expected ';' after keyword name")?;
                        continue;
                    }
                } else {
                    print!("DEBUG: Invalid feild found : {} , that is not allow in the array typed scope to use it \n\t - use custom typed scope with enable some setting it will work if it valid" , t.as_str());
                    return Err(
                        ("Syntax Error: Invalid feild  declaration at line {}, column {}").to_string(),
                    );
                }
            }
            return Ok(Stmt::StrDecl {
                is_exported: false,
                name,
                keyword,
                handles,
                settings,
                length,
                data,
                public_block: public_block_ast,
                private_block: private_block_ast,
                handle_block: handle_block_ast,
                constructor,
            });
        }

    pub(crate) fn parse_fn_decl(&mut self, name: String) -> Result<Stmt, String> {
            let mut settings: Vec<crate::parser::ast::Setting> = Vec::new();
            //todo: handle methods for future updates
            let mut handles: Vec<crate::parser::ast::HandleMethods> = Vec::new();
            let mut handle_block: Vec<Stmt> = Vec::new();
    
            let mut statement_block_ast: Vec<Stmt> = Vec::new();
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
    
            // adding allowed handles
            let mut is_not_in_scope = name == "";
    
            if !is_not_in_scope {
                //we not been redirect by the scope parsing fn
                name = self.get_identifier("Expected class name")?;
                self.advance();
                let mut method_params: Vec<Param> = Vec::new();
                let return_type: TypeNode;
                if self.peek().kind == TokenKind::LParen {
                    self.advance();
                    if self.peek().kind != TokenKind::RParen {
                        // we expect a list of params
                        // (a : int(32), b : int(32)) -> void
                        loop {
                            let name: String = self.get_identifier("Expected parameter name")?;
                            self.consume(TokenKind::Colon, "Expected ':' after parameter name")?;
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
                }
                is_not_in_scope = true;
                self.consume(TokenKind::LBrace, "Expected '{' to open class body")?;
            }
            // no for now
            while !self.is_at_end() && self.peek().kind != TokenKind::RBrace {
                // we need to check if the token is valid for the setting
                let t = self.peek().kind.clone();
                if self.is_valid_setting(t.clone()) && !is_not_in_scope {
                    // now need to know what is this section
                    //====================================================================
                    // handle -> { fn1 , fn2 , ... }
                    //====================================================================
                    if t == TokenKind::TypeHandle {
                        self.advance(); // 'handle'
                        self.consume(TokenKind::Arrow, "Expected '->' after 'handle'")?;
                        self.consume(TokenKind::LBrace, "Expected '{' to open handle block")?;
    
                        while !self.is_at_end() && self.peek().kind != TokenKind::RBrace {
                            // first we need to check if the function is a valid handle function and there is no other function with the same name
                            // we need to check if it a fn in the first place no other thing is allowed
                            if self.peek().kind == TokenKind::Fn {
                                self.advance();
                                let mut method_params: Vec<Param> = Vec::new();
                                let mut return_type: crate::parser::ast::TypeNode = crate::parser::ast::TypeNode::Simple(crate::parser::ast::TypeRef { base_type: "".to_string(), size: None });
                                if self.is_valid_handle(handles.clone(), self.peek().kind.clone()) {
                                    if self.peek().kind == TokenKind::LParen {
                                        self.advance();
    
                                        while !self.is_at_end() && self.peek().kind != TokenKind::RParen
                                        {
                                            // we expect a list of params
                                            // (a : int(32), b : int(32)) -> void
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
                                        self.consume(
                                            TokenKind::RParen,
                                            "Expected ')' after param list",
                                        )?;
                                        self.consume(
                                            TokenKind::Arrow,
                                            "Expected '->' after param list",
                                        )?;
                                        return_type = self.parse_type()?;
                                        self.consume(
                                            TokenKind::LBrace,
                                            "Expected '{' to open fn block",
                                        )?;
                                    }
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
                    // param -> { int a; int b; } ...
                    //====================================================================
                    if t == TokenKind::TypeParam {
                        self.advance(); // 'param'
                        self.consume(TokenKind::Arrow, "Expected '->' after 'param'")?;
                        self.consume(TokenKind::LBrace, "Expected '{' to open param block")?;
    
                        while !self.is_at_end() && self.peek().kind != TokenKind::RBrace {
                            //type name;
                            let type_node = self.parse_type()?;
                            let name = self.get_identifier("Expected parameter name")?;
                            params.push(Param {
                                name,
                                type_node: Some(type_node),
                            });
                            if self.peek().kind == TokenKind::Comma {
                                self.advance();
                            } else {
                                break;
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
                    if t == TokenKind::TypeStatement {
                        self.advance(); // 'statement'
                        self.consume(TokenKind::Arrow, "Expected '->' after 'statement'")?;
                        self.consume(TokenKind::LBrace, "Expected '{' to open statement block")?;
    
                        while !self.is_at_end() && self.peek().kind != TokenKind::RBrace {
                            match self.parse_statement() {
                                Ok(Some(stmt)) => statement_block_ast.push(stmt),
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
                    } else {
                        print!("DEBUG: Invalid feild found : {} , that is not allow in the array typed scope to use it \n\t - use custom typed scope with enable some setting it will work if it valid" , t.as_str());
                        return Err(
                            ("Syntax Error: Invalid feild  declaration at line {}, column {}")
                                .to_string(),
                        );
                    }
                }
                if is_not_in_scope {
                    let block = self.parse_block()?;
                    for stmt in block {
                        statement_block_ast.push(stmt);
                    }
                }
            }
            return Ok(Stmt::FnDecl {
                is_exported: false,
                name,
                params,
                return_type: return_type,
                body: statement_block_ast,
            });
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
