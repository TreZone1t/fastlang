use crate::frontend::lexer::token::TokenKind;
use crate::frontend::parser::ast::*;
use crate::frontend::parser::parser::Parser;

impl Parser {
    pub(crate) fn parse_enum_decl(&mut self) -> Result<Decl, String> {
        let mut enabled_settings: Vec<Setting> = Vec::new();
        let mut used_handles: Vec<HandleMethods> = Vec::new();
        let mut used_settings = Vec::new();
        let mut handle_block: Vec<Decl> = Vec::new(); //*
        let mut variants: Vec<EnumVariant> = Vec::new();
        self.advance(); // consume 'enum'
        let name = self.get_identifier("Expected enum name")?;
        let mut generics = Vec::new();
        if self.peek().kind == TokenKind::Less {
            self.parse_generics(&mut generics)?;
        }
        enabled_settings.push(Setting::Handle);
        enabled_settings.push(Setting::Variants);
        enabled_settings.push(Setting::Data);
        self.consume(TokenKind::Arrow, "Expected '->' to open enum body")?;
        self.consume(TokenKind::LBrace, "Expected '{' to open enum body")?;
        while !self.is_at_end() && self.peek().kind != TokenKind::RBrace {
            // we have two exceptions here
            // 1. there is no any field mean are in a normal enum
            //we need to parse the enum variants
            // variant_name(typed_size),

            // 2. we are in a scope and we need to parse the scope body
            //====================================================================
            let t = self.peek().kind.clone();

            if t == TokenKind::Handle {
                if used_settings.contains(&Setting::Handle) {
                    return Err(
                        format!(
                            "Syntax Error: Duplicate 'handle' block in enum '{}'  at line {}, column {}",
                            name,
                            self.peek().line,
                            self.peek().column
                        )
                    );
                }
                used_settings.push(Setting::Handle);
                handle_block = self.parse_handle_block(&mut used_handles)?;
            } else if t == TokenKind::Variants {
                if used_settings.contains(&Setting::Variants) {
                    // Using Custom for Variants flag
                    return Err(
                        format!(
                            "Syntax Error: Duplicate 'variants' block in enum '{}'  at line {}, column {}",
                            name,
                            self.peek().line,
                            self.peek().column
                        )
                    );
                }
                used_settings.push(Setting::Variants);
                self.advance(); // 'variants'
                self.consume(TokenKind::Arrow, "Expected '->' after 'variants'")?;
                self.consume(TokenKind::LBrace, "Expected '{' to open variants block")?;

                while !self.is_at_end() && self.peek().kind != TokenKind::RBrace {
                    let variant_name = self.get_identifier("Expected enum variant name")?;
                    if self.peek().kind == TokenKind::LParen {
                        self.advance();
                        let data_type = self.parse_type()?;
                        self.consume(TokenKind::RParen, "Expected ')' after enum variant size")?;
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
            } else {
                return Err(
                    format!(
                        "Syntax Error: Unsupported setting block '{}' in enum at line {}, column {}",
                        t.as_str(),
                        self.peek().line,
                        self.peek().column
                    )
                );
            }
        }
        let mut meta = TypeMetadata {
            name: name.clone(),
            fields: std::collections::HashMap::new(),
            constructor: None,
            params: Vec::new(),
            generics: Vec::new(),
            methods: std::collections::HashMap::new(),
            handles: used_handles.clone(),
            vars: std::collections::HashMap::new(),
            is_enum: true,
            variants: Some(variants.clone()),
        };
        for variant in &variants {
            meta.fields.insert(variant.name.clone(), BaseType::from_str(&name));
        }
        for hdl in &handle_block {
            if let Decl::FnDecl { name, params, return_type, .. } = hdl {
                meta.methods.insert(name.clone(), FnType {
                    name: name.clone(),
                    params: params.clone(),
                    return_type: return_type.clone(),
                });
            }
        }
        self.metadata.insert(name.clone(), meta);

        self.consume(TokenKind::RBrace, "Expected '}' to close enum block")?;
        if self.peek().kind == TokenKind::SemiColon {
            self.advance();
        }

        Ok(Decl::EnumDecl {
            is_exported: false,
            name,
            generics,
            handles: used_handles,
            settings: used_settings,
            handle_block,
            variants,
        })
    }
}
