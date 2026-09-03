use crate::frontend::lexer::token::TokenKind;
use crate::frontend::parser::ast::*;
use crate::frontend::parser::parser::Parser;

impl Parser {
    pub(crate) fn parse_enum_decl(&mut self) -> Result<Decl, String> {
        let mut variants: Vec<EnumVariant> = Vec::new();
        self.advance(); // consume 'enum'
        let name = self.get_identifier("Expected enum name")?;
        let mut generics = Vec::new();
        if self.peek().kind == TokenKind::Less {
            self.parse_generics(&mut generics)?;
        }
        if self.peek().kind == TokenKind::Arrow {
            self.advance();
        }
        self.consume(TokenKind::LBrace, "Expected '{' to open enum body")?;
        while !self.is_at_end() && self.peek().kind != TokenKind::RBrace {
            let t = self.peek().kind.clone();

            if matches!(t, TokenKind::Identifier(_)) {
                let variant_name = self.get_identifier("Expected enum variant name")?;
                let mut payload = EnumVariantPayload::None;
                let mut data_type = None;

                if self.peek().kind == TokenKind::LParen {
                    self.advance(); // consume '('
                    let mut tuple_types = Vec::new();
                    if self.peek().kind != TokenKind::RParen {
                        loop {
                            let dt = self.parse_type()?;
                            tuple_types.push(dt);
                            if self.peek().kind == TokenKind::Comma {
                                self.advance();
                            } else {
                                break;
                            }
                        }
                    }
                    self.consume(TokenKind::RParen, "Expected ')' after enum variant payload")?;
                    data_type = tuple_types.first().cloned();
                    payload = EnumVariantPayload::Tuple(tuple_types);
                } else if self.peek().kind == TokenKind::LBrace {
                    self.advance(); // consume '{'
                    let mut struct_fields = Vec::new();
                    while !self.is_at_end() && self.peek().kind != TokenKind::RBrace {
                        // Support both: `Type name;` and `name: Type;`
                        let is_colon_syntax = if let Some(next_tok) = self.tokens.get(self.current + 1) {
                            next_tok.kind == TokenKind::Colon || next_tok.kind == TokenKind::Walrus
                        } else {
                            false
                        };

                        if is_colon_syntax {
                            let p = self.parse_single_param()?;
                            struct_fields.push(p);
                        } else {
                            let field_type = self.parse_type()?;
                            let field_name = self.get_identifier("Expected field name")?;
                            let default_val = if self.peek().kind == TokenKind::Assign || self.peek().kind == TokenKind::Walrus {
                                self.advance();
                                Some(self.parse_expression()?)
                            } else {
                                None
                            };
                            struct_fields.push(Param {
                                name: field_name,
                                type_node: field_type,
                                default_value: default_val,
                            });
                        }
                        if self.peek().kind == TokenKind::SemiColon || self.peek().kind == TokenKind::Comma {
                            self.advance();
                        }
                    }
                    self.consume(TokenKind::RBrace, "Expected '}' to close struct variant body")?;
                    payload = EnumVariantPayload::Struct(struct_fields);
                }

                if self.peek().kind == TokenKind::Comma || self.peek().kind == TokenKind::SemiColon {
                    self.advance();
                }

                variants.push(EnumVariant {
                    name: variant_name,
                    payload,
                    data_type,
                });
            } else {
                return Err(
                    format!(
                        "Syntax Error: Unsupported token '{}' in enum at line {}, column {}",
                        t.as_str(),
                        self.peek().line,
                        self.peek().column
                    )
                );
            }
        }
        let mut meta = TypeMetadata {
            name: name.clone(),
            ty: BaseType::Enum { name: name.clone(), variants: variants.clone(), methods: Box::new(std::collections::HashMap::new()), generics: generics.clone() },
            fields: std::collections::HashMap::new(),
            constructor: None,
            methods: std::collections::HashMap::new(),
            handles: Vec::new(),
            vars: std::collections::HashMap::new(),
            variants: Some(variants.clone()),
        };
        for variant in &variants {
            meta.fields.insert(variant.name.clone(), BaseType::from_str(&name));
        }
        self.metadata.insert(name.clone(), meta);

        self.consume(TokenKind::RBrace, "Expected '}' to close enum block")?;
        if self.peek().kind == TokenKind::SemiColon {
            self.advance();
        }

        Ok(Decl::EnumDecl {
            visibility: Visibility::Private,
            name,
            generics,
            handles: Vec::new(),
            handle_block: Vec::new(),
            variants,
        })
    }
}
