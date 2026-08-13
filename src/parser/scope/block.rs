use crate::lexer::token::TokenKind;
use crate::parser::ast::*;
use crate::parser::parser::Parser;
impl Parser {
    pub(crate) fn parse_field_block(&mut self) -> Result<Vec<Stmt>, String> {
        self.advance(); // 'public' , 'private' or 'static'
        self.consume(TokenKind::Arrow, "Expected '->' after 'public'")?;
        self.consume(TokenKind::LBrace, "Expected '{' to open public block")?;
        let mut block = Vec::new();
        while !self.is_at_end() && self.peek().kind != TokenKind::RBrace {
            //we have only fn decl and var decl so we will not use the parse_statement ever here
            let token = self.peek().kind.clone();
            if token == TokenKind::Fn {
                let st: Stmt = self.parse_fn_decl("".to_string())?;
                block.push(st);
                continue;
            }
            if self.is_type_token(&token) {
                let st = self.parse_var_decl()?;
                block.push(st);
                continue;
            }
            return Err(
                "Syntax Error: Expected fn or var declaration inside public block".to_string(),
            );
        }
        self.consume(TokenKind::RBrace, "Expected '}' to close public block")?;
        if self.peek().kind == TokenKind::SemiColon {
            self.advance();
        }
        Ok(block)
    }
    //  generic -> { T; U; V; W; X; Y; Z; }
    pub(crate) fn parse_generics(&mut self) -> Result<Vec<TypeNode>, String> {
        self.advance(); // 'generic'
        self.consume(TokenKind::Arrow, "Expected '->' after 'generic'")?;
        self.consume(TokenKind::LBrace, "Expected '{' to open generic block")?;
        let mut Types = Vec::new();
        while !self.is_at_end() && self.peek().kind != TokenKind::RBrace {
            let token = self.peek().kind.clone();
            if matches!(token, TokenKind::MadeUpType(_)) {
                let type_name = self.get_sc_type(
                    format!(
                        "Unexpected error happen in generic at line {}, column {}",
                        self.peek().line,
                        self.peek().column
                    )
                    .as_str(),
                )?;
                self.consume(TokenKind::SemiColon, "Expected ';' after type name")?;
                Types.push(type_name);
                continue;
            } else if matches!(token, TokenKind::Identifier(_)) {
                let type_name = self.get_identifier(
                    format!(
                        "Unexpected error happen in generic at line {}, column {}",
                        self.peek().line,
                        self.peek().column
                    )
                    .as_str(),
                )?;
                return Err(format!(
                    "Expected Capital type name not {} in generic block at line {}, column {} \n\t - use Capital Type name ",
                    type_name,
                    self.peek().line,
                    self.peek().column
                ));
            } else {
                return Err(format!(
                    "Unexpected Token {} in generic block at line {}, column {} \n\t - use Capital Type name ",
                    token.as_str(),
                    self.peek().line,
                    self.peek().column
                ));
            }
        }
        self.consume(TokenKind::RBrace, "Expected '}' to close generic block")?;
        if self.peek().kind == TokenKind::SemiColon {
            self.advance();
        }
        Ok(Types)
    }

    pub(crate) fn parse_handle_block(
        &mut self,
        allowed_methods: Vec<HandleMethods>,
    ) -> Result<(Vec<Stmt>), String> {
        let mut handle_fn: Vec<Stmt> = vec![];
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
                let method_name = self.peek().kind.clone().as_str().to_string();
                if self.is_valid_handle(allowed_methods.clone(), self.peek().kind.clone()) {
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

                        handle_fn.push(Stmt::FnDecl {
                            is_exported: false,
                            name: method_name.as_str().to_string(),
                            params: method_params,
                            return_type: return_type,
                            body,
                        });
                    }
                }
                //debug
                print!("DEBUG: handle_fn: {:?}", method_name);
                println!("DEBUG: token: {:?}", self.peek().kind);
                self.consume(TokenKind::RBrace, "Expected '}' to close handle block")?;
            } else {
                return Err(format!("Syntax Error: this {} is not a valid allowed handle method in this scope type (array) at line {}, column {}",self.peek().kind.as_str(), self.peek().line, self.peek().column));
            }
        }
        return Ok(handle_fn);
    }

    pub(crate) fn parse_constructor_decl(
        &mut self,
    ) -> Result<Option<crate::parser::ast::ConstructorDecl>, String> {
        self.advance(); // 'constructor'
        self.consume(TokenKind::Arrow, "Expected '->' after 'constructor'")?;
        self.consume(TokenKind::LBrace, "Expected '{' to open constructor block")?;
        let mut constructor = crate::parser::ast::ConstructorDecl {
            params: Vec::new(),
            body: Vec::new(),
            expected_types: Vec::new(),
        };

        if self.peek().kind == TokenKind::Param {
            //depug
            println!("DEBUG: constructor {:?}", self.peek().kind);
            self.advance();
            self.consume(TokenKind::Arrow, "Expected '->' after 'param'")?;
            self.consume(
                TokenKind::LBrace,
                "Expected '{' to open constructor param block",
            )?;
            while !self.is_at_end() && self.peek().kind != TokenKind::RBrace {
                match self.parse_var_decl() {
                    Ok(Stmt::VarDecl {
                        name, type_node, ..
                    }) => {
                        constructor
                            .params
                            .push(crate::parser::ast::Param { name, type_node });
                    }
                    Ok(_) => {
                        return Err("Syntax Error: Expected variable declaration in param block"
                            .to_string());
                    }
                    Err(e) => return Err(e),
                }
            }
            self.consume(
                TokenKind::RBrace,
                "Expected '}' to close constructor param block",
            )?;
            if self.peek().kind == TokenKind::SemiColon {
                self.advance();
            }
        }

        while !self.is_at_end() && self.peek().kind != TokenKind::RBrace {
            constructor
                .body
                .push(self.parse_statement()?.expect("REASON"));
        }
        self.consume(TokenKind::RBrace, "Expected '}' to close constructor block")?;
        if self.peek().kind == TokenKind::SemiColon {
            self.advance();
        }
        Ok(Some(constructor))
    }

    pub(crate) fn parse_block(&mut self) -> Result<Vec<Stmt>, String> {
        let mut stmts: Vec<Stmt> = Vec::new();
        while !self.is_at_end() && self.peek().kind != TokenKind::RBrace {
            match self.parse_statement() {
                Ok(Some(stmt)) => stmts.push(stmt),
                Ok(None) => {
                    if !self.is_at_end() && self.peek().kind != TokenKind::RBrace {
                        self.advance();
                    }
                }
                Err(err) => return Err(err),
            }
        }
        Ok(stmts)
    }

    /// يقرأ constructor بالصيغة  `_(params) -> { ... }`
    /*
       pub(crate) fn parse_constructor_decl(&mut self) -> Result<ConstructorDecl, String> {
           self.advance(); // consume '_'
           self.consume(TokenKind::LParen, "Expected '(' after constructor '_'")?;

           let mut params: Vec<Param> = Vec::new();
           if self.peek().kind != TokenKind::RParen {
               loop {
                   let (name, type_node) = if matches!(self.peek().kind, TokenKind::Identifier(_))
                       && self.tokens.get(self.current + 1).map(|token| &token.kind)
                           == Some(&TokenKind::Colon)
                   {
                       let name = self.get_identifier("Expected parameter name")?;
                       self.consume(TokenKind::Colon, "Expected ':' after parameter name")?;
                       (name, self.parse_type()?)
                   } else {
                       let type_node = self.parse_type()?;
                       let name = self.get_identifier("Expected parameter name")?;
                       (name, type_node)
                   };
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
           }

           self.consume(TokenKind::RParen, "Expected ')' after constructor params")?;
           self.consume(
               TokenKind::Arrow,
               "Expected '->' after constructor signature '_(...)'",
           )?;
           self.consume(TokenKind::LBrace, "Expected '{' for constructor body")?;
           let body = self.parse_block()?;

           Ok(ConstructorDecl {
               params,
               expected_types: vec![],
               body,
           })
       }
    */
    pub(crate) fn parse_case_decl(&mut self) -> Result<(), String> {
        Err("Standalone case declarations are only valid inside switch blocks".to_string())
        /*
        self.advance();
        let option = self.peek().kind.clone();
        let mut set = Expr::Identifier("void".to_string());
        if option == TokenKind::Underscore {
            self.advance();
            self.consume(TokenKind::FatArrow, "Expected '=>' after default case")?;
            if (self.peek().kind == TokenKind::LBrace) {
                self.advance();
                let mut body = Vec::new();
                while !self.is_at_end() && self.peek().kind != TokenKind::RBrace {
                    if self.peek().kind == TokenKind::Identifier(String::new()) {
                        set = self.parse_expression()?;
                        if self.peek().kind == TokenKind::SemiColon {
                            self.advance();
                        continue;
                    }
                    }
                    match self.parse_statement() {
                            Ok(Some(stmt)) => body.push(stmt),
                            Ok(None) => {
                                if !self.is_at_end() && self.peek().kind != TokenKind::RBrace {
                                    self.advance();
                                }
                            }
                            Err(err) => return Err(err),
                        }
                    if(self.peek().kind == TokenKind::Break){
                        self.advance();
                        self.consume(TokenKind::SemiColon, "Expected ';' after break")?;
                        break;
                    }
                    }
                }
            }else if option == TokenKind::Identifier(String::new()) {

        }
        */
    }
}
