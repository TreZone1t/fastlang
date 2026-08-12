use crate::lexer::token::{Token, TokenKind};
use crate::parser::ast::*;
use crate::parser::parser::Parser;

impl Parser {
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
                            let return_type: TypeNode;
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
                if t == TokenKind::Variants {
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
                if t == TokenKind::Keyword {
                    self.advance(); // 'keyword'
                    self.consume(TokenKind::Arrow, "Expected '->' after 'keyword'")?;
                    keyword = self.get_identifier("Expected keyword name")?;
                    self.custom_keywords.insert(keyword.clone(), name.clone());
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
}
