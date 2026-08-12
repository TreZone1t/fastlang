use crate::lexer::token::TokenKind;
use crate::parser::ast::*;
use crate::parser::parser::Parser;
impl Parser {
    pub(crate) fn parse_public_block(&mut self) -> Result<Vec<Stmt>, String> {
        self.advance(); // 'public'
        self.consume(TokenKind::Arrow, "Expected '->' after 'public'")?;
        self.consume(TokenKind::LBrace, "Expected '{' to open public block")?;
        let mut block = Vec::new();
        while !self.is_at_end() && self.peek().kind != TokenKind::RBrace {
            if let Some(stmt) = self.parse_statement()? {
                block.push(stmt);
            }
        }
        self.consume(TokenKind::RBrace, "Expected '}' to close public block")?;
        if self.peek().kind == TokenKind::SemiColon {
            self.advance();
        }
        Ok(block)
    }

    pub(crate) fn parse_private_block(&mut self) -> Result<Vec<Stmt>, String> {
        self.advance(); // 'private'
        self.consume(TokenKind::Arrow, "Expected '->' after 'private'")?;
        self.consume(TokenKind::LBrace, "Expected '{' to open private block")?;
        let mut block = Vec::new();
        while !self.is_at_end() && self.peek().kind != TokenKind::RBrace {
            if let Some(stmt) = self.parse_statement()? {
                block.push(stmt);
            }
        }
        self.consume(TokenKind::RBrace, "Expected '}' to close private block")?;
        if self.peek().kind == TokenKind::SemiColon {
            self.advance();
        }
        Ok(block)
    }

    pub(crate) fn parse_static_block(&mut self) -> Result<Vec<Stmt>, String> {
        self.advance(); // 'static'
        self.consume(TokenKind::Arrow, "Expected '->' after 'static'")?;
        self.consume(TokenKind::LBrace, "Expected '{' to open static block")?;
        let mut block = Vec::new();
        while !self.is_at_end() && self.peek().kind != TokenKind::RBrace {
            if let Some(stmt) = self.parse_statement()? {
                block.push(stmt);
            }
        }
        self.consume(TokenKind::RBrace, "Expected '}' to close static block")?;
        if self.peek().kind == TokenKind::SemiColon {
            self.advance();
        }
        Ok(block)
    }

    pub(crate) fn parse_generic_block(&mut self) -> Result<Vec<Stmt>, String> {
        self.advance(); // 'generic'
        self.consume(TokenKind::Arrow, "Expected '->' after 'generic'")?;
        self.consume(TokenKind::LBrace, "Expected '{' to open generic block")?;
        let mut block = Vec::new();
        while !self.is_at_end() && self.peek().kind != TokenKind::RBrace {
            if let Some(stmt) = self.parse_statement()? {
                block.push(stmt);
            }
        }
        self.consume(TokenKind::RBrace, "Expected '}' to close generic block")?;
        if self.peek().kind == TokenKind::SemiColon {
            self.advance();
        }
        Ok(block)
    }

    pub(crate) fn parse_handle_block(
        &mut self,
        allowed_methods: &[HandleMethods],
    ) -> Result<(Vec<Stmt>, Vec<HandleMethods>), String> {
        self.advance(); // 'handle'
        self.consume(TokenKind::Arrow, "Expected '->' after 'handle'")?;
        self.consume(TokenKind::LBrace, "Expected '{' to open handle block")?;
        let mut handles = Vec::new();
        let mut handle_block = Vec::new();
        while !self.is_at_end() && self.peek().kind != TokenKind::RBrace {
            let stmt_opt = self.parse_statement()?;
            if stmt_opt.is_none() {
                continue;
            }
            let stmt = stmt_opt.unwrap();
            if let Stmt::FnDecl {
                name: ref fn_name, ..
            } = stmt
            {
                let hm = match fn_name.as_str() {
                    "index_access" => HandleMethods::IndexAccess,
                    "display" => HandleMethods::Display,
                    "add" => HandleMethods::Add,
                    "sub" => HandleMethods::Sub,
                    "mul" => HandleMethods::Mul,
                    "div" => HandleMethods::Div,
                    "mod" => HandleMethods::Mod,
                    "iterator" => HandleMethods::Iterator,
                    "next" => HandleMethods::Next,
                    "length" => HandleMethods::Length,
                    "size" => HandleMethods::Size,
                    _ => {
                        return Err(format!(
                            "Semantic Error: Invalid handle function name '{}'.",
                            fn_name
                        ));
                    }
                };
                let found = allowed_methods.iter().any(|a| *a == hm);
                if found {
                    handles.push(hm);
                } else {
                    return Err(format!(
                        "Syntax Error: '{}' handle not allowed in this scope.",
                        fn_name
                    ));
                }
            } else {
                return Err("Syntax Error: Expected fn declaration inside handle block".to_string());
            }
            handle_block.push(stmt);
        }
        self.consume(TokenKind::RBrace, "Expected '}' to close handle block")?;
        if self.peek().kind == TokenKind::SemiColon {
            self.advance();
        }
        Ok((handle_block, handles))
    }

    pub(crate) fn parse_constructor_decl_block(
        &mut self,
    ) -> Result<Option<crate::parser::ast::ConstructorDecl>, String> {
        self.advance(); // 'constructor'
        self.consume(TokenKind::Arrow, "Expected '->' after 'constructor'")?;
        self.consume(TokenKind::LBrace, "Expected '{' to open constructor block")?;
        let mut constructor = crate::parser::ast::ConstructorDecl {
            params: Vec::new(),
            body: Vec::new(),
            expected_types: Vec::new(),
        };

        if self.peek().kind == TokenKind::TypeParam {
            self.advance();
            self.consume(TokenKind::Arrow, "Expected '->' after 'param'")?;
            self.consume(
                TokenKind::LBrace,
                "Expected '{' to open constructor param block",
            )?;
            while !self.is_at_end() && self.peek().kind != TokenKind::RBrace {
                match self.parse_var_decl() {
                    Ok(Stmt::VarDecl {
                        name, type_node, ..
                    }) => {
                        constructor
                            .params
                            .push(crate::parser::ast::Param { name, type_node });
                    }
                    Ok(_) => {
                        return Err("Syntax Error: Expected variable declaration in param block"
                            .to_string());
                    }
                    Err(e) => return Err(e),
                }
            }
            self.consume(
                TokenKind::RBrace,
                "Expected '}' to close constructor param block",
            )?;
            if self.peek().kind == TokenKind::SemiColon {
                self.advance();
            }
        }

        while !self.is_at_end() && self.peek().kind != TokenKind::RBrace {
            constructor
                .body
                .push(self.parse_statement()?.expect("REASON"));
        }
        self.consume(TokenKind::RBrace, "Expected '}' to close constructor block")?;
        if self.peek().kind == TokenKind::SemiColon {
            self.advance();
        }
        Ok(Some(constructor))
    }

    pub(crate) fn parse_block(&mut self) -> Result<Vec<Stmt>, String> {
        let mut stmts: Vec<Stmt> = Vec::new();
        while !self.is_at_end() && self.peek().kind != TokenKind::RBrace {
            match self.parse_statement() {
                Ok(Some(stmt)) => stmts.push(stmt),
                Ok(None) => {
                    if !self.is_at_end() && self.peek().kind != TokenKind::RBrace {
                        self.advance();
                    }
                }
                Err(err) => return Err(err),
            }
        }
        self.consume(TokenKind::RBrace, "Expected '}' to close block")?;
        Ok(stmts)
    }

    /// يقرأ constructor بالصيغة  `_(params) -> { ... }`
    pub(crate) fn parse_constructor_decl(&mut self) -> Result<ConstructorDecl, String> {
        self.advance(); // consume '_'
        self.consume(TokenKind::LParen, "Expected '(' after constructor '_'")?;

        let mut params: Vec<Param> = Vec::new();
        if self.peek().kind != TokenKind::RParen {
            loop {
                let (name, type_node) = if matches!(self.peek().kind, TokenKind::Identifier(_))
                    && self.tokens.get(self.current + 1).map(|token| &token.kind) == Some(&TokenKind::Colon)
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
                    type_node: Some(type_node),
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
