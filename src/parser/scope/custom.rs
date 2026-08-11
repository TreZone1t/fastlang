use crate::parser::parser::Parser;
use crate::lexer::token::{Token, TokenKind};
use crate::parser::ast::*;

impl Parser {
    pub(crate) fn parse_custom_decl(&mut self, name: String) -> Result<Stmt, String> {
            let mut settings: Vec<crate::parser::ast::Setting> = Vec::new();
            let mut handles: Vec<crate::parser::ast::HandleMethods> = Vec::new();
            let mut enabled_settings: Vec<TokenKind> = Vec::new();
            let mut enabled_handle: Vec<HandleMethods> = Vec::new();
    
            let mut public_block: Vec<Stmt> = Vec::new();
            let mut private_block: Vec<Stmt> = Vec::new();
            let mut static_block_ast: Vec<Stmt> = Vec::new();
            let mut generic_block: Vec<Stmt> = Vec::new();
            let mut statement_block: Vec<Stmt> = Vec::new();
            let mut handle_block: Vec<Stmt> = Vec::new();
            let mut constructor: Option<ConstructorDecl> = None;
    
            let mut variants: Vec<EnumVariant> = Vec::new();
    
            let mut statement: Vec<Stmt> = Vec::new();
            let mut extends = String::new();
            let mut return_type: crate::parser::ast::TypeNode = crate::parser::ast::TypeNode::Simple(crate::parser::ast::TypeRef { base_type: "".to_string(), size: None });
            let mut params: Vec<Param> = Vec::new();
            let mut flags: Vec<Flag> = Vec::new();
            let mut events: Vec<EventDecl> = Vec::new();
    
            let mut fields: Vec<FieldDecl> = Vec::new(); //todo  and also  add
    
            let mut length: i64 = 0;
            let mut data;
            let mut keyword = name.clone();
    
            if name != "" {
                //we not been redirect by the scope parsing fn
                let name = self.get_identifier("Expected custom name")?;
                self.consume(TokenKind::Arrow, "Expected '->' to open custom body")?;
                self.consume(TokenKind::LBrace, "Expected '{' to open custom body")?;
            }
            // now we are the same as the one being redirected by the scope parsing fn
            // first we expect a enable line
            if self.peek().kind == TokenKind::Enable {
                self.advance();
                let t = self.peek().kind.clone();
                if t == TokenKind::All {
                    self.advance();
                    enabled_settings.push(TokenKind::All);
                    self.consume(TokenKind::SemiColon, "Expected ';' after enable all")?;
                } else {
                    self.consume(TokenKind::LBracket, "Expected '[' or all after enable")?;
                    while !self.is_at_end() && self.peek().kind != TokenKind::RBracket {
                        let t: TokenKind = self.peek().kind.clone();
                        if self.is_valid_setting(t.clone()) {
                            enabled_settings.push(t);
                            self.advance();
                        } else if t == TokenKind::Identifier("".to_string()) {
                            let hm = self.get_handle_type(t);
                            enabled_handle.push(hm);
                            self.advance();
                        } else {
                            return Err(
                                "Syntax Error: Unexpected token after enable at line {}, column {}"
                                    .to_string(),
                            );
                        }
                    }
                    self.consume(TokenKind::RBracket, "Expected ']' after enable")?;
                    self.consume(TokenKind::SemiColon, "Expected ';' after enable")?;
                }
            } else {
                return Err("Syntax Error: Expected enable line after decide a custom scope type at line {}, column {}".to_string());
            }
            // now we expect a disable line or not
            if self.peek().kind == TokenKind::Disable {
                self.advance();
                let t = self.peek().kind.clone();
                if t == TokenKind::All {
                    self.advance();
                    let predicate = |to: &mut crate::lexer::token::TokenKind| *to == TokenKind::All;
                    enabled_settings.pop_if(predicate); //we need to remove all from the enabled list
                    self.consume(TokenKind::SemiColon, "Expected ';' after disable all")?;
                } else {
                    self.consume(TokenKind::LBracket, "Expected '[' or all after disable")?;
                    while !self.is_at_end() && self.peek().kind != TokenKind::RBracket {
                        let t = self.peek().kind.clone();
                        let sti = Setting::from_token(t.clone());
                        if sti == Setting::NotFound {
                                            let predicate = |to: &mut crate::lexer::token::TokenKind| *to == t;
                            enabled_settings.pop_if(predicate); //we need to remove all from the enabled list
                            self.advance();
                        } else if t == TokenKind::Identifier("".to_string()) {
                            let hm = self.get_handle_type(t);
                            let predicate = |to: &mut crate::parser::ast::HandleMethods| *to == hm;
                            enabled_handle.pop_if(predicate);
                            self.advance();
                        } else {
                            return Err(
                                "Syntax Error: Unexpected token after disable at line {}, column {}"
                                    .to_string(),
                            );
                        }
                    }
                }
            }
            for e in enabled_settings {
                // we will check if the enable is in the settings
                if self.is_valid_setting(e.clone()) {
                    settings.push(Setting::from_token(e.clone()));
                } else {
                    return Err("Syntax Error: Unexpected error at line {}, column {}".to_string());
                }
            }
            for e in enabled_handle {
                // we will check if the enable is in the settings
                handles.push(e);
            }
            //we did now checked the enable and disable settings
            while !self.is_at_end() && self.peek().kind != TokenKind::RBrace {
                // we need to check if the token is valid for the setting
                let t = self.peek().kind.clone();
                if (self.is_valid_setting(t.clone())) {
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
                    // public -> { ... }
                    //====================================================================
                    if self.peek().kind == TokenKind::TypePublic {
                        self.advance(); // 'public'
                        self.consume(TokenKind::Arrow, "Expected '->' after 'public'")?;
                        self.consume(TokenKind::LBrace, "Expected '{' to open public block")?;
    
                        while !self.is_at_end() && self.peek().kind != TokenKind::RBrace {
                            match self.parse_statement() {
                                Ok(Some(stmt)) => public_block.push(stmt),
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
                                Ok(Some(stmt)) => private_block.push(stmt),
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
                                        handle_block.push(Stmt::FnDecl {
                                            is_exported: false,
                                            name: name.clone(),
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
                    // variants -> { ... }
                    //====================================================================
                    if t == TokenKind::TypeVariants {
                        self.advance(); // 'variants'
                        self.consume(TokenKind::Arrow, "Expected '->' after 'variants'")?;
                        self.consume(TokenKind::LBrace, "Expected '{' to open variants block")?;
    
                        while !self.is_at_end() && self.peek().kind != TokenKind::RBrace {
                            let variant_name = self.get_identifier("Expected enum variant name")?;
                            if self.peek().kind == TokenKind::LParen {
                                self.advance();
                                let mut data_types: Vec<TypeNode> = Vec::new();
                                let temp = self.parse_type()?;
                                data_types.push(temp);
                                self.consume(
                                    TokenKind::RParen,
                                    "Expected ')' after enum variant size",
                                )?;
                                self.consume(TokenKind::Comma, "Expected ',' after enum variant size")?;
                                variants.push(EnumVariant {
                                    name: variant_name,
                                    data_types: Some(data_types),
                                });
                                continue;
                            } else {
                                variants.push(EnumVariant {
                                    name: variant_name,
                                    data_types: None,
                                });
                                continue;
                            }
                        }
    
                        self.consume(TokenKind::RBrace, "Expected '}' to close variants block")?;
                        self.consume(TokenKind::SemiColon, "Expected ';' after variants block")?;
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
                            match self.parse_var_decl() {
                                Ok(crate::parser::ast::Stmt::VarDecl { name, type_node, .. }) => {
                                    params.push(crate::parser::ast::Param { name, type_node });
                                },
                                Ok(_) => {
                                    return Err("Syntax Error: Expected variable declaration".to_string());
                                },
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
                    if t == TokenKind::TypeStatement {
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
                    //====================================================================
                    // length -> <value>;  data -> <name>;
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
                    // keyword -> <str>;
                    //====================================================================
                    if t == TokenKind::TypeKeyword {
                        self.advance(); // 'keyword'
                        self.consume(TokenKind::Arrow, "Expected '->' after 'keyword'")?;
                        keyword = self.get_identifier("Expected keyword name")?;
                        self.custom_keywords.push(keyword.clone());
                        self.consume(TokenKind::SemiColon, "Expected ';' after keyword name")?;
                        continue;
                    }
    
                    //====================================================================
                    // extends -> <name>;
                    //====================================================================
                    if t == TokenKind::Extends {
                        self.advance(); // 'extends'
                        self.consume(TokenKind::Arrow, "Expected '->' after 'extends'")?;
                        extends = self.get_identifier("Expected parent class name after 'extends'")?;
                        continue;
                    }
                } else {
                    print!("DEBUG: Invalid feild found : {} , that is not allow  to use it \n\t -  enable some setting it will work if it valid" , t.as_str());
                    return Err(
                        ("Syntax Error: Invalid feild  declaration at line {}, column {}").to_string(),
                    );
                }
            }
            return Ok(Stmt::CustomDecl {
                is_exported: false,
                name,
                keyword,
                settings: Some(settings),
                handles: Some(handles),
                params: Some(params),
                flags: Some(flags),
                events: Some(events),
                fields: Some(fields),
                return_type: Some(return_type),
                public_block: Some(public_block),
                private_block: Some(private_block),
                static_block: Some(static_block_ast),
                statements: Some(statement),
                variant_block: Some(variants),
                generic_block: Some(generic_block),
                handle_block: Some(handle_block),
                constructor,
            });
        }


}
