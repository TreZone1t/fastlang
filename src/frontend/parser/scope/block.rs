use crate::frontend::lexer::token::TokenKind;
use crate::frontend::parser::ast::*;
use crate::frontend::parser::parser::Parser;

impl Parser {

    pub(crate) fn parse_label_decl(&mut self, _scope: ScopeType) -> Result<Decl, String> {
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
                if let Some(stmt) = self.parse_statement(ScopeType::Label)? {
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
        _metadata: &mut TypeMetadata,
        _field_type: Visibility //todo : fix it
    ) -> Result<Vec<Decl>, String> {
        self.advance(); // 'public' , 'private' or 'static'
        self.consume(TokenKind::Arrow, "Expected '->' after 'public'")?;
        self.consume(TokenKind::LBrace, "Expected '{' to open public block")?;
        let mut block = Vec::new();
        while !self.is_at_end() && self.peek().kind != TokenKind::RBrace {
            //we have only fn decl and var decl so we will not use the parse_statement ever here
            let stmt = self.parse_statement(ScopeType::Block)?;
            if let Some(Stmt::Declaration(s)) = stmt {
                block.push(s);
            }
        }
        self.consume(TokenKind::RBrace, "Expected '}' to close public block")?;
        if self.peek().kind == TokenKind::SemiColon {
            self.advance();
        }
        Ok(block)
    }
    // <T, U, V, W, X, Y, Z>
    pub(crate) fn parse_generics(&mut self, generics: &mut Vec<BaseType>) -> Result<(), String> {
        generics.clear();

        self.advance(); // '<'
        while !self.is_at_end() && self.peek().kind != TokenKind::Greater {
            let token = self.peek().kind.clone();
            if matches!(token, TokenKind::Identifier(_)) {
                let type_name = self.get_identifier("Unexpected error happen")?;
                generics.push(BaseType::GenericParam(type_name));
                continue;
            } else if token == TokenKind::Comma {
                self.advance();
                continue;
            } else {
                return Err(
                    format!(
                        "Unexpected Token {} in generic block at line {}, column {} \n\t - use Capital Type name ",
                        token.as_str(),
                        self.peek().line,
                        self.peek().column
                    )
                );
            }
        }
        self.consume(TokenKind::Greater, "Expected '>' to close generic block")?;
        Ok(())
    }

    pub(crate) fn parse_handle_body(
        &mut self,
        used_methods: &mut Vec<HandleMethods>
    ) -> Result<Vec<Decl>, String> {
        let mut handle_fn: Vec<Decl> = vec![];
        self.consume(TokenKind::Arrow, "Expected '->' after 'handle'")?;
        self.consume(TokenKind::LBrace, "Expected '{' to open handle block")?;

        while !self.is_at_end() && self.peek().kind != TokenKind::RBrace {
            if self.peek().kind == TokenKind::Fn {
                self.advance();
                let method_name = self.get_identifier("Expected handle method name")?;
                let handle_kind = HandleMethods::from_str(&method_name);
                if handle_kind == HandleMethods::NotFound {
                    return Err(
                        format!(
                            "Syntax Error: '{}' is not a valid allowed handle method at line {}, column {}",
                            method_name,
                            self.peek().line,
                            self.peek().column
                        )
                    );
                }
                used_methods.push(handle_kind);

                let mut method_params: Vec<Param> = Vec::new();
                if self.peek().kind == TokenKind::LParen {
                    self.advance();
                    if self.peek().kind != TokenKind::RParen {
                        loop {
                            let name: String = self.get_identifier("Expected parameter name")?;
                            self.consume(TokenKind::Colon, "Expected ':' after parameter name")?;
                            let type_node = self.parse_type()?;
                            method_params.push(Param { name, type_node });
                            if self.peek().kind == TokenKind::Comma {
                                self.advance();
                            } else {
                                break;
                            }
                        }
                    }
                    self.consume(TokenKind::RParen, "Expected ')' after handle method parameters")?;
                }

                if matches!(handle_kind, HandleMethods::Display | HandleMethods::Iterator | HandleMethods::Next | HandleMethods::Break | HandleMethods::Continue) {
                    if !method_params.is_empty() {
                        return Err(
                            format!(
                                "Syntax Error: Handle method '{}' cannot take any parameters at line {}, column {}",
                                method_name,
                                self.peek().line,
                                self.peek().column
                            )
                        );
                    }
                }

                let default_ret = if handle_kind == HandleMethods::Display {
                    BaseType::Array {
                        base_type: Box::new(BaseType::Char),
                        size: Box::new(None),
                    }
                } else {
                    BaseType::Void
                };

                let return_type = if self.peek().kind == TokenKind::Arrow {
                    self.advance();
                    if self.peek().kind == TokenKind::LBrace {
                        default_ret
                    } else {
                        self.parse_type()?
                    }
                } else {
                    default_ret
                };

                self.consume(TokenKind::LBrace, "Expected '{' to open handle method body")?;
                let body = self.parse_block(method_name.clone())?;
                self.consume(TokenKind::RBrace, "Expected '}' to close handle method body")?;

                handle_fn.push(Decl::FnDecl {
                    is_exported: false,
                    name: method_name,
                    params: method_params,
                    return_type,
                    body,
                });
            } else {
                return Err(
                    format!(
                        "Syntax Error: Unsupported token '{}' in handle block at line {}, column {}",
                        self.peek().kind.as_str(),
                        self.peek().line,
                        self.peek().column
                    )
                );
            }
        }
        self.consume(TokenKind::RBrace, "Expected '}' to close handle block")?;
        Ok(handle_fn)
    }

    pub(crate) fn parse_constructor_decl(
        &mut self,
        meta: &mut TypeMetadata
    ) -> Result<Option<Vec<ConstructorDecl>>, String> {
        self.advance(); // 'constructor'
        self.consume(TokenKind::Arrow, "Expected '->' after 'constructor'")?;
        self.consume(TokenKind::LBrace, "Expected '{' to open constructor block")?;
        let mut constructor_list = Vec::new();

        let mut con_meta: Vec<ConstructorType> = Vec::new();
        let mut init_num = 0;
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
                        self.peek().column
                    );
                }
                self.consume(TokenKind::LBrace, "Expected '{' for constructor body")?;
                let mut body = Vec::new();
                loop {
                    //we have only fn decl and var decl so we will not use the parse_statement ever here
                    let token = self.peek().kind.clone();
                    if matches!(token, TokenKind::Identifier(_)) || token == TokenKind::This {
                        let stmt = self.parse_statement(ScopeType::Fn)?;
                        if stmt.is_none() {
                            return Err(
                                "Syntax Error: Expected statement inside constructor block".to_string()
                            );
                        }
                        body.push(stmt.unwrap());
                    } else {
                        return Err(
                            "Syntax Error: Expected only reassignment inside constructor block".to_string()
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
                init_num += 1;
                let unique_name = format!("__init__{}__{}", init_num, param.len());
                con_meta.push(ConstructorType {
                    name: unique_name,
                    params: param,
                });
            } else {
                return Err(
                    format!(
                        "Syntax Error: this {} is not allowed in constructor block at line {}, column {} \n\t - use 'init(params) -> {{ ... }}' inside constructor block to declare constructors",
                        self.peek().kind.as_str(),
                        self.peek().line,
                        self.peek().column
                    )
                );
            }
        }
        self.consume(TokenKind::RBrace, "Expected '}' to close constructor block")?;
        meta.constructor = Some(con_meta);
        return Ok(Some(constructor_list));
    }
    pub(crate) fn parse_block(&mut self, _scope: String) -> Result<Vec<Stmt>, String> {
        let mut stmts: Vec<Stmt> = Vec::new();
        while !self.is_at_end() && self.peek().kind != TokenKind::RBrace {
            match self.parse_statement(ScopeType::Block) {
                Ok(Some(stmt)) => stmts.push(stmt),
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
        Ok(stmts)
    }
}
