use std::collections::HashMap;

use crate::frontend::lexer::token::TokenKind;
use crate::frontend::parser::ast::*;
use crate::frontend::parser::parser::Parser;

impl Parser {
    pub(crate) fn parse_custom_decl(&mut self, name: String) -> Result<Stmt, String> {
        let mut settings: Vec<Setting> = Vec::new();
        let mut handles: Vec<HandleMethods> = Vec::new();
        let mut used_handles: Vec<HandleMethods> = Vec::new();
        let mut enabled_settings: Vec<Setting> = Vec::new();
        let mut enabled_handle: Vec<HandleMethods> = Vec::new();

        let mut public_block: Vec<Decl> = Vec::new();
        let mut private_block: Vec<Decl> = Vec::new();
        let mut static_block: Vec<Decl> = Vec::new();
        let mut generics: Option<Vec<BaseType>> = None;
        let mut statement_block: Vec<Stmt> = Vec::new();
        let mut handle_block: Vec<Decl> = Vec::new();
        let mut constructor: Option<Vec<ConstructorDecl>> = None;

        let mut variants: Vec<EnumVariant> = Vec::new();

        let mut extends = String::new();
        let mut return_type: BaseType = BaseType::Unknown;
        let mut params: Vec<Param> = Vec::new();
        let mut flags: Vec<Flag> = Vec::new();
        let mut labels: Vec<String> = Vec::new();
        let mut label_blocks: Vec<Decl> = Vec::new();
        let mut length: i64 = 0;
        let mut data: Option<Expr> = None;
        let mut name = name.clone();

        let mut meta = TypeMetadata {
            name: name.clone(),
            fields: HashMap::new(),
            constructor: None,
            params: Vec::new(),
            generics: Vec::new(),
            methods: HashMap::new(),
            handles: Vec::new(),
            vars: HashMap::new(),
            is_enum: false,
            variants: None,
        };

        if name == "" {
            //we not been redirect by the scope parsing fn
            name = self.get_identifier("Expected custom name")?;
            self.consume(TokenKind::Arrow, "Expected '->' to open custom body")?;
            self.consume(TokenKind::LBrace, "Expected '{' to open custom body")?;
        }
        //========================================================================
        // enable [];
        //========================================================================
        if self.peek().kind == TokenKind::Enable {
            self.parse_enable(&mut enabled_settings, &mut enabled_handle, &mut flags)?;
        }
        //========================================================================
        // add  flag/label -> name;
        //========================================================================

        while self.peek().kind == TokenKind::Add {
            self.advance();
            let mut is_flags = false;
            let mut is_labels = false;

            let kind = self.peek().kind.clone();
            if let TokenKind::Identifier(name) = &kind {
                if name == "flags" || name == "flag" {
                    is_flags = true;
                } else if name == "labels" || name == "label" {
                    is_labels = true;
                } else {
                    return Err(
                        "Syntax Error: Expected 'flags' or 'labels' after 'add'".to_string()
                    );
                }
                self.advance();
            } else if kind == TokenKind::Label {
                is_labels = true;
                self.advance();
            } else {
                return Err("Syntax Error: Expected 'flags' or 'labels' after 'add'".to_string());
            }

            if self.peek().kind == TokenKind::Arrow {
                self.advance();
            }

            while !self.is_at_end() && self.peek().kind != TokenKind::SemiColon {
                let kind = self.peek().kind.clone();
                if let TokenKind::Identifier(name) = &kind {
                    if is_flags {
                        flags.push(Flag::Custom(name.clone()));
                    } else if is_labels {
                        labels.push(name.clone());
                    }
                    self.advance();
                } else if let TokenKind::LabelName(name) = &kind {
                    if is_labels {
                        labels.push(name.clone());
                    } else {
                        return Err("Syntax Error: Expected flag identifier".to_string());
                    }
                    self.advance();
                } else {
                    return Err(format!(
                        "Syntax Error: Expected identifier or label name, found {:?}",
                        kind
                    ));
                }

                if self.peek().kind == TokenKind::Comma {
                    self.advance();
                } else if self.peek().kind != TokenKind::SemiColon {
                    return Err("Syntax Error: Expected ',' or ';' after add item".to_string());
                }
            }
            self.consume(TokenKind::SemiColon, "Expected ';' after add statement")?;
        }
        for e in enabled_settings {
            settings.push(e.clone());
        }
        for e in enabled_handle.clone() {
            // we will check if the enable is in the settings
            handles.push(e);
        }
        //we did now checked the enable and disable settings
        while !self.is_at_end() && self.peek().kind != TokenKind::RBrace {
            // we need to check if the token is valid for the setting
            let t = self.peek().kind.clone();
            if self.is_valid_setting(t.clone()) {
                // now need to know what is this section
                //====================================================================
                // constructor    _ () -> { ... }
                //====================================================================
                if t == TokenKind::Constructor {
                    match self.parse_constructor_decl(&mut meta) {
                        Ok(c) => constructor = c,
                        Err(e) => {
                            eprintln!("Syntax Error in scope constructor: {}", e);
                            self.synchronize();
                        }
                    }
                    continue;
                }
                //====================================================================
                // public -> { ... }
                //====================================================================
                if self.peek().kind == TokenKind::Public {
                    public_block = self.parse_field_block(
                        &mut meta,
                        Visibility::Public,
                        generics.clone().unwrap(),
                    )?;
                    continue;
                }
                //====================================================================
                // private -> { ... }
                //====================================================================
                if t == TokenKind::Private {
                    private_block = self.parse_field_block(
                        &mut meta,
                        Visibility::Private,
                        generics.clone().unwrap(),
                    )?;
                    continue;
                }
                //====================================================================
                // static -> { ... }
                //====================================================================
                if t == TokenKind::Static {
                    static_block = self.parse_field_block(
                        &mut meta,
                        Visibility::Static,
                        generics.clone().unwrap(),
                    )?;
                    continue;
                }
                //====================================================================
                // handle -> { fn1 , fn2 , ... }
                //====================================================================
                if t == TokenKind::Handle {
                    handle_block = self.parse_handle_block(&mut handles, &mut used_handles)?;
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
                            let data_type = self.parse_type()?;
                            self.consume(
                                TokenKind::RParen,
                                "Expected ')' after enum variant size",
                            )?;
                            self.consume(TokenKind::Comma, "Expected ',' after enum variant size")?;
                            variants.push(EnumVariant {
                                name: variant_name,
                                data_type: Some(data_type),
                            });
                        } else {
                            variants.push(EnumVariant {
                                name: variant_name,
                                data_type: None,
                            });
                        }
                        if self.peek().kind == TokenKind::Comma {
                            self.advance();
                        }
                        continue;
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
                        match self.parse_var_decl(false, false) {
                            Ok(Decl::VarDecl {
                                name, type_node, ..
                            }) => {
                                params.push(Param {
                                    name: name.clone(),
                                    type_node: type_node.clone(),
                                });
                                meta.params.push(Param { name, type_node });
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
                        match self.parse_statement("statement".to_string()) {
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
                    let data_expr = self.parse_expression()?;
                    data = Some(data_expr);
                    self.consume(TokenKind::SemiColon, "Expected ';' after data name")?;
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

                if let TokenKind::LabelName(name) = t.clone() {
                    self.advance(); // consume label name
                    self.consume(TokenKind::Arrow, "Expected '->' after label block")?;
                    self.consume(TokenKind::LBrace, "Expected '{' to open label block")?;
                    let mut body = Vec::new();
                    while !self.is_at_end() && self.peek().kind != TokenKind::RBrace {
                        match self.parse_statement(name.clone()) {
                            Ok(Some(stmt)) => body.push(stmt),
                            Ok(None) => {
                                if !self.is_at_end() && self.peek().kind != TokenKind::RBrace {
                                    self.advance();
                                }
                            }
                            Err(err) => return Err(err),
                        }
                    }
                    self.consume(TokenKind::RBrace, "Expected '}' to close label block")?;
                    label_blocks.push(Decl::LabelDecl { name: name, body });
                    continue;
                }
            } else {
                print!("DEBUG: Invalid field found : {} , that is not allow  to use it \n\t -  enable some setting it will work if it valid" , t.as_str());
                return Err(
                    ("Syntax Error: Invalid field  declaration at line {}, column {}").to_string(),
                );
            }
        }
        if self.peek().kind == TokenKind::RBrace {
            self.advance();
        } else {
            return Err("Syntax Error: Expected '}' after custom scope body".to_string());
        }
        if self.peek().kind == TokenKind::SemiColon {
            self.advance();
        }
        meta.handles = enabled_handle.clone();
        if let Some(ref data_expr) = data {
            let inferred_base_type = match data_expr {
                Expr::LiteralInt(_) => BaseType::Int32,
                Expr::LiteralFloat(_) => BaseType::Float32,
                Expr::LiteralString(_) => BaseType::Unknown,
                Expr::LiteralChar(_) => BaseType::Char,
                Expr::LiteralBool(_) => BaseType::Bool,
                Expr::NamespaceAccess { ref namespace, .. } => BaseType::from_str(&namespace),
                Expr::Instantiate { ref target, .. } => {
                    if let Expr::Identifier(ref n) = **target {
                        BaseType::from_str(n)
                    } else {
                        BaseType::Unknown
                    }
                }
                Expr::Identifier(ref var_name) => {
                    if let Some(var_meta) = meta.vars.get(var_name) {
                        var_meta.type_node.clone()
                    } else {
                        BaseType::Unknown
                    }
                }
                Expr::PropertyAccess {
                    ref object,
                    ref property,
                } => {
                    if let Expr::This = **object {
                        if let Some(var_meta) = meta.vars.get(property) {
                            var_meta.type_node.clone()
                        } else {
                            BaseType::Unknown
                        }
                    } else {
                        BaseType::Unknown
                    }
                }
                _ => BaseType::Unknown,
            };

            meta.vars.insert(
                "data".to_string(),
                VarMetadata {
                    name: "data".to_string(),
                    type_node: inferred_base_type,
                    visibility: Visibility::Public,
                    editability: Editability::Editable,
                    is_array: false,
                },
            );
        }

        self.metadata.insert(name.clone(), meta);
        Ok(Stmt::Declaration(Decl::CustomDecl {
            is_exported: false,
            name,
            settings: Some(settings),
            handles: Some(enabled_handle),
            params: if params.is_empty() {
                None
            } else {
                Some(params)
            },
            flags: if flags.is_empty() { None } else { Some(flags) },
            labels: if labels.is_empty() {
                None
            } else {
                Some(labels)
            },

            length,
            data,
            extends,
            return_type: Some(return_type),
            public_block: Some(public_block),
            private_block: Some(private_block),
            static_block: Some(static_block),
            statements: Some(statement_block),
            label_blocks: Some(label_blocks),
            variant_block: Some(variants),
            generics: generics,
            handle_block: Some(handle_block),
            constructor,
        }))
    }
}
