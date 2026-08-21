use std::collections::HashMap;

use crate::frontend::lexer::token::TokenKind;
use crate::frontend::parser::ast::*;
use crate::frontend::parser::parser::Parser;

impl Parser {
    pub(crate) fn parse_custom_decl(&mut self) -> Result<Decl, String> {
        let mut enabled_settings: Vec<Setting> = Vec::new();
        let mut used_settings: Vec<Setting> = Vec::new();
        let mut used_handles: Vec<HandleMethods> = Vec::new();

        let mut public_block: Vec<Decl> = Vec::new();
        let mut private_block: Vec<Decl> = Vec::new();
        let mut static_block: Vec<Decl> = Vec::new();
        let mut generics: Vec<BaseType> = Vec::new();
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
        let mut data: Option<Expr> = None;
        self.advance(); // consume 'custom'
        let name = self.get_identifier("Expected custom name")?;
        if self.peek().kind == TokenKind::Less {
            self.parse_generics(&mut generics)?;
        }

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
        self.consume(TokenKind::Arrow, "Expected '->' to open custom body")?;
        self.consume(TokenKind::LBrace, "Expected '{' to open custom body")?;
        //========================================================================
        // enable [];
        //========================================================================
        if self.peek().kind == TokenKind::Enable {
            self.parse_enable(&mut enabled_settings)?;
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
                    return Err(
                        format!("Syntax Error: Expected identifier or label name, found {:?}", kind)
                    );
                }

                if self.peek().kind == TokenKind::Comma {
                    self.advance();
                } else if self.peek().kind != TokenKind::SemiColon {
                    return Err("Syntax Error: Expected ',' or ';' after add item".to_string());
                }
            }
            self.consume(TokenKind::SemiColon, "Expected ';' after add statement")?;
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
                    if used_settings.contains(&Setting::Constructor) {
                        return Err(
                            format!(
                                "Syntax Error: Duplicate 'constructor' block in custom '{}'  at line {}, column {}",
                                name,
                                self.peek().line,
                                self.peek().column
                            )
                        );
                    }
                    match self.parse_constructor_decl(&mut meta) {
                        Ok(c) => {
                            constructor = c;
                        }
                        Err(e) => {
                            eprintln!("Syntax Error in scope constructor: {}", e);
                            self.synchronize();
                        }
                    }
                    used_settings.push(Setting::Constructor);
                    continue;
                }
                //====================================================================
                // public -> { ... }
                //====================================================================
                if self.peek().kind == TokenKind::Public {
                    if used_settings.contains(&Setting::Public) {
                        return Err(
                            format!(
                                "Syntax Error: Duplicate 'public' block in custom '{}'  at line {}, column {}",
                                name,
                                self.peek().line,
                                self.peek().column
                            )
                        );
                    }
                    public_block = self.parse_field_block(&mut meta, Visibility::Public)?;
                    used_settings.push(Setting::Public);
                    continue;
                }
                //====================================================================
                // private -> { ... }
                //====================================================================
                if t == TokenKind::Private {
                    if used_settings.contains(&Setting::Private) {
                        return Err(
                            format!(
                                "Syntax Error: Duplicate 'private' block in custom '{}'  at line {}, column {}",
                                name,
                                self.peek().line,
                                self.peek().column
                            )
                        );
                    }
                    private_block = self.parse_field_block(&mut meta, Visibility::Private)?;
                    used_settings.push(Setting::Private);
                    continue;
                }
                //====================================================================
                // static -> { ... }
                //====================================================================
                if t == TokenKind::Static {
                    if used_settings.contains(&Setting::Static) {
                        return Err(
                            format!(
                                "Syntax Error: Duplicate 'static' block in custom '{}'  at line {}, column {}",
                                name,
                                self.peek().line,
                                self.peek().column
                            )
                        );
                    }
                    static_block = self.parse_field_block(&mut meta, Visibility::Static)?;
                    used_settings.push(Setting::Static);
                    continue;
                }
                //====================================================================
                // handle -> { fn1 , fn2 , ... }
                //====================================================================
                if t == TokenKind::Handle {
                    if used_settings.contains(&Setting::Handle) {
                        return Err(
                            format!(
                                "Syntax Error: Duplicate 'handle' block in custom '{}'  at line {}, column {}",
                                name,
                                self.peek().line,
                                self.peek().column
                            )
                        );
                    }
                    handle_block = self.parse_handle_block(&mut used_handles)?;
                    used_settings.push(Setting::Handle);
                    continue;
                }
                //====================================================================
                // variants -> { ... }
                //====================================================================
                if t == TokenKind::Variants {
                    if used_settings.contains(&Setting::Variants) {
                        // Using Custom for Variants flag
                        return Err(
                            format!(
                                "Syntax Error: Duplicate 'variants' block in custom '{}'  at line {}, column {}",
                                name,
                                self.peek().line,
                                self.peek().column
                            )
                        );
                    }
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
                                "Expected ')' after enum variant size"
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
                    if self.peek().kind == TokenKind::SemiColon {
                        self.advance();
                    }
                    continue;
                }
                //====================================================================
                // param -> { int a; int b; } ...
                //====================================================================
                if t == TokenKind::Param {
                    if used_settings.contains(&Setting::Param) {
                        return Err(
                            format!(
                                "Syntax Error: Duplicate 'param' block in custom '{}'  at line {}, column {}",
                                name,
                                self.peek().line,
                                self.peek().column
                            )
                        );
                    }
                    self.advance(); // 'param'
                    self.consume(TokenKind::Arrow, "Expected '->' after 'param'")?;
                    self.consume(TokenKind::LBrace, "Expected '{' to open param block")?;

                    while !self.is_at_end() && self.peek().kind != TokenKind::RBrace {
                        match self.parse_var_decl(ScopeType::Custom) {
                            Ok(Decl::VarDecl { name, type_node, .. }) => {
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
                    used_settings.push(Setting::Param);
                    continue;
                }
                //====================================================================
                // return -> <type>;
                //====================================================================
                if t == TokenKind::Return {
                    if used_settings.contains(&Setting::Return) {
                        return Err(
                            format!(
                                "Syntax Error: Duplicate 'return' block in custom '{}'  at line {}, column {}",
                                name,
                                self.peek().line,
                                self.peek().column
                            )
                        );
                    }
                    self.advance(); // 'return'
                    self.consume(TokenKind::Arrow, "Expected '->' after 'return'")?;
                    return_type = self.parse_type()?;
                    self.consume(TokenKind::SemiColon, "Expected ';' after return type")?;
                    used_settings.push(Setting::Return);
                    continue;
                }
                //====================================================================
                // statement -> {  ... }
                //====================================================================
                if t == TokenKind::Statement {
                    if used_settings.contains(&Setting::Statement) {
                        return Err(
                            format!(
                                "Syntax Error: Duplicate 'statement' block in custom '{}'  at line {}, column {}",
                                name,
                                self.peek().line,
                                self.peek().column
                            )
                        );
                    }
                    self.advance(); // 'statement'
                    self.consume(TokenKind::Arrow, "Expected '->' after 'statement'")?;
                    self.consume(TokenKind::LBrace, "Expected '{' to open statement block")?;

                    while !self.is_at_end() && self.peek().kind != TokenKind::RBrace {
                        match self.parse_statement(ScopeType::Custom) {
                            Ok(Some(stmt)) => statement_block.push(stmt),
                            Ok(None) => {
                                if !self.is_at_end() && self.peek().kind != TokenKind::RBrace {
                                    self.advance();
                                }
                            }
                            Err(err) => {
                                return Err(err);
                            }
                        }
                    }

                    self.consume(TokenKind::RBrace, "Expected '}' to close statement block")?;
                    if self.peek().kind == TokenKind::SemiColon {
                        self.advance(); // consume ';'
                    }
                    used_settings.push(Setting::Statement);
                    continue;
                }

                if t == TokenKind::TypeData {
                    if used_settings.contains(&Setting::Data) {
                        return Err(
                            format!(
                                "Syntax Error: Duplicate 'data' block in custom '{}'  at line {}, column {}",
                                name,
                                self.peek().line,
                                self.peek().column
                            )
                        );
                    }
                    self.advance(); // consume 'data'
                    self.consume(TokenKind::Arrow, "Expected '->' after 'data'")?;
                    let data_expr = self.parse_expression()?;
                    data = Some(data_expr);
                    self.consume(TokenKind::SemiColon, "Expected ';' after data name")?;
                    used_settings.push(Setting::Data);
                    continue;
                }

                //====================================================================
                // extends -> <name>;
                //====================================================================
                if t == TokenKind::Extends {
                    self.advance(); // 'extends'
                    self.consume(TokenKind::Arrow, "Expected '->' after 'extends'")?;
                    extends = self.get_identifier("Expected parent class name after 'extends'")?;
                    self.consume(TokenKind::SemiColon, "Expected ';' after extends name")?;
                    used_settings.push(Setting::Extends);
                    continue;
                }
                //====================================================================
                // @label -> { ... }
                //====================================================================
                // todo : improve the settings management to insure that the label is only used once
                if let TokenKind::LabelName(name) = t.clone() {
                    let label_block = self.parse_label_decl(ScopeType::Custom)?;
                    label_blocks.push(label_block);
                    continue;
                }
            } else {
                print!(
                    "DEBUG: Invalid field found : {} , that is not allow  to use it \n\t -  enable some setting it will work if it valid",
                    t.as_str()
                );
                return Err(
                    format!(
                        "Syntax Error: Invalid field ''{:?}'' declaration at line {}, column {}",
                        t,
                        self.peek().line,
                        self.peek().column
                    )
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
        meta.handles = used_handles.clone();
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
                Expr::PropertyAccess { ref object, ref property } => {
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

            meta.vars.insert("data".to_string(), VarMetadata {
                name: "data".to_string(),
                type_node: inferred_base_type,
                visibility: Visibility::Public,
                editability: Editability::Editable,
                scope: ScopeType::Custom,
                is_array: false,
            });
        }

        self.metadata.insert(name.clone(), meta);
        Ok(Decl::CustomDecl {
            is_exported: false,
            name,
            settings: Some(used_settings),
            handles: Some(used_handles),
            params: if params.is_empty() {
                None
            } else {
                Some(params)
            },
            flags: if flags.is_empty() {
                None
            } else {
                Some(flags)
            },
            labels: if labels.is_empty() {
                None
            } else {
                Some(labels)
            },
            data,
            extends,
            return_type: Some(return_type),
            public_block: Some(public_block),
            private_block: Some(private_block),
            static_block: Some(static_block),
            statements: Some(statement_block),
            label_blocks: Some(label_blocks),
            variant_block: Some(variants),
            generics: Some(generics),
            handle_block: Some(handle_block),
            constructor,
        })
    }
}
