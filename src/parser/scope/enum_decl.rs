use crate::lexer::token::TokenKind;
use crate::parser::ast::*;
use crate::parser::parser::Parser;

impl Parser {
    pub(crate) fn parse_enum_decl(&mut self, name: String) -> Result<Stmt, String> {
        let mut settings: Vec<crate::parser::ast::Setting> = Vec::new();
        let mut handles: Vec<crate::parser::ast::HandleMethods> = Vec::new();
        let mut handle_block: Vec<Stmt> = Vec::new(); //*
        let mut variants: Vec<EnumVariant> = Vec::new();
        let mut length: i64 = 0; //*
        let mut name = name.clone(); //*
        let is_not_in_scope = name == "";
        //adding the default settings to the array c
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
                // handle -> { fn1 , fn2 , ... }
                //====================================================================
                let t = self.peek().kind.clone();
                if t == TokenKind::Handle {
                    handle_block = self.parse_handle_block(handles.clone())?;
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
            }
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
