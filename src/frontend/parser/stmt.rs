use crate::frontend::lexer::token::TokenKind;
use crate::frontend::parser::ast::*;
use crate::frontend::parser::parser::Parser;

impl Parser {
    pub(crate) fn parse_statement(&mut self, scope: ScopeType) -> Result<Option<Stmt>, String> {
        let result: Result<crate::frontend::parser::ast::Stmt, String> = match &self.peek().kind {
            TokenKind::SemiColon => {
                self.advance();
                return Ok(None);
            }
            TokenKind::Import => {
                self.parse_import_stmt().map(Stmt::Declaration)
            }
            TokenKind::Public => {
                self.parse_exported_stmt().map(Stmt::Declaration)
            }
            TokenKind::Extern => self.parse_extern_decl().map(Stmt::Declaration),
            TokenKind::Define => self.parse_define_stmt(false).map(Stmt::Declaration),
            TokenKind::Const => {
                if self.tokens.get(self.current + 1).map(|t| &t.kind) == Some(&TokenKind::LBrace) {
                    self.advance(); // consume 'const'
                    self.parse_object_destructure_decl(Editability::NotEditable, scope)
                } else if self.is_named_destructure_start_at(self.current + 1) {
                    self.advance(); // consume 'const'
                    self.parse_named_destructure_decl(Editability::NotEditable, scope)
                } else {
                    self.parse_const(scope).map(Stmt::Declaration)
                }
            }
            | TokenKind::TypeInt(_)
            | TokenKind::TypeUInt(_)
            | TokenKind::TypeUSize
            | TokenKind::TypeISize
            | TokenKind::TypeFloat(_)
            | TokenKind::TypeChar
            | TokenKind::TypeUChar
            | TokenKind::TypeBool
            | TokenKind::Flag
            | TokenKind::TypeMethod => {
                if let Some(next) = self.tokens.get(self.current + 1) {
                    if let TokenKind::Identifier(_) = next.kind {
                        if let Some(after) = self.tokens.get(self.current + 2) {
                            if after.kind == TokenKind::LParen || after.kind == TokenKind::Less {
                                return self.parse_fn_decl().map(Stmt::Declaration).map(Some);
                            }
                        }
                    }
                }
                self.parse_var_decl(scope).map(Stmt::Declaration)
            }
            | TokenKind::TypeFn
            | TokenKind::TypeLambda
            | TokenKind::TypeNumber
            | TokenKind::TypeType
            | TokenKind::TypeAuto => self.parse_var_decl(scope).map(Stmt::Declaration),

            TokenKind::TypeFunction => {
                if let Some(next) = self.tokens.get(self.current + 1) {
                    if next.kind == TokenKind::DoubleColon {
                        if let Some(sub) = self.tokens.get(self.current + 2) {
                            if matches!(sub.kind, TokenKind::Fn | TokenKind::TypeFn | TokenKind::TypeMethod) {
                                self.advance(); // consume 'function'
                                self.advance(); // consume '::'
                                return self.parse_fn_decl().map(Stmt::Declaration).map(Some);
                            }
                        }
                    } else if let TokenKind::Identifier(_) = next.kind {
                        if let Some(after) = self.tokens.get(self.current + 2) {
                            if after.kind == TokenKind::LParen || after.kind == TokenKind::Less {
                                self.advance(); // consume 'function', identifier is function name
                                return self.parse_fn_decl().map(Stmt::Declaration).map(Some);
                            }
                        }
                    }
                }
                self.parse_var_decl(scope).map(Stmt::Declaration)
            }

            TokenKind::TypeObject => {
                if let Some(next) = self.tokens.get(self.current + 1) {
                    if next.kind == TokenKind::DoubleColon {
                        self.advance(); // consume 'object'
                        self.advance(); // consume '::'
                        match self.peek().kind {
                            TokenKind::TypeStruct => return self.parse_struct_decl().map(Stmt::Declaration).map(Some),
                            TokenKind::TypeClass => return self.parse_class_decl().map(Stmt::Declaration).map(Some),
                            TokenKind::TypeBluePrint => return self.parse_blueprint_decl().map(Stmt::Declaration).map(Some),
                            TokenKind::TypeEnum => return self.parse_enum_decl().map(Stmt::Declaration).map(Some),
                            TokenKind::Identifier(ref id) if id == "namespace" => {
                                self.advance(); // consume 'namespace'
                                return self.parse_namespace_decl().map(Stmt::Declaration).map(Some);
                            }
                            _ => {}
                        }
                    } else if let TokenKind::Identifier(_) = next.kind {
                        if let Some(after) = self.tokens.get(self.current + 2) {
                            if after.kind == TokenKind::DoubleColon {
                                self.advance(); // consume 'object'
                                return self.parse_namespace_decl().map(Stmt::Declaration).map(Some);
                            } else if after.kind == TokenKind::Assign {
                                if let Some(after_assign) = self.tokens.get(self.current + 3) {
                                    if after_assign.kind == TokenKind::LBrace {
                                        self.advance(); // consume 'object'
                                        return self.parse_blueprint_decl().map(Stmt::Declaration).map(Some);
                                    }
                                }
                            } else if after.kind == TokenKind::LBrace {
                                self.advance(); // consume 'object'
                                return self.parse_enum_decl().map(Stmt::Declaration).map(Some);
                            }
                        }
                    }
                }
                self.parse_var_decl(scope).map(Stmt::Declaration)
            }

            TokenKind::Using => self.parse_using_stmt(),
            TokenKind::Set => self.parse_reassign_stmt(),

            | TokenKind::TypeBluePrint
            | TokenKind::Impl
            | TokenKind::Fn
            | TokenKind::Virtual
            | TokenKind::Abstract
            | TokenKind::TypeClass
            | TokenKind::TypeStruct
            | TokenKind::TypeEnum
            | TokenKind::TypeBlock
            | TokenKind::TypeMicro
            | TokenKind::TypeMacro
            | TokenKind::TypeMachine
            | TokenKind::Del => {
                if
                    !matches!(
                        scope,
                        ScopeType::Global |
                            ScopeType::Class |
                            ScopeType::Struct |
                            ScopeType::Blueprint |
                            ScopeType::Handle |
                            ScopeType::Label |
                            ScopeType::Block |
                            ScopeType::Fn |
                            ScopeType::Impl
                    )
                {
                    return Err(
                        format!(
                            "Syntax Error: {:?} declarations are not allowed in this scope",
                            self.peek().kind
                        )
                    );
                }
                match self.peek().kind {
                    TokenKind::TypeBluePrint => self.parse_blueprint_decl().map(Stmt::Declaration),
                    TokenKind::Impl => self.parse_impl_decl().map(Stmt::Declaration),
                    TokenKind::Fn | TokenKind::Virtual | TokenKind::Abstract => {
                        if self.peek().kind == TokenKind::Fn && self.peek_ahead(1).map(|t| t.kind == TokenKind::LParen).unwrap_or(false) {
                            self.parse_expression_stmt()
                        } else {
                            self.parse_fn_decl().map(Stmt::Declaration)
                        }
                    }
                    TokenKind::TypeClass => self.parse_class_decl().map(Stmt::Declaration),
                    TokenKind::TypeStruct => self.parse_struct_decl().map(Stmt::Declaration),
                    TokenKind::TypeEnum => self.parse_enum_decl().map(Stmt::Declaration),
                    TokenKind::TypeBlock => self.parse_block_scope_decl().map(Stmt::Declaration),
                    TokenKind::TypeMicro => self.parse_micro_decl().map(Stmt::Declaration),
                    TokenKind::TypeMacro => self.parse_macro_decl().map(Stmt::Declaration),
                    TokenKind::TypeMachine => self.parse_machine_decl().map(Stmt::Declaration),
                    TokenKind::Del => self.parse_del_stmt(),
                    _ => unreachable!(),
                }
            }

            | TokenKind::If
            | TokenKind::For
            | TokenKind::Loop
            | TokenKind::While
            | TokenKind::Do
            | TokenKind::Match => {
                if
                    !matches!(
                        scope,
                        ScopeType::Global |
                            ScopeType::Fn |
                            ScopeType::Class |
                            ScopeType::Blueprint |
                            ScopeType::Block |
                            ScopeType::Label |
                            ScopeType::Handle
                    )
                {
                    return Err(
                        format!("Syntax Error: {:?} is not allowed in this scope", self.peek().kind)
                    );
                }
                match self.peek().kind {
                    TokenKind::If => self.parse_if_stmt(),
                    TokenKind::For => self.parse_for_stmt(),
                    TokenKind::Loop => self.parse_loop_stmt(),
                    TokenKind::While => self.parse_while_stmt(),
                    TokenKind::Do => self.parse_do_while_stmt(),
                    TokenKind::Match => self.parse_switch_stmt(),
                    _ => unreachable!(),
                }
            }

            TokenKind::Leave => {
                self.advance();
                if self.peek().kind == TokenKind::If {
                    self.advance();
                    let cond = self.parse_postfix_condition()?;
                    self.consume(TokenKind::SemiColon, "Expected ';' after postfix if")?;
                    return Ok(Some(Stmt::IfStmt {
                        condition: cond,
                        then_block: vec![Stmt::LeaveStmt],
                        else_block: None,
                    }));
                }
                self.consume(TokenKind::SemiColon, "Expected ';' after leave")?;
                Ok(Stmt::LeaveStmt)
            }

            TokenKind::Yield => { self.parse_yield_stmt() }
            TokenKind::Break | TokenKind::Continue => {
                let kind = self.peek().kind.clone();
                self.advance();
                let base_stmt = if kind == TokenKind::Break {
                    Stmt::BreakStmt
                } else {
                    Stmt::ContinueStmt
                };
                if self.peek().kind == TokenKind::If {
                    self.advance();
                    let cond = self.parse_postfix_condition()?;
                    self.consume(TokenKind::SemiColon, "Expected ';' after postfix if")?;
                    return Ok(Some(Stmt::IfStmt {
                        condition: cond,
                        then_block: vec![base_stmt],
                        else_block: None,
                    }));
                }
                self.consume(
                    TokenKind::SemiColon,
                    format!("Expected ';' after {:?}", kind).as_str()
                )?;
                Ok(base_stmt)
            }

            TokenKind::Return => {
                self.advance();
                let mut ret_val = None;
                if self.peek().kind != TokenKind::SemiColon && self.peek().kind != TokenKind::If {
                    ret_val = Some(self.parse_expression()?);
                }
                if self.peek().kind == TokenKind::If {
                    self.advance();
                    let cond = self.parse_postfix_condition()?;
                    self.consume(TokenKind::SemiColon, "Expected ';' after postfix if")?;
                    return Ok(Some(Stmt::IfStmt {
                        condition: cond,
                        then_block: vec![Stmt::ReturnStmt(ret_val)],
                        else_block: None,
                    }));
                }
                if self.peek().kind == TokenKind::SemiColon {
                    self.advance();
                }
                Ok(Stmt::ReturnStmt(ret_val))
            }

            TokenKind::Throw => self.parse_throw_stmt(),
            TokenKind::Try => self.parse_try_catch_stmt(),

            TokenKind::LabelName(lbl) if lbl.eq_ignore_ascii_case("@compile") => {
                if self.is_compile_validation_ahead() {
                    self.advance(); // consume '@compile'
                    if self.peek().kind == TokenKind::Arrow {
                        self.advance();
                    }
                    self.consume(TokenKind::LBrace, "Expected '{' to open @compile validation body")?;
                    let mut body = Vec::new();
                    while !self.is_at_end() && self.peek().kind != TokenKind::RBrace {
                        if let Some(s) = self.parse_statement(ScopeType::Block)? {
                            body.push(s);
                        }
                    }
                    self.consume(TokenKind::RBrace, "Expected '}' after @compile validation body")?;
                    self.consume(TokenKind::LParen, "Expected '(' after @compile validation block")?;
                    let mut args = Vec::new();
                    if self.peek().kind != TokenKind::RParen {
                        args.push(self.parse_expression()?);
                        while self.peek().kind == TokenKind::Comma {
                            self.advance();
                            args.push(self.parse_expression()?);
                        }
                    }
                    self.consume(TokenKind::RParen, "Expected ')' after @compile validation arguments")?;
                    if self.peek().kind == TokenKind::SemiColon {
                        self.advance();
                    }
                    Ok(Stmt::CompileValidation { body, args })
                } else if self.peek_ahead(1).map(|t| &t.kind) == Some(&TokenKind::Arrow)
                    || self.peek_ahead(1).map(|t| &t.kind) == Some(&TokenKind::LBrace)
                    || (self.peek_ahead(1).map(|t| &t.kind) == Some(&TokenKind::DoubleColon)
                        && self.peek_ahead(2).map(|t| &t.kind) == Some(&TokenKind::LBrace))
                {
                    self.parse_compile_decl().map(Stmt::Declaration)
                } else {
                    self.parse_expression_or_reassignment()
                }
            }

            TokenKind::Identifier(_) if self.is_namespace_decl_start() => {
                self.parse_namespace_decl().map(Stmt::Declaration)
            }

            TokenKind::Identifier(_) if self.is_named_destructure_start() => {
                self.parse_named_destructure_decl(Editability::Editable, scope)
            }
            TokenKind::Identifier(_) if self.is_var_decl_start() => {
                self.parse_var_decl(scope).map(Stmt::Declaration)
            }
            TokenKind::Identifier(_) if self.is_block_decl_start() => {
                self.parse_bare_block_decl().map(Stmt::Declaration)
            }
            TokenKind::Identifier(_) => self.parse_expression_or_reassignment(),

            TokenKind::This if self.peek_ahead(1).map(|t| &t.kind) == Some(&TokenKind::LBrace) => {
                self.advance(); // consume 'this'
                self.advance(); // consume '{'
                let mut stmts = Vec::new();
                while !self.is_at_end() && self.peek().kind != TokenKind::RBrace {
                    if let Some(s) = self.parse_statement(scope.clone())? {
                        stmts.push(s);
                    }
                }
                self.consume(TokenKind::RBrace, "Expected '}' after this scope block")?;
                if self.peek().kind == TokenKind::SemiColon {
                    self.advance();
                }
                Ok(Stmt::ThisBlock(stmts))
            }
            TokenKind::This => { self.parse_expression_or_reassignment() }

            TokenKind::Super => {
                if
                    !matches!(
                        scope,
                        ScopeType::Class |
                            ScopeType::Struct |
                            ScopeType::Blueprint |
                            ScopeType::Handle |
                            ScopeType::Label
                    )
                {
                    return Err("Syntax Error: 'super' is not allowed in this scope".to_string());
                }
                self.parse_expression_or_reassignment()
            }

            TokenKind::Goto => {
                if
                    !matches!(
                        scope,
                        ScopeType::Handle |
                            ScopeType::Label |
                            ScopeType::Fn |
                            ScopeType::Blueprint |
                            ScopeType::Block
                    )
                {
                    return Err(
                        "Syntax Error: Goto statements are not allowed in this scope".to_string()
                    );
                }
                self.parse_goto_stmt()
            }
            TokenKind::LBrace if self.is_object_destructure_start() => {
                self.parse_object_destructure_decl(Editability::Editable, scope)
            }
            TokenKind::LBrace => {
                self.advance(); // consume '{'
                let blk = self.parse_block("block".to_string())?;
                self.consume(TokenKind::RBrace, "Expected '}' to close block")?;
                if self.peek().kind == TokenKind::If {
                    self.advance(); // consume 'if'
                    let cond = self.parse_postfix_condition()?;
                    if self.peek().kind == TokenKind::SemiColon {
                        self.advance();
                    }
                    Ok(Stmt::IfStmt {
                        condition: cond,
                        then_block: blk,
                        else_block: None,
                    })
                } else {
                    if self.peek().kind == TokenKind::SemiColon {
                        self.advance();
                    }
                    Ok(Stmt::Block(blk))
                }
            }
            _ => self.parse_expression_stmt(),
        };

        match result {
            Ok(stmt) => Ok(Some(stmt)),
            Err(err) => {
                let err_str = format!("{}", err);
                let err_msg: String = if err_str.starts_with("Syntax Error:") {
                    err_str.clone()
                } else {
                    format!("Syntax Error: {}", err_str)
                };
                eprintln!("{}", err_msg);
                self.synchronize();
                Err(err_msg)
            }
        }
    }
    pub(crate) fn parse_import_stmt(&mut self) -> Result<Decl, String> {
        self.advance(); // consume 'import'

        // Check for ABI prefix e.g. import @c "stdio.h"; or import @cpp "vector";
        let mut abi: Option<String> = None;
        if let TokenKind::LabelName(lbl) = &self.peek().kind {
            if lbl.eq_ignore_ascii_case("@c") {
                self.advance();
                abi = Some("C".to_string());
            } else if lbl.eq_ignore_ascii_case("@cpp") {
                self.advance();
                abi = Some("Cpp".to_string());
            }
        }

        // Case 1: String import (e.g. import "stdio.h" or import "std/string")
        if let TokenKind::String(path) = &self.peek().kind {
            let mod_path = path.clone();
            self.advance();

            // Check if ABI is specified at the end e.g. import "stdio.h" @c;
            if abi.is_none() {
                if let TokenKind::LabelName(lbl) = &self.peek().kind {
                    if lbl.eq_ignore_ascii_case("@c") {
                        self.advance();
                        abi = Some("C".to_string());
                    } else if lbl.eq_ignore_ascii_case("@cpp") {
                        self.advance();
                        abi = Some("Cpp".to_string());
                    }
                }
            }

            let alias = if self.peek().kind == TokenKind::As {
                self.advance();
                Some(self.get_identifier("Expected alias identifier after 'as'")?)
            } else {
                None
            };

            self.consume(TokenKind::SemiColon, "Expected ';' after import statement")?;
            return Ok(Decl::Import {
                module_path: vec![mod_path],
                imports: None,
                abi,
                alias,
            });
        }

        let mut module_path: Vec<String> = Vec::new();
        let mut imports: Option<Vec<String>> = None;

        module_path.push(self.get_identifier("Expected module name after 'import'")?);

        while self.peek().kind == TokenKind::DoubleColon {
            self.advance();

            if self.peek().kind == TokenKind::LBrace {
                self.advance();
                let mut selected: Vec<String> = Vec::new();

                if !self.is_at_end() && self.peek().kind != TokenKind::RBrace {
                    loop {
                        selected.push(self.get_identifier("Expected import name in import list")?);

                        if self.peek().kind == TokenKind::Comma {
                            self.advance();
                            continue;
                        }
                        break;
                    }
                }

                self.consume(TokenKind::RBrace, "Expected '}' after import list")?;

                imports = Some(selected);
                break;
            } else {
                module_path.push(self.get_identifier("Expected module name after '::'")?);
            }
        }

        let alias = if self.peek().kind == TokenKind::As {
            self.advance();
            Some(self.get_identifier("Expected alias identifier after 'as'")?)
        } else {
            None
        };

        self.consume(TokenKind::SemiColon, "Expected ';' after import statement")?;

        Ok(Decl::Import {
            module_path,
            imports,
            abi,
            alias,
        })
    }

    pub(crate) fn parse_extern_decl(&mut self) -> Result<Decl, String> {
        self.consume(TokenKind::Extern, "Expected 'extern'")?;
        let abi = match &self.peek().kind {
            TokenKind::LabelName(lbl) if lbl.eq_ignore_ascii_case("@c") => {
                self.advance();
                "C".to_string()
            }
            TokenKind::LabelName(lbl) if lbl.eq_ignore_ascii_case("@cpp") => {
                self.advance();
                "Cpp".to_string()
            }
            _ => "C".to_string(),
        };

        if self.peek().kind == TokenKind::LBrace {
            self.advance(); // consume '{'
            let mut decls = Vec::new();
            while !self.is_at_end() && self.peek().kind != TokenKind::RBrace {
                if self.peek().kind == TokenKind::Fn {
                    let fn_decl = self.parse_extern_fn_signature(&abi)?;
                    decls.push(fn_decl);
                } else {
                    return Err(format!(
                        "Syntax Error: Unexpected token '{:?}' in extern block",
                        self.peek().kind
                    ));
                }
            }
            self.consume(TokenKind::RBrace, "Expected '}' at end of extern block")?;
            Ok(Decl::ExternBlockDecl { abi, decls })
        } else if self.peek().kind == TokenKind::Fn {
            self.parse_extern_fn_signature(&abi)
        } else {
            Err(format!(
                "Syntax Error: Expected '{{' or 'fn' after extern, found '{:?}'",
                self.peek().kind
            ))
        }
    }

    pub(crate) fn parse_single_param(&mut self) -> Result<Param, String> {
        let mut is_variadic = if self.peek().kind == TokenKind::DotDotDot {
            self.advance(); // consume '...'
            true
        } else {
            false
        };

        let is_anonymous = if is_variadic {
            let next_kind = self.peek_ahead(1).map(|t| &t.kind);
            next_kind != Some(&TokenKind::Colon) && next_kind != Some(&TokenKind::Walrus)
        } else {
            match &self.peek().kind {
                TokenKind::TypeInt(_)
                | TokenKind::TypeUInt(_)
                | TokenKind::TypeFloat(_)
                | TokenKind::TypeUSize
                | TokenKind::TypeISize
                | TokenKind::TypeChar
                | TokenKind::TypeUChar
                | TokenKind::TypeBool
                | TokenKind::TypeVoid
                | TokenKind::TypeType => {
                    let next_kind = self.peek_ahead(1).map(|t| &t.kind);
                    next_kind != Some(&TokenKind::Colon) && next_kind != Some(&TokenKind::Walrus)
                }
                TokenKind::Identifier(_) => {
                    let next_kind = self.peek_ahead(1).map(|t| &t.kind);
                    next_kind != Some(&TokenKind::Colon) && next_kind != Some(&TokenKind::Walrus)
                }
                _ => false,
            }
        };

        if is_anonymous {
            let anon_idx = self.current;
            let type_node = self.parse_type()?;
            return Ok(Param {
                name: format!("__anon_param_{}", anon_idx),
                type_node,
                default_value: None,
                is_variadic,
            });
        }

        let err_msg = format!(
            "Expected parameter name at line {}, column {}, found '{:?}'",
            self.peek().line,
            self.peek().column,
            self.peek().kind
        );
        let param_name = self.get_identifier(&err_msg)?;
        let (type_node, default_value) = if self.peek().kind == TokenKind::Walrus {
            self.advance(); // consume ':='
            let val_expr = self.parse_expression()?;
            let t = type_from_expr(&val_expr);
            (t, Some(val_expr))
        } else if self.peek().kind == TokenKind::Colon {
            self.advance(); // consume ':'
            if self.peek().kind == TokenKind::DotDotDot {
                self.advance(); // consume '...' if written like args: ...T
                is_variadic = true;
            }
            let t = self.parse_type()?;
            let default_val =
                if self.peek().kind == TokenKind::Assign || self.peek().kind == TokenKind::Walrus {
                    self.advance();
                    Some(self.parse_expression()?)
                } else {
                    None
                };
            (t, default_val)
        } else {
            return Err("Expected ':' or ':=' after parameter name".to_string());
        };
        Ok(Param {
            name: param_name,
            type_node,
            default_value,
            is_variadic,
        })
    }

    pub(crate) fn parse_extern_fn_signature(&mut self, abi: &str) -> Result<Decl, String> {
        self.consume(TokenKind::Fn, "Expected 'fn'")?;
        let name = self.get_identifier("Expected function name in extern declaration")?;
        let mut generics: Vec<BaseType> = Vec::new();
        if self.peek().kind == TokenKind::Less {
            self.parse_generics(&mut generics)?;
        }
        self.consume(TokenKind::LParen, "Expected '(' after function name")?;
        let mut params = Vec::new();
        if self.peek().kind != TokenKind::RParen {
            loop {
                let p = self.parse_single_param()?;
                params.push(p);
                if self.peek().kind == TokenKind::Comma {
                    self.advance();
                    continue;
                }
                break;
            }
        }
        self.consume(TokenKind::RParen, "Expected ')' after parameters")?;
        let return_type = if self.peek().kind == TokenKind::Arrow {
            self.advance();
            self.parse_type()?
        } else {
            BaseType::Void
        };
        let alias = if self.peek().kind == TokenKind::As {
            self.advance();
            Some(self.get_identifier("Expected alias identifier after 'as'")?)
        } else {
            None
        };
        self.consume(
            TokenKind::SemiColon,
            "Expected ';' after extern function declaration",
        )?;
        Ok(Decl::ExternFnDecl {
            abi: abi.to_string(),
            name,
            generics,
            params,
            return_type,
            alias,
        })
    }

    pub(crate) fn parse_using_stmt(&mut self) -> Result<Stmt, String> {
        self.consume(TokenKind::Using, "Expected 'using'")?;
        let name = self.get_identifier("Expected identifier after 'using'")?;
        self.consume(TokenKind::SemiColon, "Expected ';' after using statement")?;
        Ok(Stmt::UsingStmt(name))
    }

    pub(crate) fn parse_define_stmt(&mut self, is_public: bool) -> Result<Decl, String> {
        self.consume(TokenKind::Define, "Expected 'define'")?;
        let name = match &self.peek().kind {
            TokenKind::Identifier(id) => {
                let s = id.clone();
                self.advance();
                s
            }
            _ => return Err("Expected identifier after 'define'".to_string()),
        };
        if self.peek().kind == TokenKind::Assign || self.peek().kind == TokenKind::Arrow {
            self.advance();
        } else {
            return Err("Expected '=' or '->' after define identifier".to_string());
        }

        let is_type = match &self.peek().kind {
            TokenKind::TypeInt(_)
            | TokenKind::TypeUInt(_)
            | TokenKind::TypeUSize
            | TokenKind::TypeISize
            | TokenKind::TypeFloat(_)
            | TokenKind::TypeChar
            | TokenKind::TypeUChar
            | TokenKind::TypeBool
            | TokenKind::TypeVoid
            | TokenKind::TypeStruct
            | TokenKind::TypeClass
            | TokenKind::TypeBluePrint
            | TokenKind::TypeEnum
            | TokenKind::TypeMethod
            | TokenKind::TypeFn
            | TokenKind::TypeMicro
            | TokenKind::TypeLambda
            | TokenKind::TypeAuto
            | TokenKind::TypeBlock => true,
            TokenKind::Identifier(id) => {
                self.peek_ahead(1)
                    .map(|t| t.kind == TokenKind::Less || t.kind == TokenKind::LBracket)
                    .unwrap_or(false)
                    || self.metadata.contains_key(id)
            }
            _ => false,
        };

        let visibility = if is_public {
            Visibility::Public
        } else {
            Visibility::Private
        };

        if is_type {
            let t = self.parse_type()?;
            self.type_aliases.insert(name.clone(), t.clone());
            self.consume(TokenKind::SemiColon, "Expected ';' after define type alias")?;
            Ok(Decl::DefineDecl {
                visibility,
                name,
                type_alias: Some(t),
                value: None,
            })
        } else {
            let val = self.parse_expression()?;
            self.consume(TokenKind::SemiColon, "Expected ';' after define value")?;
            Ok(Decl::DefineDecl {
                visibility,
                name,
                type_alias: None,
                value: Some(val),
            })
        }
    }

    pub(crate) fn parse_const(&mut self, scope: ScopeType) -> Result<Decl, String> {
        self.advance(); // consume 'const'

        let mut decl = match &self.peek().kind {
            TokenKind::TypeInt(_)
            | TokenKind::TypeUInt(_)
            | TokenKind::TypeUSize
            | TokenKind::TypeISize
            | TokenKind::TypeFloat(_)
            | TokenKind::TypeChar
            | TokenKind::TypeUChar
            | TokenKind::TypeBool
            | TokenKind::TypeType
            | TokenKind::Identifier(_) => self.parse_var_decl(scope)?,
            _ => {
                return Err("Syntax Error: Expected variable declaration".to_string());
            }
        };

        match &mut decl {
            Decl::VarDecl {
                ref mut editability,
                ..
            } => {
                *editability = Editability::NotEditable;
            }
            Decl::DestructureDecl {
                ref mut editability,
                ..
            } => {
                *editability = Editability::NotEditable;
            }
            Decl::ArrayDecl {
                ref mut editability,
                ..
            } => {
                *editability = Editability::NotEditable;
            }
            _ => {
                return Err("Syntax Error: Only variables can be declared as const".to_string());
            }
        }

        Ok(decl)
    }
    pub(crate) fn parse_exported_stmt(&mut self) -> Result<Decl, String> {
        self.advance(); // consume 'export' or 'public'

        // After export/public, we expect a valid exportable statement (fn, block, class, struct, enum, micro, blueprint, machine, define, or var)
        let mut stmt = match &self.peek().kind {
            TokenKind::Fn | TokenKind::TypeMethod => self.parse_fn_decl()?,
            TokenKind::TypeClass => self.parse_class_decl()?,
            TokenKind::TypeStruct => self.parse_struct_decl()?,
            TokenKind::TypeEnum => self.parse_enum_decl()?,
            TokenKind::TypeMicro => self.parse_micro_decl()?,
            TokenKind::TypeMacro => self.parse_macro_decl()?,
            TokenKind::TypeBluePrint => self.parse_blueprint_decl()?,
            TokenKind::TypeMachine => self.parse_machine_decl()?,
            TokenKind::TypeBlock => self.parse_block_scope_decl()?,
            TokenKind::Define => self.parse_define_stmt(true)?,
            TokenKind::Impl => self.parse_impl_decl()?,
            TokenKind::LabelName(lbl) if lbl.eq_ignore_ascii_case("@compile") => {
                self.parse_compile_decl()?
            }
            TokenKind::Identifier(_) if self.is_namespace_decl_start() => {
                self.parse_namespace_decl()?
            }
            TokenKind::Identifier(_) if self.is_block_decl_start() => {
                self.parse_bare_block_decl()?
            }
            _ if self.is_var_decl_start() => self.parse_var_decl(ScopeType::Global)?,
            kind => {
                return Err(
                    format!(
                        "Syntax Error: Cannot make '{:?}' public, only fn, block, class, struct, enum, blueprint, machine, define, micro, impl, and variables can be public",
                        kind
                    )
                );
            }
        };

        // Set visibility to Public
        match &mut stmt {
            Decl::FnDecl {
                ref mut visibility, ..
            } => {
                *visibility = Visibility::Public;
            }
            Decl::BlockDecl {
                ref mut visibility, ..
            } => {
                *visibility = Visibility::Public;
            }
            Decl::ClassDecl {
                ref mut visibility, ..
            } => {
                *visibility = Visibility::Public;
            }
            Decl::StructDecl {
                ref mut visibility, ..
            } => {
                *visibility = Visibility::Public;
            }
            Decl::BlueprintDecl {
                ref mut visibility, ..
            } => {
                *visibility = Visibility::Public;
            }
            Decl::EnumDecl {
                ref mut visibility, ..
            } => {
                *visibility = Visibility::Public;
            }
            Decl::MicroDecl {
                ref mut visibility, ..
            } => {
                *visibility = Visibility::Public;
            }
            Decl::MacroDecl {
                ref mut visibility, ..
            } => {
                *visibility = Visibility::Public;
            }
            Decl::NamespaceDecl {
                ref mut visibility, ..
            } => {
                *visibility = Visibility::Public;
            }
            Decl::MachineDecl {
                ref mut visibility, ..
            } => {
                *visibility = Visibility::Public;
            }
            Decl::DefineDecl {
                ref mut visibility, ..
            } => {
                *visibility = Visibility::Public;
            }
            Decl::CompileDecl { ref mut decls, .. } => {
                for d in decls {
                    match d {
                        Decl::FnDecl {
                            ref mut visibility, ..
                        }
                        | Decl::BlockDecl {
                            ref mut visibility, ..
                        }
                        | Decl::ClassDecl {
                            ref mut visibility, ..
                        }
                        | Decl::StructDecl {
                            ref mut visibility, ..
                        }
                        | Decl::BlueprintDecl {
                            ref mut visibility, ..
                        }
                        | Decl::EnumDecl {
                            ref mut visibility, ..
                        }
                        | Decl::MicroDecl {
                            ref mut visibility, ..
                        }
                        | Decl::MacroDecl {
                            ref mut visibility, ..
                        }
                        | Decl::MachineDecl {
                            ref mut visibility, ..
                        }
                        | Decl::DefineDecl {
                            ref mut visibility, ..
                        }
                        | Decl::VarDecl {
                            ref mut visibility, ..
                        }
                        | Decl::DestructureDecl {
                            ref mut visibility, ..
                        } => {
                            *visibility = Visibility::Public;
                        }
                        _ => {}
                    }
                }
            }
            Decl::VarDecl {
                ref mut visibility, ..
            } => {
                *visibility = Visibility::Public;
            }
            Decl::ImplDecl { .. } => {}
            Decl::DestructureDecl {
                ref mut visibility, ..
            } => {
                *visibility = Visibility::Public;
            }
            Decl::ArrayDecl {
                ref mut visibility, ..
            } => {
                *visibility = Visibility::Public;
            }
            Decl::ObjectDecl {
                ref mut visibility, ..
            } => {
                *visibility = Visibility::Public;
            }
            _ => {}
        }

        Ok(stmt)
    }

    pub(crate) fn consume_optional_let(&mut self) {
        if let TokenKind::Identifier(name) = &self.peek().kind {
            if name == "let" {
                self.advance();
            }
        }
    }

    pub(crate) fn is_var_decl_start(&self) -> bool {
        match &self.peek().kind {
            | TokenKind::Const
            | TokenKind::TypeInt(_)
            | TokenKind::TypeUInt(_)
            | TokenKind::TypeUSize
            | TokenKind::TypeISize
            | TokenKind::TypeFloat(_)
            | TokenKind::TypeChar
            | TokenKind::TypeUChar
            | TokenKind::TypeBool
            | TokenKind::Flag
            //| TokenKind::Scope
            | TokenKind::TypeType
            | TokenKind::TypeMethod => {
                if let Some(next) = self.tokens.get(self.current + 1) {
                    if let TokenKind::Identifier(_) = next.kind {
                        if let Some(after) = self.tokens.get(self.current + 2) {
                            if after.kind == TokenKind::LParen || after.kind == TokenKind::Less {
                                return false;
                            }
                        }
                    }
                }
                true
            }
            | TokenKind::TypeFn
            | TokenKind::TypeLambda
            | TokenKind::TypeAuto => true,
            TokenKind::Identifier(_) => {
                if let Some(next) = self.tokens.get(self.current + 1) {
                    if next.kind == TokenKind::Walrus {
                        return true;
                    }
                }
                self.is_custom_type_var_decl_start_at(self.current)
            }
            _ => false,
        }
    }

    pub(crate) fn is_custom_type_var_decl_start_at(&self, start: usize) -> bool {
        let mut idx = start;
        if idx >= self.tokens.len() {
            return false;
        }
        let first_name = match &self.tokens[idx].kind {
            TokenKind::Identifier(name) => name.clone(),
            _ => return false,
        };
        if self.macro_metadata.contains(&first_name) {
            return false;
        }
        idx += 1;
        while idx < self.tokens.len() && self.tokens[idx].kind == TokenKind::DoubleColon {
            idx += 1;
            if idx >= self.tokens.len()
                || !matches!(&self.tokens[idx].kind, TokenKind::Identifier(_))
            {
                return false;
            }
            idx += 1;
        }
        if idx >= self.tokens.len() {
            return false;
        }
        if matches!(&self.tokens[idx].kind, TokenKind::Ampersand | TokenKind::Multiply) {
            while idx < self.tokens.len()
                && matches!(&self.tokens[idx].kind, TokenKind::Ampersand | TokenKind::Multiply)
            {
                idx += 1;
            }
            if idx < self.tokens.len() && matches!(&self.tokens[idx].kind, TokenKind::Identifier(_)) {
                return true;
            }
            return false;
        }

        if matches!(&self.tokens[idx].kind, TokenKind::Identifier(_)) {
            if let Some(after) = self.tokens.get(idx + 1) {
                if after.kind == TokenKind::Comma {
                    return false;
                }
            }
            return true;
        }

        if self.tokens[idx].kind == TokenKind::Less {
            let mut depth = 0;
            while idx < self.tokens.len() {
                if self.tokens[idx].kind == TokenKind::Less {
                    depth += 1;
                } else if self.tokens[idx].kind == TokenKind::Greater {
                    depth -= 1;
                    if depth == 0 {
                        idx += 1;
                        break;
                    }
                } else if matches!(
                    self.tokens[idx].kind,
                    TokenKind::SemiColon | TokenKind::Assign | TokenKind::Arrow | TokenKind::EOF
                ) {
                    return false;
                }
                idx += 1;
            }
            if depth != 0 || idx >= self.tokens.len() {
                return false;
            }
            if matches!(&self.tokens[idx].kind, TokenKind::Identifier(_)) {
                return true;
            }
        }

        if self.tokens[idx].kind == TokenKind::LBracket {
            while idx < self.tokens.len() && self.tokens[idx].kind == TokenKind::LBracket {
                let mut depth = 0;
                while idx < self.tokens.len() {
                    if self.tokens[idx].kind == TokenKind::LBracket {
                        depth += 1;
                    } else if self.tokens[idx].kind == TokenKind::RBracket {
                        depth -= 1;
                        if depth == 0 {
                            idx += 1;
                            break;
                        }
                    } else if matches!(
                        self.tokens[idx].kind,
                        TokenKind::SemiColon
                            | TokenKind::Assign
                            | TokenKind::Arrow
                            | TokenKind::EOF
                    ) {
                        return false;
                    }
                    idx += 1;
                }
                if depth != 0 {
                    return false;
                }
            }
            if idx < self.tokens.len() && matches!(&self.tokens[idx].kind, TokenKind::Identifier(_))
            {
                return true;
            }
        }

        false
    }

    pub(crate) fn is_named_destructure_start_at(&self, start: usize) -> bool {
        let mut idx = start;
        if idx >= self.tokens.len() {
            return false;
        }
        if !matches!(&self.tokens[idx].kind, TokenKind::Identifier(_)) {
            return false;
        }
        idx += 1;
        while idx < self.tokens.len() && self.tokens[idx].kind == TokenKind::DoubleColon {
            idx += 1;
            if idx >= self.tokens.len()
                || !matches!(&self.tokens[idx].kind, TokenKind::Identifier(_))
            {
                return false;
            }
            idx += 1;
        }
        if idx >= self.tokens.len() {
            return false;
        }
        let open_kind = &self.tokens[idx].kind;
        if *open_kind != TokenKind::LBrace && *open_kind != TokenKind::LParen {
            return false;
        }
        let (open_tok, close_tok) = if *open_kind == TokenKind::LBrace {
            (TokenKind::LBrace, TokenKind::RBrace)
        } else {
            (TokenKind::LParen, TokenKind::RParen)
        };
        let mut depth = 0;
        while idx < self.tokens.len() {
            if self.tokens[idx].kind == open_tok {
                depth += 1;
            } else if self.tokens[idx].kind == close_tok {
                depth -= 1;
                if depth == 0 {
                    let next = self.tokens.get(idx + 1).map(|t| &t.kind);
                    return matches!(next, Some(TokenKind::Assign) | Some(TokenKind::Arrow));
                }
            } else if self.tokens[idx].kind == TokenKind::EOF {
                break;
            }
            idx += 1;
        }
        false
    }

    pub(crate) fn is_named_destructure_start(&self) -> bool {
        self.is_named_destructure_start_at(self.current)
    }

    pub(crate) fn is_namespace_decl_start(&self) -> bool {
        if let TokenKind::Identifier(_) = &self.peek().kind {
            if let Some(next) = self.peek_ahead(1) {
                if next.kind == TokenKind::DoubleColon {
                    if let Some(after) = self.peek_ahead(2) {
                        return after.kind == TokenKind::LBrace;
                    }
                }
            }
        }
        false
    }

    pub(crate) fn parse_namespace_decl(&mut self) -> Result<Decl, String> {
        let name = self.get_identifier("Expected namespace name")?;
        self.consume(TokenKind::DoubleColon, "Expected '::' after namespace name")?;
        self.consume(TokenKind::LBrace, "Expected '{' to open namespace body")?;
        let mut decls = Vec::new();
        while !self.is_at_end() && self.peek().kind != TokenKind::RBrace {
            let start_pos = self.current;
            match self.parse_statement(ScopeType::Global)? {
                Some(Stmt::Declaration(d)) => decls.push(d),
                Some(_) => {
                    return Err(
                        "Syntax Error: Only declarations (functions, types, namespaces, micros, variables) are allowed directly inside a namespace".to_string(),
                    );
                }
                None => {
                    if self.current == start_pos && !self.is_at_end() && self.peek().kind != TokenKind::RBrace {
                        self.advance();
                    }
                }
            }
        }
        self.consume(TokenKind::RBrace, "Expected '}' to close namespace body")?;
        if self.peek().kind == TokenKind::SemiColon {
            self.advance();
        }
        Ok(Decl::NamespaceDecl {
            visibility: Visibility::Private,
            name,
            decls,
        })
    }

    pub(crate) fn is_block_decl_start(&self) -> bool {
        if let TokenKind::Identifier(_) = &self.peek().kind {
            if let Some(next) = self.peek_ahead(1) {
                if next.kind == TokenKind::LBrace {
                    return true;
                }
                if next.kind == TokenKind::Arrow {
                    let mut i = 2;
                    while let Some(tok) = self.peek_ahead(i) {
                        if tok.kind == TokenKind::LBrace {
                            return true;
                        }
                        if tok.kind == TokenKind::SemiColon
                            || tok.kind == TokenKind::Assign
                            || tok.kind == TokenKind::EOF
                        {
                            return false;
                        }
                        i += 1;
                    }
                }
            }
        }
        false
    }

    pub(crate) fn parse_bare_block_decl(&mut self) -> Result<Decl, String> {
        let name = self.get_identifier("Expected block name")?;
        let mut return_type = None;
        if self.peek().kind == TokenKind::Arrow {
            self.advance(); // consume '->'
            if self.peek().kind != TokenKind::LBrace {
                return_type = Some(self.parse_type()?);
            }
        }
        self.consume(TokenKind::LBrace, "Expected '{' to open block body")?;
        let mut statements = Vec::new();
        while !self.is_at_end() && self.peek().kind != TokenKind::RBrace {
            match self.parse_statement(ScopeType::Block) {
                Ok(Some(stmt)) => statements.push(stmt),
                Ok(None) => {
                    if !self.is_at_end() && self.peek().kind != TokenKind::RBrace {
                        self.advance();
                    }
                }
                Err(err) => return Err(err),
            }
        }
        self.consume(TokenKind::RBrace, "Expected '}' to close block body")?;
        if self.peek().kind == TokenKind::SemiColon {
            self.advance();
        }
        Ok(Decl::BlockDecl {
            visibility: Visibility::Private,
            name,
            return_type,
            statements,
        })
    }

    pub(crate) fn parse_named_destructure_decl(
        &mut self,
        editability: Editability,
        scope: ScopeType,
    ) -> Result<Stmt, String> {
        let mut type_name = self.get_identifier("Expected type name in destructure")?;
        while self.peek().kind == TokenKind::DoubleColon {
            self.advance();
            let sub = self.get_identifier("Expected identifier after '::'")?;
            type_name.push_str("::");
            type_name.push_str(&sub);
        }
        let is_tuple = self.peek().kind == TokenKind::LParen;
        if is_tuple {
            self.consume(TokenKind::LParen, "Expected '(' in tuple destructure")?;
        } else {
            self.consume(TokenKind::LBrace, "Expected '{' in struct destructure")?;
        }
        let close_kind = if is_tuple {
            TokenKind::RParen
        } else {
            TokenKind::RBrace
        };
        let mut fields = Vec::new();
        while !self.is_at_end() && self.peek().kind != close_kind {
            if self.peek().kind == TokenKind::DotDot {
                self.advance(); // consume '..'
                if self.peek().kind == TokenKind::SemiColon || self.peek().kind == TokenKind::Comma
                {
                    self.advance();
                }
                continue;
            }
            let (type_node, name) = if self.is_var_decl_start() {
                let ty = self.parse_type()?;
                let n = self.get_identifier("Expected variable name in destructure")?;
                (ty, n)
            } else {
                let n = self.get_identifier("Expected variable name in destructure")?;
                (BaseType::Unknown, n)
            };
            if self.peek().kind == TokenKind::SemiColon || self.peek().kind == TokenKind::Comma {
                self.advance();
            }
            let var_meta = VarMetadata {
                name: name.clone(),
                type_node: type_node.clone(),
                visibility: Visibility::Private,
                editability: editability.clone(),
                scope: scope.clone(),
                is_array: false,
            };
            self.var_metadata.insert(name.clone(), var_meta);
            fields.push((type_node, name));
        }
        self.consume(
            close_kind,
            "Expected closing brace/paren after destructure fields",
        )?;
        self.consume(TokenKind::Assign, "Expected '=' after destructure pattern")?;
        let rhs = self.parse_expression()?;
        if self.peek().kind == TokenKind::SemiColon {
            self.advance();
        }
        Ok(Stmt::Declaration(Decl::ObjectDestructureDecl {
            visibility: Visibility::Private,
            editability,
            type_name: Some(type_name),
            fields,
            rhs,
        }))
    }

    pub(crate) fn is_object_destructure_start(&self) -> bool {
        if self.peek().kind != TokenKind::LBrace {
            return false;
        }
        let mut depth = 0;
        let mut idx = self.current;
        while idx < self.tokens.len() {
            match &self.tokens[idx].kind {
                TokenKind::LBrace => {
                    depth += 1;
                }
                TokenKind::RBrace => {
                    depth -= 1;
                    if depth == 0 {
                        let next = self.tokens.get(idx + 1).map(|t| &t.kind);
                        return matches!(next, Some(TokenKind::Assign) | Some(TokenKind::Arrow));
                    }
                }
                TokenKind::EOF => {
                    break;
                }
                _ => {}
            }
            idx += 1;
        }
        false
    }

    pub(crate) fn parse_object_destructure_decl(
        &mut self,
        editability: Editability,
        scope: ScopeType,
    ) -> Result<Stmt, String> {
        self.consume(
            TokenKind::LBrace,
            "Expected '{' to start object destructure",
        )?;
        let mut fields = Vec::new();
        while !self.is_at_end() && self.peek().kind != TokenKind::RBrace {
            if self.peek().kind == TokenKind::DotDot {
                self.advance(); // consume '..'
                if self.peek().kind == TokenKind::SemiColon || self.peek().kind == TokenKind::Comma
                {
                    self.advance();
                }
                continue;
            }
            let (type_node, name) = if self.is_var_decl_start() {
                let ty = self.parse_type()?;
                let n = self.get_identifier("Expected variable name in destructure")?;
                (ty, n)
            } else {
                let n = self.get_identifier("Expected variable name in destructure")?;
                (BaseType::Unknown, n)
            };
            if self.peek().kind == TokenKind::SemiColon || self.peek().kind == TokenKind::Comma {
                self.advance();
            }
            let var_meta = VarMetadata {
                name: name.clone(),
                type_node: type_node.clone(),
                visibility: Visibility::Private,
                editability: editability.clone(),
                scope: scope.clone(),
                is_array: false,
            };
            self.var_metadata.insert(name.clone(), var_meta);
            fields.push((type_node, name));
        }
        self.consume(
            TokenKind::RBrace,
            "Expected '}' after object destructure fields",
        )?;
        self.consume(
            TokenKind::Assign,
            "Expected '=' after object destructure pattern",
        )?;
        let rhs = self.parse_expression()?;
        if self.peek().kind == TokenKind::SemiColon {
            self.advance();
        }
        Ok(Stmt::Declaration(Decl::ObjectDestructureDecl {
            visibility: Visibility::Private,
            editability,
            type_name: None,
            fields,
            rhs,
        }))
    }

    pub(crate) fn parse_for_init_stmt(&mut self) -> Result<Stmt, String> {
        self.consume_optional_let();
        if self.is_var_decl_start() {
            self.parse_var_decl(ScopeType::Block).map(Stmt::Declaration)
        } else {
            self.parse_expression_stmt()
        }
    }

    pub(crate) fn parse_pattern_item(&mut self) -> Result<Pattern, String> {
        match &self.peek().kind.clone() {
            TokenKind::LParen => {
                self.advance();
                let mut items = Vec::new();
                if self.peek().kind != TokenKind::RParen {
                    items.push(self.parse_pattern_item()?);
                    while self.peek().kind == TokenKind::Comma {
                        self.advance();
                        items.push(self.parse_pattern_item()?);
                    }
                }
                self.consume(TokenKind::RParen, "Expected ')' after pattern tuple")?;
                if items.len() > 3 {
                    println!(
                        "Warning (Clean Code): Stack destructuring contains {} elements (recommended max: 3).",
                        items.len()
                    );
                }
                for sub in &items {
                    if let Pattern::Tuple(sub_items) = sub {
                        if sub_items.len() > 2 {
                            println!(
                                "Warning (Clean Code): Nested stack destructuring contains {} elements (recommended max: 2).",
                                sub_items.len()
                            );
                        }
                    }
                }
                Ok(Pattern::Tuple(items))
            }
            TokenKind::LBracket => {
                self.advance();
                let mut items = Vec::new();
                if self.peek().kind != TokenKind::RBracket {
                    items.push(self.parse_pattern_item()?);
                    while self.peek().kind == TokenKind::Comma {
                        self.advance();
                        items.push(self.parse_pattern_item()?);
                    }
                }
                self.consume(TokenKind::RBracket, "Expected ']' after pattern list")?;
                Ok(Pattern::Tuple(items))
            }
            TokenKind::LBrace => {
                self.advance();
                let mut fields = Vec::new();
                while self.peek().kind != TokenKind::RBrace && self.peek().kind != TokenKind::EOF {
                    if self.peek().kind == TokenKind::DotDot {
                        self.advance();
                        if self.peek().kind == TokenKind::Comma
                            || self.peek().kind == TokenKind::SemiColon
                        {
                            self.advance();
                        }
                        continue;
                    }
                    let fname = self
                        .get_identifier("Expected field identifier inside destructuring braces")?;
                    fields.push(fname);
                    if self.peek().kind == TokenKind::Comma
                        || self.peek().kind == TokenKind::SemiColon
                    {
                        self.advance();
                    }
                }
                self.consume(TokenKind::RBrace, "Expected '}' after destructuring fields")?;
                Ok(Pattern::Struct { name: None, fields })
            }
            _ => {
                let err = format!(
                    "Expected variable name or destructuring pattern, found {:?}",
                    self.peek().kind
                );
                let name = self.get_identifier(&err)?;
                Ok(Pattern::Identifier(name))
            }
        }
    }

    pub(crate) fn parse_destructuring_pattern(&mut self) -> Result<Pattern, String> {
        let first = self.parse_pattern_item()?;
        if self.peek().kind == TokenKind::Comma {
            let mut list = vec![first];
            while self.peek().kind == TokenKind::Comma {
                self.advance();
                list.push(self.parse_pattern_item()?);
            }
            if list.len() > 3 {
                println!(
                    "Warning (Clean Code): Declaring {} variables in a single statement is discouraged for code readability.",
                    list.len()
                );
            }
            Ok(Pattern::Tuple(list))
        } else {
            Ok(first)
        }
    }

    pub(crate) fn parse_rhs_item(&mut self) -> Result<RhsValue, String> {
        if self.peek().kind == TokenKind::LParen && !self.is_lambda_ahead() {
            self.advance();
            let mut items = Vec::new();
            let mut has_comma = false;
            if self.peek().kind != TokenKind::RParen {
                items.push(self.parse_rhs_item()?);
                while self.peek().kind == TokenKind::Comma {
                    has_comma = true;
                    self.advance();
                    items.push(self.parse_rhs_item()?);
                }
            }
            self.consume(TokenKind::RParen, "Expected ')' after group in RHS")?;
            if has_comma || items.len() > 1 {
                Ok(RhsValue::Group(items))
            } else if items.len() == 1 {
                let inner = items.into_iter().next().unwrap();
                if let RhsValue::Single(expr) = inner {
                    let full_expr = self.parse_expr_with_lhs(expr, 0)?;
                    Ok(RhsValue::Single(full_expr))
                } else {
                    Ok(inner)
                }
            } else {
                Ok(RhsValue::Group(Vec::new()))
            }
        } else {
            let expr = self.parse_expression()?;
            Ok(RhsValue::Single(expr))
        }
    }

    pub(crate) fn parse_rhs_values(&mut self) -> Result<RhsValue, String> {
        let first = self.parse_rhs_item()?;
        if self.peek().kind == TokenKind::Comma {
            let mut list = vec![first];
            while self.peek().kind == TokenKind::Comma {
                self.advance();
                list.push(self.parse_rhs_item()?);
            }
            Ok(RhsValue::Group(list))
        } else {
            Ok(first)
        }
    }

    pub(crate) fn parse_var_decl(&mut self, scope: ScopeType) -> Result<Decl, String> {
        if self.peek().kind == TokenKind::Const {
            if let (Some(t1), Some(t2)) = (
                self.tokens.get(self.current + 1),
                self.tokens.get(self.current + 2),
            ) {
                if matches!(&t1.kind, TokenKind::Identifier(_)) && t2.kind == TokenKind::Walrus {
                    self.advance(); // consume 'const'
                    let name = self.get_identifier("Expected variable name after 'const'")?;
                    self.consume(TokenKind::Walrus, "Expected ':=' after variable name")?;
                    let val = self.parse_expression()?;
                    if self.peek().kind == TokenKind::SemiColon {
                        self.advance();
                    }
                    let visibility = if scope == ScopeType::Global {
                        Visibility::Public
                    } else {
                        Visibility::Private
                    };
                    let var_meta = VarMetadata {
                        name: name.clone(),
                        type_node: BaseType::Unknown,
                        visibility: visibility.clone(),
                        editability: Editability::NotEditable,
                        scope,
                        is_array: false,
                    };
                    self.var_metadata.insert(name.clone(), var_meta);
                    return Ok(Decl::VarDecl {
                        visibility,
                        editability: Editability::NotEditable,
                        type_node: BaseType::Unknown,
                        place: Place::Local,
                        name,
                        assign_op: ":=".to_string(),
                        value: val,
                    });
                }
            }
        }

        if let TokenKind::Identifier(ref _ident) = self.peek().kind {
            if let Some(t1) = self.tokens.get(self.current + 1) {
                if t1.kind == TokenKind::Walrus {
                    let name = self.get_identifier("Expected variable name")?;
                    self.consume(TokenKind::Walrus, "Expected ':=' after variable name")?;
                    let val = self.parse_expression()?;
                    if self.peek().kind == TokenKind::SemiColon {
                        self.advance();
                    }
                    let visibility = if scope == ScopeType::Global {
                        Visibility::Public
                    } else {
                        Visibility::Private
                    };
                    let var_meta = VarMetadata {
                        name: name.clone(),
                        type_node: BaseType::Unknown,
                        visibility: visibility.clone(),
                        editability: Editability::Editable,
                        scope,
                        is_array: false,
                    };
                    self.var_metadata.insert(name.clone(), var_meta);
                    return Ok(Decl::VarDecl {
                        visibility,
                        editability: Editability::Editable,
                        type_node: BaseType::Unknown,
                        place: Place::Local,
                        name,
                        assign_op: ":=".to_string(),
                        value: val,
                    });
                }
            }
        }

        let type_name = self.parse_type()?;
        let pattern = self.parse_destructuring_pattern()?;

        let mut is_arr = false;
        let mut array_sizes = Vec::new();
        if let Pattern::Identifier(_) = &pattern {
            while self.peek().kind == TokenKind::LBracket {
                is_arr = true;
                self.advance();
                let size = if self.peek().kind != TokenKind::RBracket {
                    Some(self.parse_expression()?)
                } else {
                    None
                };
                self.consume(TokenKind::RBracket, "Expected ']' after array size")?;
                array_sizes.push(size);
            }
        }

        let mut assign_op = String::new();
        let mut rhs_opt = None;
        if self.peek().kind == TokenKind::Assign || self.peek().kind == TokenKind::Arrow {
            assign_op = self.peek().kind.clone().as_str().to_string();
            self.advance();
            rhs_opt = Some(self.parse_rhs_values()?);
        }

        if self.peek().kind == TokenKind::SemiColon {
            self.advance();
        }

        let visibility = if scope == ScopeType::Global {
            Visibility::Public
        } else {
            Visibility::Private
        };

        match pattern {
            Pattern::Identifier(ref name) if !is_arr => {
                let val = match rhs_opt {
                    Some(RhsValue::Single(expr)) => expr,
                    Some(RhsValue::Group(items)) => {
                        // Case 7: int(32) x = (10, 20); -> ArrayLiteral
                        let mut elems = Vec::new();
                        for item in items {
                            match item {
                                RhsValue::Single(e) => elems.push(e),
                                RhsValue::Group(_) => {
                                    let mut sub = Vec::new();
                                    flatten_rhs(&item, &mut sub);
                                    elems.push(Expr::ArrayLiteral(sub));
                                }
                            }
                        }
                        Expr::ArrayLiteral(elems)
                    }
                    None => Expr::Default(None),
                };

                let var_meta = VarMetadata {
                    name: name.clone(),
                    type_node: type_name.clone(),
                    visibility: visibility.clone(),
                    editability: Editability::Editable,
                    scope,
                    is_array: false,
                };
                self.var_metadata.insert(name.clone(), var_meta);

                let place = if matches!(val, Expr::New { .. }) {
                    Place::Heap
                } else {
                    Place::Local
                };

                Ok(Decl::VarDecl {
                    visibility,
                    editability: Editability::Editable,
                    type_node: type_name,
                    place,
                    assign_op,
                    name: name.clone(),
                    value: val,
                })
            }
            Pattern::Identifier(ref name) if is_arr => {
                let val = match rhs_opt {
                    Some(RhsValue::Single(expr)) => expr,
                    Some(RhsValue::Group(items)) => {
                        let mut elems = Vec::new();
                        for item in items {
                            match item {
                                RhsValue::Single(e) => elems.push(e),
                                RhsValue::Group(_) => {
                                    let mut sub = Vec::new();
                                    flatten_rhs(&item, &mut sub);
                                    elems.push(Expr::ArrayLiteral(sub));
                                }
                            }
                        }
                        Expr::ArrayLiteral(elems)
                    }
                    None => Expr::Default(None),
                };

                let elem_type = if array_sizes.len() > 1 {
                    let mut t = type_name.clone();
                    for size in array_sizes.iter().skip(1).rev() {
                        t = BaseType::Array {
                            base_type: Box::new(t),
                            size: Box::new(size.clone()),
                        };
                    }
                    t
                } else {
                    type_name.clone()
                };

                let var_meta = VarMetadata {
                    name: name.clone(),
                    type_node: elem_type.clone(),
                    visibility: visibility.clone(),
                    editability: Editability::Editable,
                    scope,
                    is_array: true,
                };
                self.var_metadata.insert(name.clone(), var_meta);

                let len_expr = array_sizes
                    .first()
                    .cloned()
                    .flatten()
                    .unwrap_or(Expr::LiteralInt(0));

                Ok(Decl::ArrayDecl {
                    visibility,
                    editability: Editability::Editable,
                    type_node: elem_type,
                    name: name.clone(),
                    assign_op,
                    length: len_expr,
                    value: val,
                })
            }
            _ => {
                // Destructuring (multiple vars, tuple, struct)
                let mut assignments = Vec::new();
                if let Some(rhs) = rhs_opt {
                    match_destructure(&pattern, &rhs, &mut assignments)?;
                } else {
                    let mut names = Vec::new();
                    collect_pattern_identifiers(&pattern, &mut names);
                    for n in names {
                        assignments.push((n, Expr::Default(None)));
                    }
                }

                for (name, _) in &assignments {
                    let var_meta = VarMetadata {
                        name: name.clone(),
                        type_node: type_name.clone(),
                        visibility: visibility.clone(),
                        editability: Editability::Editable,
                        scope: scope.clone(),
                        is_array: false,
                    };
                    self.var_metadata.insert(name.clone(), var_meta);
                }

                Ok(Decl::DestructureDecl {
                    visibility,
                    editability: Editability::Editable,
                    type_node: type_name,
                    pattern,
                    assignments,
                    assign_op,
                })
            }
        }
    }

    // ====================================================
    // Control Flow Parsers
    // ====================================================

    // --- set <target> -> <value>; OR set name<T>(params) -> return_type; ---
    pub(crate) fn parse_reassign_stmt(&mut self) -> Result<Stmt, String> {
        self.advance(); // 'set'

        // Check if this is an intrinsic signature declaration: `set name<T>(params) -> return_type;` or `set name() -> return_type;`
        if let TokenKind::Identifier(name) = &self.peek().kind.clone() {
            if let Some(next) = self.tokens.get(self.current + 1) {
                if next.kind == TokenKind::Less || next.kind == TokenKind::LParen {
                    let fn_name = name.clone();
                    self.advance(); // consume name
                    let mut generics = Vec::new();
                    if self.peek().kind == TokenKind::Less {
                        self.parse_generics(&mut generics)?;
                        for g in &generics {
                            if let BaseType::GenericParam(gen_name) = g {
                                self.current_generics.insert(gen_name.clone());
                            }
                        }
                    }

                    self.consume(TokenKind::LParen, "Expected '(' in signature declaration")?;
                    let mut params = Vec::new();
                    while !self.is_at_end() && self.peek().kind != TokenKind::RParen {
                        let p = if self.peek().kind == TokenKind::DotDotDot
                            || self
                                .peek_ahead(1)
                                .map(|t| t.kind == TokenKind::Colon)
                                .unwrap_or(false)
                        {
                            self.parse_single_param()?
                        } else {
                            let ty = self.parse_type()?;
                            Param {
                                name: format!("arg_{}", params.len()),
                                type_node: ty,
                                default_value: None,
                                is_variadic: false,
                            }
                        };
                        params.push(p);
                        if self.peek().kind == TokenKind::Comma {
                            self.advance();
                        } else if self.peek().kind != TokenKind::RParen {
                            return Err("Expected ',' or ')' in parameter list".to_string());
                        }
                    }
                    self.consume(TokenKind::RParen, "Expected ')' after parameter list")?;

                    let return_type = if self.peek().kind == TokenKind::Arrow {
                        self.advance();
                        self.parse_type()?
                    } else {
                        BaseType::Void
                    };

                    self.consume(
                        TokenKind::SemiColon,
                        "Expected ';' after signature declaration",
                    )?;

                    for g in &generics {
                        if let BaseType::GenericParam(gen_name) = g {
                            self.current_generics.remove(gen_name);
                        }
                    }

                    return Ok(Stmt::Declaration(Decl::FnDecl {
                        visibility: Visibility::Public,
                        name: fn_name,
                        generics,
                        params,
                        return_type,
                        body: vec![],
                        is_virtual: false,
                        is_abstract: false,
                    }));
                }
            }
        }

        let target = self.parse_expression()?;

        let op = self.peek().kind.as_str().to_string();
        if op != TokenKind::Arrow.as_str() && op != TokenKind::Assign.as_str() {
            return Err(
                format!(
                    "Syntax Error: Expected '->' or '=' after target in set statement at line {}, column {}",
                    self.peek().line,
                    self.peek().column
                )
            );
        }

        self.advance();

        let value = self.parse_expression()?;
        self.consume(TokenKind::SemiColon, "Expected ';' after set statement")?;

        Ok(Stmt::ReassignStmt { target, value, op })
    }

    // --- if (cond) { ... } else { ... } -------------------
    pub(crate) fn parse_if_stmt(&mut self) -> Result<Stmt, String> {
        self.advance(); // 'if'

        self.consume(TokenKind::LParen, "Expected '(' after 'if'")?;
        let condition = self.parse_expression()?;
        self.consume(TokenKind::RParen, "Expected ')' after if condition")?;

        if self.peek().kind == TokenKind::Arrow {
            self.advance();
        }

        // then block
        let then_block = if self.peek().kind == TokenKind::LBrace {
            self.advance(); // consume '{'
            let blk = self.parse_block("if".to_string())?;
            self.consume(TokenKind::RBrace, "Expected '}' to close if body")?;
            blk
        } else {
            match self.parse_statement(ScopeType::Block)? {
                Some(stmt) => vec![stmt],
                None => vec![],
            }
        };

        // else block
        let else_block = if self.peek().kind == TokenKind::Else {
            self.advance(); // 'else'

            if self.peek().kind == TokenKind::If {
                let nested = self.parse_if_stmt()?;
                Some(vec![nested])
            } else {
                if self.peek().kind == TokenKind::Arrow {
                    self.advance();
                }
                if self.peek().kind == TokenKind::LBrace {
                    self.advance(); // consume '{'
                    let blk = self.parse_block("if".to_string())?;
                    self.consume(TokenKind::RBrace, "Expected '}' to close else block")?;
                    Some(blk)
                } else {
                    match self.parse_statement(ScopeType::Block)? {
                        Some(stmt) => Some(vec![stmt]),
                        None => None,
                    }
                }
            }
        } else {
            None
        };

        Ok(Stmt::IfStmt {
            condition,
            then_block,
            else_block,
        })
    }

    //todo : make parse_scope_decl deal with switch
    pub(crate) fn parse_switch_stmt(&mut self) -> Result<Stmt, String> {
        self.advance(); // consume 'switch' or 'match'
        let condition = if self.peek().kind == TokenKind::LParen {
            self.advance();
            let c = self.parse_expression()?;
            self.consume(
                TokenKind::RParen,
                "Expected ')' after switch/match condition",
            )?;
            c
        } else {
            self.parse_expression()?
        };

        if self.peek().kind == TokenKind::Arrow {
            self.advance(); // consume optional '->'
        }

        self.consume(TokenKind::LBrace, "Expected '{' to open switch/match block")?;
        let mut cases = Vec::new();
        while !self.is_at_end() && self.peek().kind != TokenKind::RBrace {
            if self.peek().kind == TokenKind::Underscore {
                self.advance(); // consume '_'
                self.consume(
                    TokenKind::FatArrow,
                    "Expected '=>' after '_' in default match branch",
                )?;

                let def_body = if self.peek().kind == TokenKind::LBrace {
                    self.advance();
                    let b = self.parse_block("switch".to_string())?;
                    self.consume(TokenKind::RBrace, "Expected '}' to close default block")?;
                    if self.peek().kind == TokenKind::Comma
                        || self.peek().kind == TokenKind::SemiColon
                    {
                        self.advance();
                    }
                    b
                } else {
                    let s = self.parse_statement(ScopeType::Block)?;
                    if self.peek().kind == TokenKind::Comma {
                        self.advance();
                    }
                    match s {
                        Some(st) => vec![st],
                        None => vec![],
                    }
                };

                cases.push(Stmt::CaseStmt {
                    option: Expr::Identifier("void".to_string()),
                    set: Expr::Identifier("void".to_string()),
                    body: def_body,
                });
            } else {
                self.in_match_pattern = true;
                let val_res = self.parse_expression();
                self.in_match_pattern = false;
                let val = val_res?;
                self.consume(TokenKind::FatArrow, "Expected '=>' after match pattern")?;

                let case_body = if self.peek().kind == TokenKind::LBrace {
                    self.advance();
                    let b = self.parse_block("case".to_string())?;
                    self.consume(
                        TokenKind::RBrace,
                        "Expected '}' to close match branch block",
                    )?;
                    if self.peek().kind == TokenKind::Comma
                        || self.peek().kind == TokenKind::SemiColon
                    {
                        self.advance();
                    }
                    b
                } else {
                    let s = self.parse_statement(ScopeType::Block)?;
                    if self.peek().kind == TokenKind::Comma {
                        self.advance();
                    }
                    match s {
                        Some(st) => vec![st],
                        None => vec![],
                    }
                };

                cases.push(Stmt::CaseStmt {
                    option: val,
                    set: Expr::Identifier("void".to_string()),
                    body: case_body,
                });
            }
        }
        self.consume(
            TokenKind::RBrace,
            "Expected '}' to close switch/match block",
        )?;

        Ok(Stmt::SwitchStmt {
            name: String::new(),
            condition,
            cases,
        })
    }

    pub(crate) fn parse_del_stmt(&mut self) -> Result<Stmt, String> {
        self.advance(); // consume 'del'
        let expr = self.parse_expression()?;
        let mut is_array = false;
        if let Expr::Identifier(name) = &expr {
            let var = self.var_metadata.get(name);
            is_array = var.is_some() && var.unwrap().is_array;
        }
        self.consume(TokenKind::SemiColon, "Expected ';' after del statement")?;
        Ok(Stmt::DelStmt {
            target: expr,
            is_array,
        })
    }

    pub(crate) fn parse_throw_stmt(&mut self) -> Result<Stmt, String> {
        self.advance(); // 'throw'
        let expr = self.parse_expression()?;
        self.consume(TokenKind::SemiColon, "Expected ';' after throw statement")?;
        Ok(Stmt::ThrowStmt(expr))
    }

    pub(crate) fn parse_goto_stmt(&mut self) -> Result<Stmt, String> {
        self.advance(); // consume 'goto'
        if self.peek().kind == TokenKind::Arrow {
            self.advance();
        }
        let target = if let TokenKind::LabelName(name) = self.peek().kind.clone() {
            self.advance();
            name
        } else {
            return Err("Expected label name ex @label after 'goto'".to_string());
        };

        self.consume(TokenKind::SemiColon, "Expected ';' after goto statement")?;

        Ok(Stmt::GotoStmt(Expr::Identifier(target)))
    }
    //todo : remove this

    pub(crate) fn parse_yield_stmt(&mut self) -> Result<Stmt, String> {
        self.advance(); // consume 'yield'
        let mut expr = None;
        if self.peek().kind != TokenKind::SemiColon {
            expr = Some(self.parse_expression()?);
        }
        self.consume(TokenKind::SemiColon, "Expected ';' after yield")?;
        Ok(Stmt::YieldStmt(expr))
    }

    //TODO: fix this or remove it and build a better system for it
    pub(crate) fn parse_expression_or_reassignment(&mut self) -> Result<Stmt, String> {
        if let TokenKind::Identifier(name) = self.peek().kind.clone() {
            if name == "let" {
                self.advance();
                return self.parse_var_decl(ScopeType::Block).map(Stmt::Declaration);
            }

            let next1 = self.tokens.get(self.current + 1).map(|t| &t.kind);
            let next2 = self.tokens.get(self.current + 2).map(|t| &t.kind);

            // `TypeName varName =` or `TypeName<T> varName =` or `varName :=` pattern
            let is_var_decl = if self.macro_metadata.contains(&name) {
                false
            } else if next1 == Some(&TokenKind::Walrus) {
                true
            } else {
                match (next1, next2) {
                    (
                        Some(TokenKind::Identifier(_)),
                        Some(
                            TokenKind::Assign
                            | TokenKind::Arrow
                            | TokenKind::SemiColon
                            | TokenKind::LBracket,
                        ),
                    ) => true,
                    (Some(TokenKind::LBrace), _) => true,
                    (Some(TokenKind::Less), _) => {
                        let mut i = self.current + 2;
                        let mut depth = 1;
                        while i < self.tokens.len() && depth > 0 {
                            match &self.tokens[i].kind {
                                TokenKind::Less => {
                                    depth += 1;
                                }
                                TokenKind::Greater => {
                                    depth -= 1;
                                }
                                _ => {}
                            }
                            i += 1;
                        }
                        matches!(
                            (
                                self.tokens.get(i).map(|t| &t.kind),
                                self.tokens.get(i + 1).map(|t| &t.kind),
                            ),
                            (
                                Some(TokenKind::Identifier(_)),
                                Some(
                                    TokenKind::Arrow
                                        | TokenKind::Assign
                                        | TokenKind::SemiColon
                                        | TokenKind::LBracket
                                ),
                            ) | (Some(TokenKind::LBrace), _)
                        )
                    }
                    _ => false,
                }
            };

            if is_var_decl {
                return self.parse_var_decl(ScopeType::Block).map(Stmt::Declaration);
            }
        }

        let expr = self.parse_expression()?;

        // `target -> value;` or `target = value;`
        let op = self.peek().kind.clone();
        if op == TokenKind::Arrow {
            self.advance(); // consume '->'
            let value = self.parse_expression()?;
            self.consume(TokenKind::SemiColon, "Expected ';' after arrow expression")?;

            return Ok(Stmt::ExpressionStmt(Expr::BinaryOp {
                left: Box::new(expr),
                operator: "->".to_string(),
                right: Box::new(value),
            }));
        }

        if op == TokenKind::Assign
            || op == TokenKind::PlusAssign
            || op == TokenKind::MinusAssign
            || op == TokenKind::MulAssign
            || op == TokenKind::DivAssign
            || op == TokenKind::Walrus
            || op == TokenKind::Colon
        {
            self.advance(); // consume '=' or ':' or '+=' etc.
            let value = self.parse_expression()?;
            let op_str = if op == TokenKind::Colon {
                "=".to_string()
            } else {
                op.as_str().to_string()
            };
            let reassign_stmt = Stmt::ReassignStmt {
                target: expr,
                value,
                op: op_str,
            };
            if self.peek().kind == TokenKind::If {
                self.advance();
                let cond = self.parse_postfix_condition()?;
                if self.peek().kind == TokenKind::SemiColon || self.peek().kind == TokenKind::Comma {
                    self.advance();
                }
                return Ok(Stmt::IfStmt {
                    condition: cond,
                    then_block: vec![reassign_stmt],
                    else_block: None,
                });
            }
            if self.peek().kind == TokenKind::Comma {
                self.advance();
            } else if self.peek().kind == TokenKind::SemiColon {
                self.advance();
            } else if self.peek().kind != TokenKind::RBrace {
                self.consume(
                    TokenKind::SemiColon,
                    "Expected ';' or ',' after reassignment",
                )?;
            }
            return Ok(reassign_stmt);
        }

        // Command-style macro / function invocation: `callee arg1, arg2;`
        if matches!(&expr, Expr::Identifier(_) | Expr::NamespaceAccess { .. }) {
            if !self.is_at_end()
                && self.peek().kind != TokenKind::SemiColon
                && self.peek().kind != TokenKind::RBrace
                && self.peek().kind != TokenKind::RParen
                && self.peek().kind != TokenKind::RBracket
                && self.peek().kind != TokenKind::Comma
            {
                let mut args = Vec::new();
                args.push(self.parse_expression()?);
                while self.peek().kind == TokenKind::Comma {
                    self.advance();
                    args.push(self.parse_expression()?);
                }
                if self.peek().kind == TokenKind::SemiColon {
                    self.advance();
                }
                return Ok(Stmt::CallStmt(Expr::Call {
                    callee: Box::new(expr),
                    generics: Vec::new(),
                    args,
                }));
            }
        }

        let base_stmt = Stmt::ExpressionStmt(expr);
        if self.peek().kind == TokenKind::If {
            self.advance();
            let cond = self.parse_postfix_condition()?;
            if self.peek().kind == TokenKind::SemiColon || self.peek().kind == TokenKind::Comma {
                self.advance();
            }
            return Ok(Stmt::IfStmt {
                condition: cond,
                then_block: vec![base_stmt],
                else_block: None,
            });
        }
        if self.peek().kind == TokenKind::SemiColon || self.peek().kind == TokenKind::Comma {
            self.advance();
        }
        Ok(base_stmt)
    }

    pub(crate) fn parse_postfix_condition(&mut self) -> Result<Expr, String> {
        let cond = if self.peek().kind == TokenKind::LParen {
            self.advance();
            let c = self.parse_expression()?;
            self.consume(TokenKind::RParen, "Expected ')' after postfix if condition")?;
            c
        } else {
            self.parse_expression()?
        };
        if self.peek().kind == TokenKind::Else {
            return Err("Syntax Error: 'else' is not allowed in postfix if statements".to_string());
        }
        Ok(cond)
    }

    pub(crate) fn parse_expression_stmt(&mut self) -> Result<Stmt, String> {
        let expr = self.parse_expression()?;
        // --- Bare reassignment: x = 10; or this.x = 20; ---
        let op = self.peek().kind.clone();
        if op == TokenKind::Assign
            || op == TokenKind::Arrow
            || op == TokenKind::PlusAssign
            || op == TokenKind::MinusAssign
            || op == TokenKind::MulAssign
            || op == TokenKind::DivAssign
        {
            self.advance(); // consume '=' or '->' or '+=' etc
            let value = self.parse_expression()?;
            let op_str = op.as_str().to_string();
            let base_stmt = Stmt::ReassignStmt {
                target: expr,
                value,
                op: op_str,
            };
            if self.peek().kind == TokenKind::If {
                self.advance();
                let cond = self.parse_postfix_condition()?;
                self.consume(
                    TokenKind::SemiColon,
                    "Expected ';' after postfix if",
                )?;
                return Ok(Stmt::IfStmt {
                    condition: cond,
                    then_block: vec![base_stmt],
                    else_block: None,
                });
            }
            self.consume(
                TokenKind::SemiColon,
                "Expected ';' after assignment statement",
            )?;
            return Ok(base_stmt);
        }
        let base_stmt = Stmt::ExpressionStmt(expr);
        if self.peek().kind == TokenKind::If {
            self.advance();
            let cond = self.parse_postfix_condition()?;
            self.consume(
                TokenKind::SemiColon,
                "Expected ';' after postfix if",
            )?;
            return Ok(Stmt::IfStmt {
                condition: cond,
                then_block: vec![base_stmt],
                else_block: None,
            });
        }
        if self.peek().kind == TokenKind::SemiColon {
            self.consume(
                TokenKind::SemiColon,
                "Expected ';' after expression statement",
            )?;
        }
        Ok(base_stmt)
    }

    pub(crate) fn parse_try_catch_stmt(&mut self) -> Result<Stmt, String> {
        self.advance(); // 'try'
        if self.peek().kind == TokenKind::Arrow {
            self.advance(); // consume '->'
        }
        self.consume(TokenKind::LBrace, "Expected '{' to open try block")?;
        let try_block = self.parse_block("try".to_string())?;
        self.consume(TokenKind::RBrace, "Expected '}' to close try block")?;

        self.consume(TokenKind::Catch, "Expected 'catch' after try block")?;
        self.consume(TokenKind::LParen, "Expected '(' after 'catch'")?;

        let catch_param: String = if let TokenKind::Identifier(n) = &self.peek().kind.clone() {
            let n = n.to_string();
            self.advance();
            n
        } else {
            return Err(format!(
                "Expected parameter name in catch block at line {}",
                self.peek().line
            ));
        };

        self.consume(TokenKind::RParen, "Expected ')' after catch parameter")?;
        if self.peek().kind == TokenKind::Arrow {
            self.advance(); // consume '->'
        }
        self.consume(TokenKind::LBrace, "Expected '{' to open catch block")?;
        let catch_block = self.parse_block("catch".to_string())?;
        self.consume(TokenKind::RBrace, "Expected '}' to close catch block")?;

        Ok(Stmt::TryCatchStmt {
            try_block,
            catch_param: catch_param.to_string(),
            catch_block,
        })
    }

    // ====================================================
    // OOP & Enum Parsers
    // ====================================================

    pub(crate) fn get_identifier(&mut self, err_msg: &str) -> Result<String, String> {
        match &self.peek().kind {
            TokenKind::Identifier(n) => {
                let s = n.clone();
                self.advance();
                Ok(s)
            }
            _ => Err(err_msg.to_string()),
        }
    }

    pub(crate) fn get_handle_identifier(&mut self, err_msg: &str) -> Result<String, String> {
        match &self.peek().kind {
            TokenKind::Identifier(n) => {
                let s = n.clone();
                self.advance();
                Ok(s)
            }
            TokenKind::Call => {
                self.advance();
                Ok("call".to_string())
            }
            TokenKind::Return => {
                self.advance();
                Ok("return".to_string())
            }
            TokenKind::Break => {
                self.advance();
                Ok("break".to_string())
            }
            TokenKind::Continue => {
                self.advance();
                Ok("continue".to_string())
            }
            TokenKind::Leave => {
                self.advance();
                Ok("leave".to_string())
            }
            TokenKind::Yield => {
                self.advance();
                Ok("yield".to_string())
            }
            TokenKind::Throw => {
                self.advance();
                Ok("throw".to_string())
            }
            TokenKind::Share => {
                self.advance();
                Ok("share".to_string())
            }
            _ => Err(err_msg.to_string()),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum RhsValue {
    Single(Expr),
    Group(Vec<RhsValue>),
}

fn match_destructure(
    pattern: &Pattern,
    value: &RhsValue,
    results: &mut Vec<(String, Expr)>,
) -> Result<(), String> {
    match pattern {
        Pattern::Identifier(name) => match value {
            RhsValue::Single(expr) => {
                results.push((name.clone(), expr.clone()));
                Ok(())
            }
            RhsValue::Group(items) => {
                let mut expr_elements = Vec::new();
                for item in items {
                    match item {
                        RhsValue::Single(e) => expr_elements.push(e.clone()),
                        RhsValue::Group(_) => {
                            let mut sub = Vec::new();
                            flatten_rhs(item, &mut sub);
                            expr_elements.push(Expr::ArrayLiteral(sub));
                        }
                    }
                }
                results.push((name.clone(), Expr::ArrayLiteral(expr_elements)));
                Ok(())
            }
        },
        Pattern::Tuple(patterns) => match value {
            RhsValue::Single(Expr::ArrayLiteral(elems)) if elems.len() == patterns.len() => {
                for (p, elem) in patterns.iter().zip(elems.iter()) {
                    match_destructure(p, &RhsValue::Single(elem.clone()), results)?;
                }
                Ok(())
            }
            RhsValue::Single(expr) => {
                for p in patterns {
                    match_destructure(p, &RhsValue::Single(expr.clone()), results)?;
                }
                Ok(())
            }
            RhsValue::Group(items) => {
                if items.len() == 1 {
                    let single_item = &items[0];
                    for p in patterns {
                        match_destructure(p, single_item, results)?;
                    }
                    Ok(())
                } else if patterns.len() == items.len() {
                    for (p, item) in patterns.iter().zip(items.iter()) {
                        match_destructure(p, item, results)?;
                    }
                    Ok(())
                } else {
                    Err(
                            format!(
                                "Semantic Error: Destructuring mismatch: {} variables on LHS vs {} values on RHS.",
                                patterns.len(),
                                items.len()
                            )
                        )
                }
            }
        },
        Pattern::Struct { name: _, fields } => match value {
            RhsValue::Single(expr) => {
                for f in fields {
                    let prop_access = Expr::PropertyAccess {
                        object: Box::new(expr.clone()),
                        property: f.clone(),
                    };
                    results.push((f.clone(), prop_access));
                }
                Ok(())
            }
            RhsValue::Group(_) => Err(
                "Semantic Error: Cannot unpack struct pattern from tuple/group RHS.".to_string(),
            ),
        },
    }
}

fn flatten_rhs(val: &RhsValue, out: &mut Vec<Expr>) {
    match val {
        RhsValue::Single(e) => out.push(e.clone()),
        RhsValue::Group(items) => {
            for it in items {
                flatten_rhs(it, out);
            }
        }
    }
}

fn collect_pattern_identifiers(pattern: &Pattern, out: &mut Vec<String>) {
    match pattern {
        Pattern::Identifier(n) => out.push(n.clone()),
        Pattern::Tuple(pats) => {
            for p in pats {
                collect_pattern_identifiers(p, out);
            }
        }
        Pattern::Struct { fields, .. } => {
            for f in fields {
                out.push(f.clone());
            }
        }
    }
}
