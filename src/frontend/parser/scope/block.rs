use crate::frontend::lexer::token::TokenKind;
use crate::frontend::parser::ast::*;
use crate::frontend::parser::parser::Parser;

impl Parser {
    pub(crate) fn parse_enable(
        &mut self,
        setting: &mut Vec<Setting>,
        handles: &mut Vec<HandleMethods>,
        flags: &mut Vec<Flag>,
    ) -> Result<(), String> {
        self.advance(); // enable
        self.consume(TokenKind::LBracket, "Expected '[' or all after enable")?;
        while !self.is_at_end() {
            let current_kind = self.peek().kind.clone();
            if current_kind == TokenKind::RBracket {
                break;
            }
            if self.is_valid_setting(current_kind.clone()) {
                let current_setting = Setting::from_token(current_kind.clone());
                setting.push(current_setting);
            }
            self.advance();
            let next_kind = self.peek().kind.clone();
            if next_kind == TokenKind::Comma {
                self.advance();
            } else if next_kind != TokenKind::RBracket {
                return Err(format!(
                    "Syntax Error: Expected ',' or ']' after setting, found '{}'",
                    next_kind.as_str()
                ));
            }
        }
        // we need now to filter the settings and enable the handles
        for e in setting.clone() {
            match e {
                Setting::CustomDisplay => handles.push(HandleMethods::Display),
                Setting::Call => handles.push(HandleMethods::Call),
                Setting::CustomIndexAccess => handles.push(HandleMethods::IndexAccess),
                Setting::CustomIterator => {
                    handles.push(HandleMethods::Iterator);
                    handles.push(HandleMethods::Next);
                }
                Setting::CustomOperators => {
                    handles.push(HandleMethods::Add);
                    handles.push(HandleMethods::Sub);
                    handles.push(HandleMethods::Mul);
                    handles.push(HandleMethods::Div);
                    handles.push(HandleMethods::Mod);
                }
                Setting::Break => {
                    if !flags.contains(&Flag::HasBreak) {
                        flags.push(Flag::HasBreak);
                    }
                    handles.push(HandleMethods::Break)
                }
                Setting::Error => {
                    if !flags.contains(&Flag::HasError) {
                        flags.push(Flag::HasError);
                    }
                    handles.push(HandleMethods::Error)
                }
                Setting::Throw => {
                    if !flags.contains(&Flag::HasThrow) {
                        flags.push(Flag::HasThrow);
                    }
                    handles.push(HandleMethods::Error)
                }
                Setting::Leave => handles.push(HandleMethods::Leave),
                Setting::Yield => handles.push(HandleMethods::Yield),
                Setting::Data => handles.push(HandleMethods::Data),
                Setting::Length => handles.push(HandleMethods::Length),
                Setting::Size => handles.push(HandleMethods::Size),
                Setting::Exit => {
                    if !flags.contains(&Flag::HasExit) {
                        flags.push(Flag::HasExit);
                    }
                    handles.push(HandleMethods::Exit)
                }
                Setting::Return => {
                    if !flags.contains(&Flag::HasReturn) {
                        flags.push(Flag::HasReturn);
                    }
                }
                Setting::Case => {
                    if !flags.contains(&Flag::HasSwitch) {
                        flags.push(Flag::HasSwitch);
                    }
                }
                _ => {}
            }
        }
        if setting.contains(&Setting::Custom) {
            setting.push(Setting::CustomIndexAccess);
            setting.push(Setting::CustomConstructor);
            setting.push(Setting::CustomGeneric);
            setting.push(Setting::CustomIterator);
            setting.push(Setting::CustomDisplay);
            setting.push(Setting::CustomOperators);
            // we need to remove the custom from the list
            setting.retain(|s| s != &Setting::Custom);
        } else if setting.contains(&Setting::All) {
            self.enable_all(setting)?;
            setting.retain(|s| s != &Setting::All);
        } else if setting.contains(&Setting::OOP) {
            self.enable_oop(setting)?;
            setting.retain(|s| s != &Setting::OOP);
        } else if setting.contains(&Setting::Function) {
            self.enable_function(setting)?;
            setting.retain(|s| s != &Setting::Function);
        }
        self.consume(TokenKind::RBracket, "Expected ']' after enable list")?;
        self.consume(TokenKind::SemiColon, "Expected ';' after enable statement")?;
        Ok(())
    }
    pub(crate) fn enable_all(&mut self, setting: &mut Vec<Setting>) -> Result<(), String> {
        setting.push(Setting::Constructor); //1
        setting.push(Setting::Private); //2
        setting.push(Setting::Public); //3
        setting.push(Setting::Static); //4
        setting.push(Setting::Extends); //5
        setting.push(Setting::Param); //6
        setting.push(Setting::Statement); //7
        setting.push(Setting::Return); //8
        setting.push(Setting::Break); //9
        setting.push(Setting::Case); //10
        setting.push(Setting::CustomIndexAccess);
        setting.push(Setting::CustomConstructor); //11
        setting.push(Setting::CustomGeneric); //12
        setting.push(Setting::CustomIterator); //13
        setting.push(Setting::CustomDisplay); //14
        setting.push(Setting::CustomOperators); //15
        setting.push(Setting::Error); //16
        setting.push(Setting::Handle); //17
        setting.push(Setting::Variants); //18
        setting.push(Setting::Length); //19
        setting.push(Setting::Data); //20

        Ok(())
    }

    pub(crate) fn enable_oop(&mut self, setting: &mut Vec<Setting>) -> Result<(), String> {
        setting.push(Setting::Constructor);
        setting.push(Setting::Private);
        setting.push(Setting::Public);
        setting.push(Setting::Static);
        setting.push(Setting::Extends);
        Ok(())
    }
    pub(crate) fn enable_function(&mut self, setting: &mut Vec<Setting>) -> Result<(), String> {
        setting.push(Setting::Param);
        setting.push(Setting::Statement);
        setting.push(Setting::Return);
        Ok(())
    }

    pub(crate) fn parse_label_decl(&mut self, scope: String) -> Result<Decl, String> {
        let label_name = if let TokenKind::LabelName(name) = self.peek().kind.clone() {
            self.advance();
            name
        } else {
            return Err("Expected label name".to_string());
        };

        let mut body = Vec::new();

        if self.peek().kind == TokenKind::Arrow {
            self.advance();
        }

        if self.peek().kind == TokenKind::LBrace {
            self.advance(); // consume '{'
            while self.peek().kind != TokenKind::RBrace && self.peek().kind != TokenKind::EOF {
                if let Some(stmt) = self.parse_statement(scope.clone())? {
                    body.push(stmt);
                }
            }
            self.consume(TokenKind::RBrace, "Expected '}' after label block")?;
        } else if self.peek().kind == TokenKind::SemiColon {
            self.advance(); // consume ';'
        } else {
            return Err(format!("Expected '{{' or ';' after label '{}'", label_name));
        }

        Ok(Decl::LabelDecl {
            name: label_name,
            body,
        })
    }

    pub(crate) fn parse_field_block(
        &mut self,
        metadata: &mut TypeMetadata,
        field_type: Visibility,
        generics: Vec<BaseType>,
    ) -> Result<Vec<Decl>, String> {
        self.advance(); // 'public' , 'private' or 'static'
        self.consume(TokenKind::Arrow, "Expected '->' after 'public'")?;
        self.consume(TokenKind::LBrace, "Expected '{' to open public block")?;
        let mut block = Vec::new();
        let mut generics = generics;
        while !self.is_at_end() && self.peek().kind != TokenKind::RBrace {
            //we have only fn decl and var decl so we will not use the parse_statement ever here
            let token = self.peek().kind.clone();
            if token == TokenKind::Fn {
                self.advance(); // consume 'fn'
                let method_name = self.get_identifier(
                    format!(
                        "Expected a scope method name after 'fn' at line {}, column {}",
                        self.peek().line,
                        self.peek().column
                    )
                    .as_str(),
                )?;
                let mut params: Vec<Param> = Vec::new();
                let return_type: BaseType;
                self.consume(TokenKind::LParen, "Expected '(' after scope method name")?;
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
                self.consume(
                    TokenKind::RParen,
                    "Expected ')' after scope method parameters",
                )?;
                self.consume(
                    TokenKind::Arrow,
                    "Expected '->' after scope method parameters",
                )?;
                if matches!(self.peek().kind, TokenKind::Identifier(_)) {
                    let type_name = self.get_identifier("Unexpected error happen")?;
                    if generics.contains(&BaseType::from_str(&type_name)) {
                        return_type = BaseType::from_str(&type_name);
                    } else {
                        return Err(format!(
                            "Syntax Error: Expected generic type after scope method name at line {}, column {}",
                            self.peek().line,
                            self.peek().column
                        ));
                    }
                } else if self.is_type_token(&self.peek().kind.clone()) {
                    return_type = self.parse_type()?;
                } else {
                    return_type = BaseType::Void;
                }
                let fn_type = FnType {
                    name: method_name.clone(),
                    params: params.clone(),
                    return_type: return_type.clone(),
                };
                self.consume(TokenKind::LBrace, "Expected '{' to open scope method body")?;
                let body = self.parse_block("oop".to_string())?;
                self.consume(TokenKind::RBrace, "Expected '}' to close scope method body")?;
                metadata.methods.insert(method_name.clone(), fn_type);
                block.push(Decl::FnDecl {
                    is_exported: false,
                    name: method_name.clone(),
                    params,
                    return_type,
                    body,
                });
                continue;
            }
            if self.is_type_token(&token) {
                let type_name = self.parse_type()?;
                let var_name = self.get_identifier(
                    format!(
                        " expected a name after type {:?} at line {}, column {}",
                        type_name.clone(),
                        self.peek().line,
                        self.peek().column
                    )
                    .as_str(),
                )?;
                metadata.fields.insert(var_name.clone(), type_name.clone());
                let mut val: Option<Expr> = None;
                if self.peek().kind == TokenKind::Arrow {
                    self.advance();
                    val = Some(self.parse_expression()?);
                }
                // ! fix it
                if self.peek().kind == TokenKind::SemiColon {
                    println!("DEBUG: parse_field_block: {:?}", self.peek().kind);
                    self.advance();
                }
                let is_heaped = if let BaseType::Pointer(_) = type_name {
                    true
                } else {
                    false
                };
                block.push(Decl::VarDecl {
                    visibility: field_type.clone(),
                    editability: Editability::Editable,
                    type_node: type_name,
                    name: var_name,
                    value: val.unwrap_or(Expr::Identifier("__default__".to_string())),
                });
                continue;
            }
            if matches!(token, TokenKind::Identifier(_)) {
                let type_name = self.get_identifier("Unexpected error happen")?;
                if generics.contains(&BaseType::from_str(&type_name)) {
                    let var_name = self.get_identifier(
                        format!(
                            " expected a name after generic type {} at line {}, column {}",
                            type_name.clone(),
                            self.peek().line,
                            self.peek().column
                        )
                        .as_str(),
                    )?;
                    let type_ = BaseType::from_str(&type_name);
                    metadata.fields.insert(var_name.clone(), type_.clone());
                    let mut val: Option<Expr> = None;
                    if self.peek().kind == TokenKind::Arrow {
                        self.advance();
                        val = Some(self.parse_expression()?);
                        // ! fix it
                        if self.peek().kind == TokenKind::SemiColon {
                            println!("DEBUG: parse_field_block: {:?}", self.peek().kind);
                            self.advance();
                        }
                    }
                    let is_heaped = if let BaseType::Pointer(_) = type_.clone() {
                        true
                    } else {
                        false
                    };
                    block.push(Decl::VarDecl {
                        visibility: field_type,
                        editability: Editability::Editable,
                        type_node: BaseType::from_str(&type_name),
                        name: var_name,
                        value: val.unwrap_or(Expr::Identifier("__default__".to_string())),
                    });
                }
            }
            println!(
                "DEBUG: token in parse_field_block: {:?} at line {}, column {}",
                token,
                self.peek().line,
                self.peek().column
            );
            return Err(format!(
                "Syntax Error: Expected fn or var declaration inside public block, found {:?}",
                token
            ));
        }
        self.consume(TokenKind::RBrace, "Expected '}' to close public block")?;
        if self.peek().kind == TokenKind::SemiColon {
            self.advance();
        }
        Ok(block)
    }
    // <T, U, V, W, X, Y, Z>
    pub(crate) fn parse_generics(&mut self, generics: &mut Vec<BaseType>) -> Result<(), String> {
        if generics.is_empty() {
            generics.clear();
        }

        self.advance(); // '<'
        while !self.is_at_end() && self.peek().kind != TokenKind::Greater {
            let token = self.peek().kind.clone();
            if matches!(token, TokenKind::Identifier(_)) {
                let type_name = self.get_identifier("Unexpected error happen")?;
                generics.push(BaseType::from_str(&type_name));
                continue;
            } else if token == TokenKind::Comma {
                self.advance();
                continue;
            } else {
                return Err(format!(
                    "Unexpected Token {} in generic block at line {}, column {} \n\t - use Capital Type name ",
                    token.as_str(),
                    self.peek().line,
                    self.peek().column
                ));
            }
        }
        self.consume(TokenKind::Greater, "Expected '>' to close generic block")?;
        Ok(())
    }

    pub(crate) fn parse_handle_block(
        &mut self,
        allowed_methods: &mut Vec<HandleMethods>,
        used_methods: &mut Vec<HandleMethods>,
    ) -> Result<Vec<Decl>, String> {
        let mut handle_fn: Vec<Decl> = vec![];
        self.advance(); // 'handle'
        self.consume(TokenKind::Arrow, "Expected '->' after 'handle'")?;
        self.consume(TokenKind::LBrace, "Expected '{' to open handle block")?;

        while !self.is_at_end() && self.peek().kind != TokenKind::RBrace {
            // first we need to check if the function is a valid handle function and there is no other function with the same name
            // we need to check if it a fn in the first place no other thing is allowed
            if self.peek().kind == TokenKind::Fn {
                self.advance();
                let mut method_params: Vec<Param> = Vec::new();
                let return_type: BaseType;
                let method_name = self.peek().kind.clone().as_str().to_string();
                if self.is_valid_handle(allowed_methods.clone(), self.peek().kind.clone())
                    && !used_methods.contains(&self.get_handle_type(self.peek().kind.clone()))
                {
                    used_methods.push(self.get_handle_type(self.peek().kind.clone()));
                    allowed_methods
                        .pop_if(|m| m == &self.get_handle_type(self.peek().kind.clone()));
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
                                    type_node: type_node,
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
                        self.consume(TokenKind::LBrace, "Expected '{' to open handle method body")?;
                        let body = self.parse_block(method_name.clone())?;

                        handle_fn.push(Decl::FnDecl {
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
                return Err(format!("Syntax Error: this {} is not a valid allowed handle method in this scope at line {}, column {}",self.peek().kind.as_str(), self.peek().line, self.peek().column));
            }
        }
        self.consume(TokenKind::RBrace, "Expected '}' to close handle block")?;
        return Ok(handle_fn);
    }

    pub(crate) fn parse_constructor_decl(
        &mut self,
        meta: &mut TypeMetadata,
    ) -> Result<Option<Vec<ConstructorDecl>>, String> {
        self.advance(); // 'constructor'
        self.consume(TokenKind::Arrow, "Expected '->' after 'constructor'")?;
        self.consume(TokenKind::LBrace, "Expected '{' to open constructor block")?;
        let mut constructor_list = Vec::new();

        let mut con_meta: Vec<ConstructorType> = Vec::new();
        let mut params_size = 0;
        let mut init_num = 0;
        let mut unique_name = String::new();
        while !self.is_at_end() && self.peek().kind != TokenKind::RBrace {
            if self.peek().kind == TokenKind::Init {
                self.advance(); // consume 'init'
                let mut param = Vec::new();
                self.consume(TokenKind::LParen, "Expected '(' after 'init'")?;
                if self.peek().kind != TokenKind::RParen {
                    // we expect a list of params
                    // (a : int(32), b : int(32)) -> void
                    loop {
                        let name: String = self.get_identifier("Expected parameter name")?;
                        self.consume(TokenKind::Colon, "Expected ':' after parameter name")?;
                        let type_node = self.parse_type()?;
                        param.push(Param {
                            name,
                            type_node: type_node,
                        });
                        params_size += 1;
                        if self.peek().kind == TokenKind::Comma {
                            self.advance();
                        } else {
                            break;
                        }
                    }
                }
                self.consume(TokenKind::RParen, "Expected ')' after constructor params")?;
                if self.peek().kind == TokenKind::Arrow {
                    self.advance();
                } else {
                    //?advice: better to add -> after init
                    println!(
                        "WARNING:constructors should have a return type after init, adding ->{{ at line {}, column {} ",
                        self.peek().line,
                        self.peek().column ,
                    );
                }
                self.consume(TokenKind::LBrace, "Expected '{' for constructor body")?;
                let mut body = Vec::new();
                loop {
                    //we have only fn decl and var decl so we will not use the parse_statement ever here
                    let token = self.peek().kind.clone();
                    if matches!(token, TokenKind::Identifier(_)) || token == TokenKind::This {
                        let stmt = self.parse_statement("constructor".to_string())?;
                        if stmt.is_none() {
                            return Err(
                                "Syntax Error: Expected statement inside constructor block"
                                    .to_string(),
                            );
                        }
                        body.push(stmt.unwrap());
                    } else {
                        return Err(
                            "Syntax Error: Expected only reassignment inside constructor block"
                                .to_string(),
                        );
                    }

                    if self.peek().kind == TokenKind::RBrace {
                        self.advance();
                        break;
                    }
                }
                constructor_list.push(ConstructorDecl {
                    expected_types: Vec::new(), // will be populated in analyzer or later
                    params: param.clone(),
                    body,
                });
                params_size = 0;
                init_num += 1;
                unique_name = format!("__init__{}__{}", init_num, params_size);
                con_meta.push(ConstructorType {
                    name: unique_name.clone(),
                    params: param,
                });
            } else {
                return Err(format!("Syntax Error: this {} is not allowed in constructor block at line {}, column {} \n\t - use 'init(params) -> {{ ... }}' inside constructor block to declare constructors", self.peek().kind.as_str(), self.peek().line, self.peek().column));
            }
        }
        self.consume(TokenKind::RBrace, "Expected '}' to close constructor block")?;
        meta.constructor = Some(con_meta);
        return Ok(Some(constructor_list));
    }
    pub(crate) fn parse_block(&mut self, scope: String) -> Result<Vec<Stmt>, String> {
        let mut stmts: Vec<Stmt> = Vec::new();
        while !self.is_at_end() && self.peek().kind != TokenKind::RBrace {
            match self.parse_statement(scope.clone()) {
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
                       type_node: type_node,
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
