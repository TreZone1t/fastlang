use crate::lexer::token::TokenKind;
use crate::parser::ast::*;
use crate::parser::parser::Parser;

impl Parser {
    pub(crate) fn parse_enum_decl(&mut self, name: String) -> Result<Stmt, String> {
        println!("DEBUG: parse_enum_decl called with name='{}'", name);
        let mut settings: Vec<Setting> = Vec::new();
        let mut handles: Vec<HandleMethods> = Vec::new();
        let mut used_handles: Vec<HandleMethods> = Vec::new();
        let mut used_settings: std::collections::HashSet<Setting> = std::collections::HashSet::new();
        let mut handle_block: Vec<Stmt> = Vec::new(); //*
        let mut variants: Vec<EnumVariant> = Vec::new();
        let mut length: i64 = 0; //*
        let mut name = name.clone(); //*
        let is_not_in_scope = name == "";
        //adding the default settings to the array c
        settings.push(Setting::Length);
        // adding allowed handles
        //we have display , iterator , next , length , size
        handles.push(HandleMethods::Display);
        handles.push(HandleMethods::Length);
        if is_not_in_scope {
            //we not been redirect by the scope parsing fn
            name = self.get_identifier("Expected enum name")?;
            self.consume(TokenKind::Arrow, "Expected '->' to open enum body")?;
            self.consume(TokenKind::LBrace, "Expected '{' to open enum body")?;
        }
        while !self.is_at_end() && self.peek().kind != TokenKind::RBrace {
            // we have two exceptions here
            // 1. there is no any field mean are in a normal enum
            if is_not_in_scope {
                //we need to parse the enum variants
                // variant_name(typed_size),
                let variant_name = self.get_identifier("Expected enum variant name")?;
                if self.peek().kind == TokenKind::LParen {
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
                let t = self.peek().kind.clone();
                if t == TokenKind::Handle {
                    if used_settings.contains(&Setting::Handle) {
                        return Err(format!("Syntax Error: Duplicate 'handle' block in enum '{}'", name));
                    }
                    used_settings.insert(Setting::Handle);
                    handle_block = self.parse_handle_block(&mut handles, &mut used_handles)?;
                } else if t == TokenKind::Variants {
                    if used_settings.contains(&Setting::Custom) { // Using Custom for Variants flag
                        return Err(format!("Syntax Error: Duplicate 'variants' block in enum '{}'", name));
                    }
                    used_settings.insert(Setting::Custom);
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
                            variants.push(EnumVariant {
                                name: variant_name,
                                data_types: Some(data_types),
                            });
                        } else {
                            variants.push(EnumVariant {
                                name: variant_name,
                                data_types: None,
                            });
                        }
                        if self.peek().kind == TokenKind::Comma {
                            self.advance();
                        }
                        continue;
                    }

                    self.consume(TokenKind::RBrace, "Expected '}' to close variants block")?;
                    self.consume(TokenKind::SemiColon, "Expected ';' after variants block")?;
                } else if t == TokenKind::TypeLength {
                    if used_settings.contains(&Setting::Length) {
                        return Err(format!("Syntax Error: Duplicate 'length' block in enum '{}'", name));
                    }
                    used_settings.insert(Setting::Length);
                    self.advance(); // consume 'length'
                    self.consume(TokenKind::Arrow, "Expected '->' after 'length'")?;
                    let value = self.parse_expression()?;
                    self.consume(TokenKind::SemiColon, "Expected ';' after length value")?;
                    let temp = match value {
                        Expr::LiteralInt(i) => i,
                        _ => return Err("Syntax Error: Expected integer value for length".to_string())
                    };
                    length = temp;
                } else {
                    return Err(format!("Syntax Error: Unsupported setting block '{}' in enum", t.as_str()));
                }
            }
        }
        let mut meta = TypeMetadata {
            name: name.clone(),
            fields: std::collections::HashMap::new(),
            constructor: None,
            params: Vec::new(),
            generics: Vec::new(),
            methods: std::collections::HashMap::new(),
            handles: used_handles,
            vars: std::collections::HashMap::new(),
            is_enum: true,
            variants: Some(variants.clone()),
        };
        for variant in &variants {
            meta.fields.insert(
                variant.name.clone(),
                TypeNode::Simple(TypeRef {
                    base_type: BaseType::from_str(&name),
                    size: None,
                }),
            );
        }
        self.metadata.insert(name.clone(), meta);

        self.consume(TokenKind::RBrace, "Expected '}' to close enum block")?;
        if self.peek().kind == TokenKind::SemiColon {
            self.advance();
        }

        return Ok(Stmt::EnumDecl {
            is_exported: false,
            name,
            handles,
            settings,
            length,
            handle_block,
            variants,
        });
    }
}
