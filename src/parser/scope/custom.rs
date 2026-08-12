use crate::lexer::token::{Token, TokenKind};
use crate::parser::ast::*;
use crate::parser::parser::Parser;

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
        let mut return_type: crate::parser::ast::TypeNode =
            crate::parser::ast::TypeNode::Simple(crate::parser::ast::TypeRef {
                base_type: "".to_string(),
                size: None,
            });
        let mut params: Vec<Param> = Vec::new();
        let mut flags: Vec<Flag> = Vec::new();
        let mut events: Vec<EventDecl> = Vec::new();

        let mut fields: Vec<FieldDecl> = Vec::new(); //todo  and also  add

        let mut length: i64 = 0;
        let mut data;
        let mut keyword = name.clone();

        if name == "" {
            //we not been redirect by the scope parsing fn
            let name = self.get_identifier("Expected custom name")?;
            self.consume(TokenKind::Arrow, "Expected '->' to open custom body")?;
            self.consume(TokenKind::LBrace, "Expected '{' to open custom body")?;
        }
        // now we are the same as the one being redirected by the scope parsing fn
        // ==========================================
        // 1. قراءة سطر الـ Enable بصرامة
        // ==========================================
        if self.peek().kind == TokenKind::Enable {
            self.advance();
            let t = self.peek().kind.clone();
            if t == TokenKind::All {
                self.advance();
                enabled_settings.push(TokenKind::All);
                self.consume(TokenKind::SemiColon, "Expected ';' after enable all")?;
            } else {
                self.consume(TokenKind::LBracket, "Expected '[' or all after enable")?;

                // اللوب الصارمة الجديدة
                while !self.is_at_end() {
                    let current_kind = self.peek().kind.clone();

                    // كسر اللوب فوراً عند رؤية القوس الأيمن
                    if current_kind == TokenKind::RBracket {
                        break;
                    }

                    // التحقق من صحة الإعداد
                    if self.is_valid_setting(current_kind.clone()) {
                        enabled_settings.push(current_kind.clone());
                        self.advance();
                    } else if current_kind == TokenKind::Identifier("".to_string()) {
                        let hm = self.get_handle_type(current_kind.clone());
                        enabled_handle.push(hm);
                        self.advance();
                    } else {
                        return Err(format!(
                            "Syntax Error: Invalid setting '{}' inside enable array",
                            current_kind.as_str()
                        ));
                    }

                    // التأكد من وجود فاصلة أو نهاية القوس بعد كل إعداد
                    let next_kind = self.peek().kind.clone();
                    if next_kind == TokenKind::Comma {
                        self.advance(); // نتخطى الفاصلة ونكمل
                    } else if next_kind != TokenKind::RBracket {
                        return Err(format!(
                            "Syntax Error: Expected ',' or ']' after setting, found '{}'",
                            next_kind.as_str()
                        ));
                    }
                }

                self.consume(TokenKind::RBracket, "Expected ']' after enable list")?;
                self.consume(TokenKind::SemiColon, "Expected ';' after enable statement")?;
            }
        } else {
            return Err(
                "Syntax Error: Expected enable line after decide a custom scope type".to_string(),
            );
        }

        // ==========================================
        // 2. قراءة سطر الـ Disable بصرامة (اختياري)
        // ==========================================
        if self.peek().kind == TokenKind::Disable {
            self.advance();
            let t = self.peek().kind.clone();
            if t == TokenKind::All {
                self.advance();
                let predicate = |to: &mut crate::lexer::token::TokenKind| *to == TokenKind::All;
                enabled_settings.pop_if(predicate);
                self.consume(TokenKind::SemiColon, "Expected ';' after disable all")?;
            } else {
                self.consume(TokenKind::LBracket, "Expected '[' or all after disable")?;

                // اللوب الصارمة الجديدة للـ Disable
                while !self.is_at_end() {
                    let current_kind = self.peek().kind.clone();

                    if current_kind == TokenKind::RBracket {
                        break;
                    }

                    let sti = Setting::from_token(current_kind.clone());
                    if sti == Setting::NotFound {
                        let predicate =
                            |to: &mut crate::lexer::token::TokenKind| *to == current_kind;
                        enabled_settings.pop_if(predicate);
                        self.advance();
                    } else if current_kind == TokenKind::Identifier("".to_string()) {
                        let hm = self.get_handle_type(current_kind.clone());
                        let predicate = |to: &mut crate::parser::ast::HandleMethods| *to == hm;
                        enabled_handle.pop_if(predicate);
                        self.advance();
                    } else {
                        return Err(format!(
                            "Syntax Error: Invalid setting '{}' inside disable array",
                            current_kind.as_str()
                        ));
                    }

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

                self.consume(TokenKind::RBracket, "Expected ']' after disable list")?;
                self.consume(TokenKind::SemiColon, "Expected ';' after disable statement")?;
            }
        }
        for e in enabled_settings {
            // we will check if the enable is in the settings
            if self.is_valid_setting(e.clone()) {
                settings.push(Setting::from_token(e.clone()));
            } else {
                return Err(format!(
                    "Syntax Error: Invalid custom setting '{}' in custom scope",
                    e.as_str()
                ));
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
                // generic -> { ... }
                //====================================================================
                if t == TokenKind::Generic {
                    generic_block = self.parse_generic_block()?;
                }
                //====================================================================
                // public -> { ... }
                //====================================================================
                if self.peek().kind == TokenKind::Public {
                    public_block = self.parse_field_block()?;
                    continue;
                }
                //====================================================================
                // private -> { ... }
                //====================================================================
                if t == TokenKind::Private {
                    private_block = self.parse_field_block()?;
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
                            let mut return_type: crate::parser::ast::TypeNode =
                                crate::parser::ast::TypeNode::Simple(crate::parser::ast::TypeRef {
                                    base_type: "".to_string(),
                                    size: None,
                                });
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
                if t == TokenKind::Variants {
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
                if t == TokenKind::Keyword {
                    self.advance(); // 'keyword'
                    self.consume(TokenKind::Arrow, "Expected '->' after 'keyword'")?;
                    keyword = self.get_identifier("Expected keyword name")?;
                    // key = الكلمة المخصصة, value = اسم الـ scope الأصلي
                    self.custom_keywords.insert(keyword.clone(), name.clone());
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
                print!("DEBUG: Invalid field found : {} , that is not allow  to use it \n\t -  enable some setting it will work if it valid" , t.as_str());
                return Err(
                    ("Syntax Error: Invalid field  declaration at line {}, column {}").to_string(),
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
