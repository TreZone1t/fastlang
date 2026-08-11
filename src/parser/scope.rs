use std::vec;

//we will move parsing the scopes here
// fn - function
// block - block
// class - class
// struct - struct
// custom - custom
// looped - looped
// case - case
// array - array
// str - str
use crate::lexer::token::TokenKind;
use crate::parser::ast::*;
use crate::parser::parser::Parser;
impl Parser {
    pub fn get_handle_type(&self, t: TokenKind) -> HandleMethods {
        let name = HandleMethods::from_str(t.as_str());
        return name;
    }
    pub fn is_valid_handle(&mut self, handles: Vec<HandleMethods>, t: TokenKind) -> bool {
        let t_h = self.get_handle_type(t);
        for h in handles {
            if h == t_h {
                return true;
            }
        }
        false
    }

    pub fn is_valid_setting(&mut self, t: TokenKind) -> bool {
        let mut all_settings: Vec<Setting> = Vec::new();
        all_settings.push(Setting::CustomIndexAccess);
        all_settings.push(Setting::CustomConstructor);
        all_settings.push(Setting::CustomKeyword);
        all_settings.push(Setting::CustomIterator);
        all_settings.push(Setting::CustomDisplay);
        all_settings.push(Setting::CustomGeneric);
        all_settings.push(Setting::CustomOperators);
        all_settings.push(Setting::Param);
        all_settings.push(Setting::Private);
        all_settings.push(Setting::Public);
        all_settings.push(Setting::Static);
        all_settings.push(Setting::Length);
        all_settings.push(Setting::Size);
        all_settings.push(Setting::Extends);
        all_settings.push(Setting::Variants);
        all_settings.push(Setting::Data);
        all_settings.push(Setting::Error);
        all_settings.push(Setting::Statement);
        all_settings.push(Setting::Constructor);
        all_settings.push(Setting::Handle);
        all_settings.push(Setting::Return);
        let t_s = Setting::from_token(t);
        for s in all_settings {
            if s == t_s {
                return true;
            }
        }
        false
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
        self.consume(TokenKind::RBrace, "Expected '}' to close block")?;
        Ok(stmts)
    }

    pub(crate) fn parse_constructor_decl(
        &mut self,
    ) -> Result<crate::parser::ast::ConstructorDecl, String> {
        self.advance(); // '_'
        self.consume(TokenKind::LParen, "Expected '(' after constructor '_'")?;
        let mut params: Vec<crate::parser::ast::Param> = Vec::new();
        if self.peek().kind != TokenKind::RParen {
            loop {
                let type_node = self.parse_type()?;
                let name = self.get_identifier("Expected parameter name")?;

                params.push(crate::parser::ast::Param {
                    name,
                    type_node: Some(type_node.clone()),
                });

                if self.peek().kind == TokenKind::Comma {
                    self.advance();
                } else {
                    break;
                }
            }
        }
        self.consume(TokenKind::RParen, "Expected ')' after constructor params")?;
        // Constructor is fn-like: -> is required before the body
        self.consume(
            TokenKind::Arrow,
            "Expected '->' after constructor signature '_(...)'",
        )?;
        self.consume(TokenKind::LBrace, "Expected '{' for constructor body")?;
        let body = self.parse_block()?;
        Ok(crate::parser::ast::ConstructorDecl {
            params,
            expected_types: vec![],
            body,
        })
    }
    /*  ====================================================
    / Scope Declaration Parser
    / ====================================================
    /
    / Syntax:
    /   scope_type <name> -> {
    /       type      -> SomeType;           ← metadata
    /       param     -> { int a; int b; }   ← metadata (Fn/custom only)
    /       keyword   <str>;                ← metadata  (custom only)
    /       flag      <name>;                ← metadata  (custom only)
    /       public / private / static -> { ... }   ← metadata (oop scopes ex enum and custom only)
    /       legth / size / data  -> { ... }   ← metadata (str and array and custom only)
    /        error                             ← metadata (custom only)
    /      event -> { ... }          ← metadata     ( custom only)
    /      handle  -> { ... }         ← metadata     (oop scopes and custom only)
    /       return    -> <expr>;             ← metadata (Fn/block/custom)
    /      statement -> { ... }             ← impl block (executable code)
    /   }
    */
    pub(crate) fn parse_scope_decl(&mut self) -> Result<Stmt, String> {
        let token_scope_type = self.peek().kind.clone();
        let name;
        let res;
        let scope_type = match token_scope_type {
            TokenKind::TypeClass => {
                self.advance();
                if self.peek().kind == TokenKind::Identifier(&"") {
                    // we need to change the value of name
                    name = self.get_identifier("Expected scope name")?;

                    self.advance();
                    // consume the name then ->
                    self.consume(TokenKind::Arrow, "Expected '->' after scope name")?;
                    //{
                    self.advance();
                    ScopeType::Class
                } else {
                    return Err(
                        "Syntax Error: Expected scope name after 'class' at line {}, column {}"
                            .to_string(),
                    );
                }
            }
            TokenKind::TypeCustom => {
                self.advance(); // consume 'custom'
                if self.peek().kind == TokenKind::Identifier(&"") {
                    // we need to change the value of name
                    name = self.get_identifier("Expected scope name")?;

                    self.advance();
                    // consume the name then ->
                    self.consume(TokenKind::Arrow, "Expected '->' after scope name")?;
                    //{
                    self.advance();
                    // we expect a whitespace until we find the next token which will be type of scope
                    self.consume(
                        TokenKind::Enable,
                        "Expected  type  in the first of the scope body",
                    )?;
                    ScopeType::Custom
                } else {
                    return Err(
                        "Syntax Error: Expected scope name after 'custom' at line {}, column {}"
                            .to_string(),
                    );
                }
            }
            TokenKind::TypeEnum => {
                self.advance(); // consume 'enum'
                if self.peek().kind == TokenKind::Identifier(&"") {
                    // we need to change the value of name
                    name = self.get_identifier("Expected scope name")?;

                    self.advance();
                    ScopeType::Enum
                } else {
                    return Err(
                        "Syntax Error: Expected scope name after 'enum' at line {}, column {}"
                            .to_string(),
                    );
                }
            }
            TokenKind::TypeStruct => {
                self.advance(); // consume 'struct'
                if self.peek().kind == TokenKind::Identifier(&"") {
                    // we need to change the value of name
                    name = self.get_identifier("Expected scope name")?;

                    self.advance();
                    ScopeType::Struct
                } else {
                    return Err(
                        "Syntax Error: Expected scope name after 'struct' at line {}, column {}"
                            .to_string(),
                    );
                }
            }
            TokenKind::TypeScope => {
                self.advance(); // consume 'scope'
                                // Check if next is 'name'
                if self.peek().kind == TokenKind::Identifier(&"") {
                    // we need to change the value of name
                    name = self.get_identifier("Expected scope name")?;
                    // consume the name then ->
                    self.advance();
                    self.consume(TokenKind::Arrow, "Expected '->' after scope name")?;
                    //{
                    self.advance();
                    self.consume(
                        TokenKind::TypeType,
                        "Expected  type  in the first of the scope body",
                    )?;
                    // consume -> and then the type
                    self.advance();
                    let type_node = self.peek().kind.clone();
                    // we expect one of the following types
                    //array , str , block , class , struct , enum , fn , custom
                    match type_node {
                        TokenKind::TypeArray => {
                            self.advance();
                            self.consume(TokenKind::SemiColon, "Expected ';' after scope type")?;
                            ScopeType::Array
                        }
                        TokenKind::TypeStr => {
                            self.advance();
                            self.consume(TokenKind::SemiColon, "Expected ';' after scope type")?;
                            ScopeType::String
                        }
                        TokenKind::TypeBlock => {
                            self.advance();
                            self.consume(TokenKind::SemiColon, "Expected ';' after scope type")?;
                            ScopeType::Block
                        }
                        TokenKind::TypeClass => {
                            self.advance();
                            self.consume(TokenKind::SemiColon, "Expected ';' after scope type")?;
                            ScopeType::Class
                        }
                        TokenKind::TypeStruct => {
                            self.advance();
                            self.consume(TokenKind::SemiColon, "Expected ';' after scope type")?;
                            ScopeType::Struct
                        }
                        TokenKind::TypeEnum => {
                            self.advance();
                            self.consume(TokenKind::SemiColon, "Expected ';' after scope type")?;
                            ScopeType::Enum
                        }
                        TokenKind::Fn => {
                            self.advance();
                            self.consume(TokenKind::SemiColon, "Expected ';' after scope type")?;
                            ScopeType::Fn
                        }
                        TokenKind::TypeCustom => {
                            self.advance();
                            self.consume(TokenKind::SemiColon, "Expected ';' after scope type")?;
                            ScopeType::Custom
                        }
                        _ => {
                            return Err("Syntax Error: Expected scope type after 'type' at line {}, column {} /n the types are array , str , block , class , struct , enum , fn , custom".to_string());
                        }
                    };
                } else {
                    return Err(
                        "Syntax Error: Expected scope name after 'scope' at line {}, column {}"
                            .to_string(),
                    );
                }
                return Err("Syntax Error: Unexpected Error at line {}, column {}".to_string());
            }
            _ => todo!(),
        };
        res = match scope_type {
            ScopeType::Array => self.parse_array_decl(name), //* */
            ScopeType::Block => self.parse_block_decl(name),
            ScopeType::Class => self.parse_class_decl(name), // */
            ScopeType::Custom => self.parse_custom_decl(name), //*  */
            ScopeType::Enum => self.parse_enum_decl(name),   //* */
            ScopeType::Fn => self.parse_fn_decl(name),       //* */
            ScopeType::String => self.parse_str_decl(name),  //*  */
            ScopeType::Struct => self.parse_struct_decl(name), //* */
            _ => {
                return Err("unknown scope type error".to_string());
            }
        };
        return res;
    }
    //====================================================================
    // Array   :  array<T> name -> [ele1, ele2, ..., eleN];
    //====================================================================
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
            let con = self.is_valid_setting(t);
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
    //====================================================================
    //scope name -> {
    //      type -> str;
    //====================================================================*/
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
            if self.is_valid_setting(t) {
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
    //====================================================================
    // Struct   :  struct <name> -> { ... }
    //====================================================================
    //scope name -> {
    //      type -> struct;

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
        if name != "" {
            //we not been redirect by the scope parsing fn
            name = self.get_identifier("Expected struct name")?;
            self.consume(TokenKind::Arrow, "Expected '->' to open struct body")?;
            self.consume(TokenKind::LBrace, "Expected '{' to open struct body")?;
        }
        // now we are the same as the one being redirected by the scope parsing fn

        while !self.is_at_end() && self.peek().kind != TokenKind::RBrace {
            // we need to check if the token is valid for the setting
            let t = self.peek().kind.clone();
            //todo : check if the settings are in the settings vec
            if self.is_valid_setting(t) {
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
                // static -> { ... }
                //====================================================================
                if t == TokenKind::TypeStatic {
                    self.advance(); // 'static'
                    self.consume(TokenKind::Arrow, "Expected '->' after 'static'")?;
                    self.consume(TokenKind::LBrace, "Expected '{' to open static block")?;

                    while !self.is_at_end() && self.peek().kind != TokenKind::RBrace {
                        match self.parse_statement() {
                            Ok(Some(stmt)) => static_block_ast.push(stmt),
                            Ok(None) => {
                                if !self.is_at_end() && self.peek().kind != TokenKind::RBrace {
                                    self.advance();
                                }
                            }
                            Err(err) => return Err(err),
                        }
                    }

                    self.consume(TokenKind::RBrace, "Expected '}' to close static block")?;
                    if self.peek().kind == TokenKind::SemiColon {
                        self.advance();
                    }
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
            } else {
                print!("DEBUG: Invalid feild found : {} , that is not allow in the array typed scope to use it \n\t - use custom typed scope with enable some setting it will work if it valid" , t.as_str());
                return Err(
                    ("Syntax Error: Invalid feild  declaration at line {}, column {}").to_string(),
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

    //====================================================================
    // fn   :  scope <name> -> {
    //      type -> fn;
    // }
    //====================================================================
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
            if self.is_valid_setting(t) && !is_not_in_scope {
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
                            let mut return_type: TypeNode;
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
    //====================================================================
    // Class   :  class <name> -> { ... }
    //====================================================================
    //scope name -> {
    //      type -> class;
    //====================================================================
    //todo: we will add enable some settings and handle methods but with limitations
    pub(crate) fn parse_class_decl(&mut self, name: String) -> Result<Stmt, String> {
        let mut settings: Vec<crate::parser::ast::Setting> = Vec::new();
        let mut constructor: Option<crate::parser::ast::ConstructorDecl> = None;
        let mut handles: Vec<crate::parser::ast::HandleMethods> = Vec::new();
        let mut handle_block: Vec<Stmt> = Vec::new();
        let mut public_block: Vec<Stmt> = Vec::new();
        let mut private_block: Vec<Stmt> = Vec::new();
        let mut static_block: Vec<Stmt> = Vec::new();
        let mut generic_block: Vec<Stmt> = Vec::new();
        let mut length: i64 = 0;

        //we need to ensure no duplicated extends
        let mut extends = None;
        let mut has_extends = false;
        let mut name = name.clone();
        let mut keyword = name.clone();
        //adding the default settings to the class scope
        settings.push(crate::parser::ast::Setting::CustomIndexAccess);
        settings.push(crate::parser::ast::Setting::CustomKeyword);
        settings.push(crate::parser::ast::Setting::Private);
        settings.push(crate::parser::ast::Setting::Public);
        settings.push(crate::parser::ast::Setting::Static);
        settings.push(crate::parser::ast::Setting::Extends);
        // adding allowed handles
        //we have display , iterator , next , length , size
        handles.push(crate::parser::ast::HandleMethods::IndexAccess);
        handles.push(crate::parser::ast::HandleMethods::Display);
        handles.push(crate::parser::ast::HandleMethods::Iterator);
        handles.push(crate::parser::ast::HandleMethods::Next);
        handles.push(crate::parser::ast::HandleMethods::Length);
        if name != "" {
            //we not been redirect by the scope parsing fn
            name = self.get_identifier("Expected class name")?;
            // if we have extends :
            if self.peek().kind == TokenKind::Extends {
                self.advance();
                extends = Some(self.get_identifier("Expected parent class name after 'extends'")?);
                has_extends = true;
            }
            self.consume(TokenKind::Arrow, "Expected '->' to open class body")?;
            self.consume(TokenKind::LBrace, "Expected '{' to open class body")?;
        }
        while !self.is_at_end() && self.peek().kind != TokenKind::RBrace {
            // we need to check if the token is valid for the setting
            let t = self.peek().kind.clone();
            if (self.is_valid_setting(t)) {
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
                                    handle_block.push(Stmt::FnDecl {
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
                // static -> { ... }
                //====================================================================
                if t == TokenKind::TypeStatic {
                    self.advance(); // 'static'
                    self.consume(TokenKind::Arrow, "Expected '->' after 'static'")?;
                    self.consume(TokenKind::LBrace, "Expected '{' to open static block")?;

                    while !self.is_at_end() && self.peek().kind != TokenKind::RBrace {
                        match self.parse_statement() {
                            Ok(Some(stmt)) => static_block.push(stmt),
                            Ok(None) => {
                                if !self.is_at_end() && self.peek().kind != TokenKind::RBrace {
                                    self.advance();
                                }
                            }
                            Err(err) => return Err(err),
                        }
                    }

                    self.consume(TokenKind::RBrace, "Expected '}' to close static block")?;
                    if self.peek().kind == TokenKind::SemiColon {
                        self.advance();
                    }
                    continue;
                }
                //====================================================================
                // length -> <value>;
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
                    if has_extends {
                        return Err("Syntax Error: Class can only have one extends and you already have one".to_string());
                    } else {
                        self.advance(); // 'extends'
                        self.consume(TokenKind::Arrow, "Expected '->' after 'extends'")?;
                        extends = Some(
                            self.get_identifier("Expected parent class name after 'extends'")?,
                        );
                        continue;
                    }
                }
            } else {
                print!("DEBUG: Invalid feild found : {} , that is not allow in the array typed scope to use it \n\t - use custom typed scope with enable some setting it will work if it valid" , t.as_str());
                return Err(
                    ("Syntax Error: Invalid feild  declaration at line {}, column {}").to_string(),
                );
            }
        }

        return Ok(Stmt::ClassDecl {
            is_exported: false,
            name,
            keyword,
            extends,
            handles,
            settings,
            length,
            public_block,
            private_block,
            static_block,
            generic_block,
            handle_block,
            constructor,
        });
    }
    //====================================================================
    // enum   :  enum <name> -> { ... }
    //====================================================================
    //scope name -> {
    //      type -> enum;
    //====================================================================
    pub(crate) fn parse_enum_decl(&mut self, name: String) -> Result<Stmt, String> {
        let mut settings: Vec<crate::parser::ast::Setting> = Vec::new();
        let mut handles: Vec<crate::parser::ast::HandleMethods> = Vec::new();
        let mut handle_block_ast: Vec<Stmt> = Vec::new(); //*
        let mut variants: Vec<EnumVariant> = Vec::new();
        let mut length: i64 = 0; //*
        let mut name = name.clone(); //*
        let mut keyword = name.clone(); //*
        let mut is_not_in_scope = name == "";
        let mut extends = String::new();
        //adding the default settings to the array c
        settings.push(crate::parser::ast::Setting::CustomKeyword);
        settings.push(crate::parser::ast::Setting::Length);
        // adding allowed handles
        //we have display , iterator , next , length , size
        handles.push(crate::parser::ast::HandleMethods::Display);
        handles.push(crate::parser::ast::HandleMethods::Length);
        if !is_not_in_scope {
            //we not been redirect by the scope parsing fn
            name = self.get_identifier("Expected enum name")?;
            self.consume(TokenKind::Arrow, "Expected '->' to open enum body")?;
            self.consume(TokenKind::LBrace, "Expected '{' to open enum body")?;
        }
        while !self.is_at_end() && self.peek().kind != TokenKind::RBrace {
            // we have two exceptions here
            // 1. there is no any field mean are in a normal enum
            if (is_not_in_scope) {
                //we need to parse the enum variants
                // variant_name(typed_size),
                let variant_name = self.get_identifier("Expected enum variant name")?;
                if (self.peek().kind == TokenKind::LParen) {
                    self.advance();
                    let mut data_types: Vec<TypeNode> = Vec::new();
                    let temp = self.parse_type()?;
                    data_types.push(temp);
                    self.consume(TokenKind::RParen, "Expected ')' after enum variant size")?;
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
            } else {
                // 2. we are in a scope and we need to parse the scope body
                //====================================================================
                // handle -> { fn1 , fn2 , ... }
                //====================================================================
                let t = self.peek().kind.clone();
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
                // variants -> { ... }
                //====================================================================
                if t == TokenKind::TypeVariants {
                    self.advance(); // 'variants'
                    self.consume(TokenKind::Arrow, "Expected '->' after 'variants'")?;
                    self.consume(TokenKind::LBrace, "Expected '{' to open variants block")?;

                    while !self.is_at_end() && self.peek().kind != TokenKind::RBrace {
                        let variant_name = self.get_identifier("Expected enum variant name")?;
                        if (self.peek().kind == TokenKind::LParen) {
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
                // length -> <value>;
                //====================================================================
                if t == TokenKind::TypeLength {
                    self.advance(); // consume 'length'
                    self.consume(TokenKind::Arrow, "Expected '->' after 'length'")?;
                    let value = self.parse_expression()?;
                    self.consume(TokenKind::SemiColon, "Expected ';' after length value")?;
                    let temp = match value {
                        Expr::LiteralInt(i) => i,
                        _ => {
                            return Err(
                                "Syntax Error: Expected integer value for length".to_string()
                            )
                        }
                    };
                    length = temp;
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
            }
        }
        return Ok(Stmt::EnumDecl {
            is_exported: false,
            name,
            keyword,
            handles,
            settings,
            handle_block: handle_block_ast,
            variants,
        });
    }
    //====================================================================
    // custom   :  custom <name> -> { ... }
    //====================================================================
    //scope name -> {
    //      type -> custom;
    //====================================================================
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
        let mut return_type: TypeNode;
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
                    if self.is_valid_setting(t) {
                        enabled_settings.push(t);
                        self.advance();
                    } else if t == TokenKind::Identifier(&"") {
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
                let predicate = |to| to == TokenKind::All;
                enabled_settings.pop_if(predicate); //we need to remove all from the enabled list
                self.consume(TokenKind::SemiColon, "Expected ';' after disable all")?;
            } else {
                self.consume(TokenKind::LBracket, "Expected '[' or all after disable")?;
                while !self.is_at_end() && self.peek().kind != TokenKind::RBracket {
                    let t = self.peek().kind.clone();
                    let sti = Setting::from_token(t);
                    if sti == Setting::NotFound {
                        let predicate = |to| to == t;
                        enabled_settings.pop_if(predicate); //we need to remove all from the enabled list
                        self.advance();
                    } else if t == TokenKind::Identifier(&"") {
                        let hm = self.get_handle_type(t);
                        let predicate = |to| to == hm;
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
            if self.is_valid_setting(e) {
                settings.push(Setting::from_token(e));
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
            if (self.is_valid_setting(t)) {
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
                            let mut return_type: TypeNode;
                            if self.is_valid_handle(handles, self.peek().kind) {
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
                                        name,
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
                        match self.parse_var_decl_bare() {
                            Ok(stmt) => params.push(stmt),
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
    //====================================================================
    // block   :  block <name> -> { ... }
    //====================================================================
    //scope name -> {
    //      type -> block;
    //====================================================================
    pub(crate) fn parse_block_decl(&mut self, name: String) -> Result<Stmt, String> {
        // block have statements only
        let mut settings: Vec<crate::parser::ast::Setting> = Vec::new();
        settings.push(crate::parser::ast::Setting::Statement);
        let mut statements: Vec<Stmt> = Vec::new();
        if (name != "") {
            //we not been redirect by the scope parsing fn
            let name = self.get_identifier("Expected block name")?;
            self.consume(TokenKind::Arrow, "Expected '->' to open block body")?;
            self.consume(TokenKind::LBrace, "Expected '{' to open block body")?;
        }
        // now we are the same as the one being redirected by the scope parsing fn
        while !self.is_at_end() && self.peek().kind != TokenKind::RBrace {
            // we need to check sif the token is valid for the setting
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
        return Ok(Stmt::BlockDecl {
            is_exported: false,
            name,
            statements,
        });
    }
}
