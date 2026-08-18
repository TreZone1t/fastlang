use crate::backend::codegen::stmt;
use crate::frontend::lexer::token::TokenKind;
use crate::frontend::parser::ast::*;
use crate::frontend::parser::parser::Parser;

impl Parser {
    pub(crate) fn parse_statement(&mut self, scope: String) -> Result<Option<Stmt>, String> {
        eprintln!(
            "DISPATCH: {:?} at line {}",
            self.peek().kind,
            self.peek().line
        );
        let result: Result<crate::frontend::parser::ast::Stmt, String> = match &self.peek().kind {
            TokenKind::SemiColon => {
                self.advance();
                return Ok(None);
            }
            TokenKind::Import => self.parse_import_stmt().map(Stmt::Declaration),
            TokenKind::Export => self.parse_exported_stmt().map(Stmt::Declaration),
            TokenKind::Const => self.parse_const().map(Stmt::Declaration),
            | TokenKind::TypeInt
            | TokenKind::TypeFloat
            | TokenKind::TypeChar
            | TokenKind::TypeBool
            | TokenKind::TypeType
          /*//todo | TokenKind::TypeObject */=> self.parse_var_decl(true, false).map(Stmt::Declaration),
            TokenKind::TypeName => self.parse_name().map(Stmt::Declaration),
            TokenKind::TypeBluePrint => self.parse_blueprint_decl().map(Stmt::Declaration),
            TokenKind::Impl => self.parse_impl_decl().map(Stmt::Declaration),
            TokenKind::Set => self.parse_reassign_stmt(),
            TokenKind::If => self.parse_if_stmt(),
            TokenKind::For => self.parse_for_stmt(),
            TokenKind::Loop => self.parse_loop_stmt(),
            TokenKind::While => self.parse_while_stmt(),
            TokenKind::Switch => self.parse_switch_stmt(),
            TokenKind::Fn => self.parse_fn_decl().map(Stmt::Declaration),
            TokenKind::TypeClass => self.parse_class_decl().map(Stmt::Declaration),
                        TokenKind::TypeCustom => {
                self.advance();
                let name = self.get_identifier("Expected custom scope name")?;
                self.parse_custom_decl(name)
            },
            TokenKind::TypeStruct => self.parse_struct_decl().map(Stmt::Declaration),
            TokenKind::TypeEnum => self.parse_enum_decl().map(Stmt::Declaration),
            TokenKind::Del => self.parse_del_stmt(),
            TokenKind::Leave => {
                self.advance();
                self.consume(TokenKind::SemiColon, "Expected ';' after leave")?;
                Ok(Stmt::LeaveStmt)
            }
            TokenKind::Yield => self.parse_yield_stmt(),
            TokenKind::Call => self.parse_call_stmt(),
            TokenKind::Break => {
                self.advance();
                self.consume(TokenKind::SemiColon, "Expected ';' after break")?;
                Ok(Stmt::BreakStmt)
            }
            TokenKind::Continue => {
                self.advance();
                self.consume(TokenKind::SemiColon, "Expected ';' after continue")?;
                Ok(Stmt::ContinueStmt)
            }
            TokenKind::Return => {
                self.advance();

                if self.peek().kind == TokenKind::SemiColon {
                    self.advance();
                    Ok(Stmt::ReturnStmt(Expr::Identifier("null".to_string())))
                } else {
                    match self.parse_expression() {
                        Ok(val) => {
                            if self.peek().kind == TokenKind::SemiColon {
                                self.advance();
                            }
                            Ok(Stmt::ReturnStmt(val))
                        }
                        Err(e) => Err(e),
                    }
                }
            }
            TokenKind::Throw => self.parse_throw_stmt(),
            TokenKind::Try => self.parse_try_catch_stmt(),
            // Custom keyword: `my_list<int(32)> items -> [1, 2];`
            TokenKind::Identifier(_) => self.parse_expression_or_reassignment(),
            TokenKind::This => self.parse_expression_or_reassignment(),
            TokenKind::Super => self.parse_expression_or_reassignment(),
            TokenKind::Goto => self.parse_goto_stmt( scope.clone()),
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

        self.consume(TokenKind::SemiColon, "Expected ';' after import statement")?;

        Ok(Decl::Import {
            module_path,
            imports,
        })
    }
    pub(crate) fn parse_const(&mut self) -> Result<Decl, String> {
        self.advance(); // consume 'const'

        // هنا بنشيل الـ .map(Stmt::Declaration) ونستخدم الـ ? عشان يفضل نوع المتغير Decl
        let mut decl = match &self.peek().kind {
            TokenKind::TypeInt
            | TokenKind::TypeFloat
            | TokenKind::TypeChar
            | TokenKind::TypeBool
            | TokenKind::TypeType => self.parse_var_decl(true, false)?,
            TokenKind::TypeName => self.parse_name()?,
            _ => return Err("Syntax Error: Expected variable declaration".to_string()),
        };

        // تصليح كلمة match وتصليح الحقول لتطابق الـ Decl والـ editability بحرف سمول
        match &mut decl {
            Decl::VarDecl {
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
        self.advance(); // consume 'export'

        // After export, we expect a valid exportable statement (fn, scope, let, class, struct, enum)
        let mut stmt = match &self.peek().kind {
            TokenKind::Fn => self.parse_fn_decl()?,
            TokenKind::TypeClass => self.parse_class_decl()?,
            TokenKind::TypeStruct => self.parse_struct_decl()?,
            TokenKind::TypeEnum => self.parse_enum_decl()?,
            kind => return Err(format!("Syntax Error: Cannot export '{:?}', only let, fn, scope, class, struct, and enum can be exported", kind)),
        };

        // Set is_exported flag to true
        match &mut stmt {
            Decl::FnDecl {
                ref mut is_exported,
                ..
            } => *is_exported = true,
            Decl::BlockDecl {
                ref mut is_exported,
                ..
            } => *is_exported = true,
            Decl::CustomDecl {
                ref mut is_exported,
                ..
            } => *is_exported = true,
            Decl::ClassDecl {
                ref mut is_exported,
                ..
            } => *is_exported = true,
            Decl::StructDecl {
                ref mut is_exported,
                ..
            } => *is_exported = true,
            Decl::EnumDecl {
                ref mut is_exported,
                ..
            } => *is_exported = true,
            Decl::VarDecl {
                ref mut visibility, ..
            } => *visibility = Visibility::Public,
            Decl::ArrayDecl {
                ref mut visibility, ..
            } => *visibility = Visibility::Public,
            _ => {}
        }

        Ok(stmt)
    }

    pub(crate) fn parse_var_decl(
        &mut self,
        is_global: bool,
        no_semi: bool,
    ) -> Result<Decl, String> {
        let var_meta: VarMetadata;
        let type_name = self.parse_type()?;
        let name = self.get_identifier("Expected variable name after type")?;
        let mut size = None;
        if self.peek().kind == TokenKind::LBracket {
            //[size]
            self.advance();
            if matches!(self.peek().kind, TokenKind::Int(_)) {
                size = Some(self.parse_expression()?);
            } else {
                return Err("Syntax Error: Expected size for array".to_string());
            }
            self.consume(TokenKind::RBracket, "Expected ']' after array size")?;
        }
        let mut value = Expr::Identifier("__default__".to_string());

        if self.peek().kind == TokenKind::Arrow || self.peek().kind == TokenKind::Assign {
            self.advance();
            value = self.parse_expression()?;
        }

        if !no_semi {
            // ! fix it
            if self.peek().kind == TokenKind::SemiColon {
                println!("DEBUG: parse_var_decl: {:?}", self.peek().kind);
                self.advance();
            }
        }
        let is_heaped = if let BaseType::Pointer(_) = type_name {
            true
        } else {
            false
        };
        if size.is_none() {
            var_meta = VarMetadata {
                name: name.clone(),
                type_node: type_name.clone(),
                visibility: if is_global {
                    Visibility::Public
                } else {
                    Visibility::Private
                },
                editability: Editability::Editable,
                scope: if is_global {
                    ScopeType::Global
                } else {
                    ScopeType::Local
                },
                is_heaped: is_heaped,
                is_array: false,
            };
            self.var_metadata.insert(name.clone(), var_meta);
            if is_heaped {
                Ok(Decl::PointerDecl {
                    name,
                    inner_type: type_name,
                    length: None,
                    value,
                })
            } else {
                Ok(Decl::VarDecl {
                    visibility: Visibility::Private,
                    editability: Editability::Editable,
                    type_node: type_name,
                    name,
                    value,
                })
            }
        } else {
            var_meta = VarMetadata {
                name: name.clone(),
                type_node: type_name.clone(),
                visibility: if is_global {
                    Visibility::Public
                } else {
                    Visibility::Private
                },
                editability: Editability::Editable,
                scope: if is_global {
                    ScopeType::Global
                } else {
                    ScopeType::Local
                },
                is_heaped: is_heaped,
                is_array: true,
            };
            self.var_metadata.insert(name.clone(), var_meta);
            if is_heaped {
                Ok(Decl::PointerDecl {
                    name,
                    inner_type: type_name,
                    length: size.clone(),
                    value,
                })
            } else {
                Ok(Decl::ArrayDecl {
                    visibility: Visibility::Private,
                    editability: Editability::Editable,
                    type_node: type_name,
                    name,
                    length: size.unwrap(),
                    value,
                })
            }
        }
    }

    // ====================================================
    // Control Flow Parsers
    // ====================================================

    // --- set <target> -> <value>; -------------------------
    pub(crate) fn parse_reassign_stmt(&mut self) -> Result<Stmt, String> {
        self.advance(); // 'set'

        let target = self.parse_expression()?;

        if self.peek().kind != TokenKind::Arrow && self.peek().kind != TokenKind::Assign {
            return Err(format!(
                "Syntax Error: Expected '->' or '=' after target in set statement at line {}, column {}",
                self.peek().line, self.peek().column
            ));
        }
        self.advance();

        let value = self.parse_expression()?;
        self.consume(TokenKind::SemiColon, "Expected ';' after set statement")?;

        Ok(Stmt::ReassignStmt { target, value })
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
        self.consume(TokenKind::LBrace, "Expected '{' to open if body")?;
        let then_block = self.parse_block("if".to_string())?;
        self.consume(TokenKind::RBrace, "Expected '}' to close if body")?;

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
                self.consume(TokenKind::LBrace, "Expected '{' after 'else'")?;
                let blk = self.parse_block("if".to_string())?;
                self.consume(TokenKind::RBrace, "Expected '}' to close else block")?;
                Some(blk)
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

    // --- loop N -> { ... }  or  loop -> { ... } (infinite) --
    // أو  loop N -> scope_name()  /  loop -> scope_name()
    pub(crate) fn parse_loop_stmt(&mut self) -> Result<Stmt, String> {
        self.advance(); // 'loop'
        let count = if self.peek().kind == TokenKind::Arrow {
            None
        } else {
            Some(self.parse_expression()?)
        };

        self.consume(
            TokenKind::Arrow,
            "Expected '->' after loop count (use: loop N -> { } or loop N -> scope())",
        )?;

        let body = if self.peek().kind == TokenKind::LBrace {
            self.advance(); // '{'
            let stmts = self.parse_block("loop".to_string())?;
            self.consume(TokenKind::RBrace, "Expected '}' to close loop body")?;
            EitherBlock::Inline(stmts)
        } else {
            let expr = self.parse_expression()?;
            if self.peek().kind == TokenKind::SemiColon {
                self.advance();
            }
            EitherBlock::External(expr)
        };

        Ok(Stmt::LoopStmt { count, body })
    }

    // --- while (cond) -> { ... }  or  while (cond) -> scope_name() ---
    pub(crate) fn parse_while_stmt(&mut self) -> Result<Stmt, String> {
        self.advance(); // 'while'
        self.consume(TokenKind::LParen, "Expected '(' after 'while'")?;
        let condition = self.parse_expression()?;
        self.consume(TokenKind::RParen, "Expected ')' after while condition")?;

        self.consume(TokenKind::Arrow, "Expected '->' after while condition (use: while (cond) -> { } or while (cond) -> scope())")?;

        let body = if self.peek().kind == TokenKind::LBrace {
            self.advance(); // '{'
            let stmts = self.parse_block("while".to_string())?;
            self.consume(TokenKind::RBrace, "Expected '}' to close while body")?;
            EitherBlock::Inline(stmts)
        } else {
            let expr = self.parse_expression()?;
            if self.peek().kind == TokenKind::SemiColon {
                self.advance();
            }
            EitherBlock::External(expr)
        };

        Ok(Stmt::WhileStmt { condition, body })
    }
    //todo : make parse_scope_decl deal with switch
    pub(crate) fn parse_switch_stmt(&mut self) -> Result<Stmt, String> {
        self.advance(); // consume 'switch'
        self.consume(TokenKind::LParen, "Expected '(' after switch")?;
        let condition = self.parse_expression()?;
        self.consume(TokenKind::RParen, "Expected ')' after switch condition")?;

        self.consume(TokenKind::Arrow, "Expected '->' after switch condition")?;

        let cases = if self.peek().kind == TokenKind::LBrace {
            self.advance(); // consume '{'
            let mut body = Vec::new();
            while !self.is_at_end() && self.peek().kind != TokenKind::RBrace {
                if self.peek().kind == TokenKind::Case {
                    self.advance(); // consume 'case'
                    let val = self.parse_expression()?;
                    self.consume(TokenKind::FatArrow, "Expected '=>' after case value")?;

                    let case_body = if self.peek().kind == TokenKind::LBrace {
                        self.advance();
                        let b = self.parse_block("case".to_string())?;
                        self.consume(TokenKind::RBrace, "Expected '}' to close case block")?;
                        b
                    } else {
                        return Err("Syntax Error: Expected '{' after '=>' in case".to_string());
                    };

                    body.push(Stmt::CaseStmt {
                        option: val,
                        set: Expr::Identifier("void".to_string()),
                        body: case_body,
                    });
                } else if self.peek().kind == TokenKind::Underscore {
                    self.advance(); // consume '_'
                    self.consume(TokenKind::FatArrow, "Expected '=>' after default case")?;

                    let def_body = if self.peek().kind == TokenKind::LBrace {
                        self.advance();
                        let b = self.parse_block("switch".to_string())?;
                        self.consume(TokenKind::RBrace, "Expected '}' to close default block")?;
                        b
                    } else {
                        return Err(
                            "Syntax Error: Expected '{' after '=>' in default case".to_string()
                        );
                    };
                    // todo: add SwitchDecl for future updates
                    body.push(Stmt::CaseStmt {
                        option: Expr::Identifier("void".to_string()),
                        set: Expr::Identifier("void".to_string()),
                        body: def_body,
                    });
                } else {
                    return Err(
                        "Syntax Error: Expected 'case' or '_' inside switch block".to_string()
                    );
                }
            }
            self.consume(TokenKind::RBrace, "Expected '}' to close switch block")?;
            body
        } else {
            return Err(
                "External switch scopes are not supported yet; use a switch block".to_string(),
            );
        };

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

    pub(crate) fn parse_for_stmt(&mut self) -> Result<Stmt, String> {
        self.advance(); // 'for'
        self.consume(TokenKind::LParen, "Expected '(' after 'for'")?;

        // Lookahead to see if it's a for-in loop
        let mut is_for_in = false;
        let mut lookahead = self.current;
        let mut paren_depth = 1; // We already consumed the first LParen
        while lookahead < self.tokens.len() {
            match &self.tokens[lookahead].kind {
                TokenKind::In => {
                    if paren_depth == 1 {
                        is_for_in = true;
                        break;
                    }
                }
                TokenKind::LParen => {
                    paren_depth += 1;
                }
                TokenKind::RParen => {
                    paren_depth -= 1;
                    if paren_depth == 0 {
                        break;
                    }
                }
                TokenKind::SemiColon => {
                    if paren_depth == 1 {
                        break;
                    }
                }
                _ => {}
            }
            lookahead += 1;
        }

        if is_for_in {
            return self.parse_for_in_stmt_body();
        }

        let init = if self.peek().kind == TokenKind::SemiColon {
            self.advance(); // skip ';'
            None
        } else {
            let stmt = match self.peek().kind {
                TokenKind::Const
                | TokenKind::TypeInt
                | TokenKind::TypeFloat
                | TokenKind::TypeChar
                | TokenKind::TypeBool
                | TokenKind::TypeName => {
                    self.parse_var_decl(false, false).map(Stmt::Declaration)?
                }
                _ => self.parse_expression_stmt()?,
            };
            Some(Box::new(stmt))
        };

        let condition = if self.peek().kind == TokenKind::SemiColon {
            None
        } else {
            Some(self.parse_expression()?)
        };
        self.consume(TokenKind::SemiColon, "Expected ';' after for condition")?;

        let increment = if self.peek().kind == TokenKind::RParen {
            None
        } else {
            let expr = self.parse_expression()?;
            let op = self.peek().kind.clone();
            if op == TokenKind::Arrow
                || op == TokenKind::Assign
                || op == TokenKind::PlusAssign
                || op == TokenKind::MinusAssign
                || op == TokenKind::MulAssign
                || op == TokenKind::DivAssign
            {
                self.advance();
                let mut value = self.parse_expression()?;
                if op == TokenKind::PlusAssign {
                    value = Expr::BinaryOp {
                        left: Box::new(expr.clone()),
                        operator: "+".to_string(),
                        right: Box::new(value),
                    };
                } else if op == TokenKind::MinusAssign {
                    value = Expr::BinaryOp {
                        left: Box::new(expr.clone()),
                        operator: "-".to_string(),
                        right: Box::new(value),
                    };
                } else if op == TokenKind::MulAssign {
                    value = Expr::BinaryOp {
                        left: Box::new(expr.clone()),
                        operator: "*".to_string(),
                        right: Box::new(value),
                    };
                } else if op == TokenKind::DivAssign {
                    value = Expr::BinaryOp {
                        left: Box::new(expr.clone()),
                        operator: "/".to_string(),
                        right: Box::new(value),
                    };
                }
                Some(Box::new(Stmt::ReassignStmt {
                    target: expr,
                    value,
                }))
            } else {
                Some(Box::new(Stmt::ExpressionStmt(expr)))
            }
        };
        self.consume(TokenKind::RParen, "Expected ')' after for clauses")?;

        self.consume(TokenKind::Arrow, "Expected '->' after 'for' clauses")?;

        let body = if self.peek().kind == TokenKind::LBrace {
            self.advance();
            let stmts = self.parse_block("for".to_string())?;
            self.consume(TokenKind::RBrace, "Expected '}' to close for body")?;
            EitherBlock::Inline(stmts)
        } else {
            let expr = self.parse_expression()?;
            if self.peek().kind == TokenKind::SemiColon {
                self.advance();
            }
            EitherBlock::External(expr)
        };

        Ok(Stmt::ForStmt {
            init,
            condition,
            increment,
            body,
        })
    }

    pub(crate) fn parse_for_in_stmt_body(&mut self) -> Result<Stmt, String> {
        // We already consumed `for (`
        // Now we parse the item
        let item = match self.peek().kind {
            TokenKind::Const
            | TokenKind::TypeInt
            | TokenKind::TypeFloat
            | TokenKind::TypeChar
            | TokenKind::TypeBool => self.parse_var_decl(false, true).map(Stmt::Declaration)?, // pass true for `no_semi`
            TokenKind::TypeName => self.parse_name().map(Stmt::Declaration)?,
            _ => {
                let expr = self.parse_expression()?;
                Stmt::ExpressionStmt(expr)
            }
        };

        self.consume(TokenKind::In, "Expected 'in' in for-in loop")?;
        let iterable = self.parse_expression()?;
        self.consume(TokenKind::RParen, "Expected ')' after for-in clauses")?;
        self.consume(TokenKind::Arrow, "Expected '->' after 'for-in' clauses")?;

        let body = if self.peek().kind == TokenKind::LBrace {
            self.advance();
            let stmts = self.parse_block("for-in".to_string())?;
            self.consume(TokenKind::RBrace, "Expected '}' to close for-in body")?;
            EitherBlock::Inline(stmts)
        } else {
            let expr = self.parse_expression()?;
            if self.peek().kind == TokenKind::SemiColon {
                self.advance();
            }
            EitherBlock::External(expr)
        };

        Ok(Stmt::ForInStmt {
            item: Box::new(item),
            iterable,
            body,
        })
    }

    pub(crate) fn parse_throw_stmt(&mut self) -> Result<Stmt, String> {
        //todo : we will make throw work only in fn and custom scope so you can take the name of the scope as a parameter
        //todo :  and update the scope data the flag has_throw to true and the error flied to the string that we will throw to  allow the user to catch it or handle it using the scope handler
        //! check 21_throw.fs to see how it will work
        self.advance(); // 'throw'

        // Optional 'new' (like: throw new error("...")) //todo: we will make it not optional
        if self.peek().kind == TokenKind::New {
            self.advance();
        }

        let expr = self.parse_expression()?;
        self.consume(TokenKind::SemiColon, "Expected ';' after throw statement")?;
        Ok(Stmt::ThrowStmt(expr))
    }

    pub(crate) fn parse_goto_stmt(&mut self, scope: String) -> Result<Stmt, String> {
        if scope.is_empty() {
            return Err(
                "Syntax Error: You can't use goto outside of a label or the call method"
                    .to_string(),
            );
        } else if !scope.contains("@") || !scope.contains("call") {
            return Err(
                "Syntax Error: You can't use goto outside of a label or the call method"
                    .to_string(),
            );
        }
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

    pub(crate) fn parse_call_stmt(&mut self) -> Result<Stmt, String> {
        self.advance(); // consume 'call'
        if self.peek().kind == TokenKind::Arrow {
            self.advance();
        }
        let target = self.parse_expression()?;
        self.consume(TokenKind::SemiColon, "Expected ';' after call statement")?;
        Ok(Stmt::CallStmt(target))
    }

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
        if let TokenKind::Identifier(_) = self.peek().kind.clone() {
            let next1 = self.tokens.get(self.current + 1).map(|t| &t.kind);
            let next2 = self.tokens.get(self.current + 2).map(|t| &t.kind);

            // `TypeName varName ->` pattern
            let is_var_decl = match (next1, next2) {
                (Some(TokenKind::Identifier(_)), Some(TokenKind::Arrow)) => true,
                (Some(TokenKind::Less), _) => {
                    // `TypeName<...> varName ->` - بنفحص أبعد
                    // نبحث عن `>` ثم Identifier ثم `->`
                    let mut i = self.current + 2;
                    let mut depth = 1;
                    while i < self.tokens.len() && depth > 0 {
                        match &self.tokens[i].kind {
                            TokenKind::Less => depth += 1,
                            TokenKind::Greater => depth -= 1,
                            _ => {}
                        }
                        i += 1;
                    }
                    // بعد الـ `>`: هل في Identifier ثم `->`؟
                    matches!(
                        (
                            self.tokens.get(i).map(|t| &t.kind),
                            self.tokens.get(i + 1).map(|t| &t.kind)
                        ),
                        (Some(TokenKind::Identifier(_)), Some(TokenKind::Arrow))
                    )
                }
                _ => false,
            };

            if is_var_decl {
                return self.parse_var_decl(true, false).map(Stmt::Declaration);
            }
        }

        let expr = self.parse_expression()?;

        // `target -> value;` or `target = value;`
        let op = self.peek().kind.clone();
        if op == TokenKind::Arrow
            || op == TokenKind::Assign
            || op == TokenKind::PlusAssign
            || op == TokenKind::MinusAssign
            || op == TokenKind::MulAssign
            || op == TokenKind::DivAssign
        {
            self.advance(); // consume '->' or '=' or '+=' etc.
            let mut value = self.parse_expression()?;

            if op == TokenKind::PlusAssign {
                value = Expr::BinaryOp {
                    left: Box::new(expr.clone()),
                    operator: "+".to_string(),
                    right: Box::new(value),
                };
            } else if op == TokenKind::MinusAssign {
                value = Expr::BinaryOp {
                    left: Box::new(expr.clone()),
                    operator: "-".to_string(),
                    right: Box::new(value),
                };
            } else if op == TokenKind::MulAssign {
                value = Expr::BinaryOp {
                    left: Box::new(expr.clone()),
                    operator: "*".to_string(),
                    right: Box::new(value),
                };
            } else if op == TokenKind::DivAssign {
                value = Expr::BinaryOp {
                    left: Box::new(expr.clone()),
                    operator: "/".to_string(),
                    right: Box::new(value),
                };
            }

            self.consume(TokenKind::SemiColon, "Expected ';' after reassignment")?;
            return Ok(Stmt::ReassignStmt {
                target: expr,
                value,
            });
        }
        print!("DEBUG: parse_expression_reassign_stmt: expr: {:?}", expr);
        if self.peek().kind == TokenKind::SemiColon {
            self.consume(
                TokenKind::SemiColon,
                "Expected ';' after expression statement",
            )?;
        }
        Ok(Stmt::ExpressionStmt(expr))
    }

    /// يُعالج تعريف متغير باستخدام Custom Keyword:
    /// `my_list<int(32)> items -> [1, 2];`
    /// حيث `keyword_name` = "my_list" و `original_scope_name` = "array"
    // todo: remove this
    /*   pub(crate) fn parse_custom_keyword_var_decl(
            &mut self,
            keyword_name: String,
            original_scope_name: String,
        ) -> Result<Stmt, String> {
            // parse optional generic params: <int(32)>
            let mut generics = Vec::new();
            let mut _size: Option<i64> = None;
            if self.peek().kind == crate::frontend::lexer::token::TokenKind::Less {
                self.advance();
                self.parse_generic_list(&mut generics, &mut _size, original_scope_name == "array")?;
            }

            // اسم المتغير
            let name = self.get_identifier("Expected variable name after custom type keyword")?;

            // '->' للتعيين
            self.consume(
                crate::frontend::lexer::token::TokenKind::Arrow,
                "Expected '->' after variable name in custom keyword declaration",
            )?;
            let value = self.parse_expression()?;
            self.consume(
                crate::frontend::lexer::token::TokenKind::SemiColon,
                "Expected ';' after custom keyword variable declaration",
            )?;

            let type_node = if generics.is_empty() {
                 TypeNode::Simple( TypeRef {
                    base_type: keyword_name,
                    size: None,
                })
            } else {
                 TypeNode::Generic( Generic {
                    base_type: keyword_name,
                    generics,
                })
            };

            Ok(Decl::VarDecl {
                visibility:  Visibility::Private,
                editability:  Editability::Editable,
                type_node: type_node,
                name,
                value,
            })
        }
    */
    pub(crate) fn parse_expression_stmt(&mut self) -> Result<Stmt, String> {
        let expr = self.parse_expression()?;
        print!(
            "DEBUG: parse_expression_stmt: 1. expr: {:?} \n",
            self.peek().kind
        );
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
            let mut value = self.parse_expression()?;

            if op == TokenKind::PlusAssign {
                value = Expr::BinaryOp {
                    left: Box::new(expr.clone()),
                    operator: "+".to_string(),
                    right: Box::new(value),
                };
            } else if op == TokenKind::MinusAssign {
                value = Expr::BinaryOp {
                    left: Box::new(expr.clone()),
                    operator: "-".to_string(),
                    right: Box::new(value),
                };
            } else if op == TokenKind::MulAssign {
                value = Expr::BinaryOp {
                    left: Box::new(expr.clone()),
                    operator: "*".to_string(),
                    right: Box::new(value),
                };
            } else if op == TokenKind::DivAssign {
                value = Expr::BinaryOp {
                    left: Box::new(expr.clone()),
                    operator: "/".to_string(),
                    right: Box::new(value),
                };
            }

            self.consume(
                TokenKind::SemiColon,
                "Expected ';' after assignment statement",
            )?;
            return Ok(Stmt::ReassignStmt {
                target: expr,
                value,
            });
        }
        print!(
            "DEBUG: parse_expression_stmt: 3. expr: {:?} \n",
            self.peek().kind
        );
        if self.peek().kind == TokenKind::SemiColon {
            self.consume(
                TokenKind::SemiColon,
                "Expected ';' after expression statement",
            )?;
        }
        Ok(Stmt::ExpressionStmt(expr))
    }

    pub(crate) fn parse_try_catch_stmt(&mut self) -> Result<Stmt, String> {
        self.advance(); // 'try'
        self.consume(TokenKind::Arrow, "Expected '->' after 'try'")?;
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
        self.consume(TokenKind::Arrow, "Expected '->' after catch(...)")?;
        self.consume(TokenKind::LBrace, "Expected '{' to open catch block")?;
        let catch_block = self.parse_block("catch".to_string())?;
        self.consume(TokenKind::RBrace, "Expected '}' to close catch block")?;

        Ok(Stmt::TryCatchStmt {
            try_block,
            catch_param: catch_param.to_string(),
            catch_block,
        })
    }

    /// يقرأ قائمة statements حتى '}' ويستهلك الـ '}'
    /// يُستعمل في كل block: if/else/loop/while body

    // ====================================================
    // OOP & Enum Parsers
    // ====================================================

    pub(crate) fn get_identifier(&mut self, err_msg: &str) -> Result<String, String> {
        if let TokenKind::Identifier(n) = &self.peek().kind.clone() {
            let s = n.to_string();
            self.advance();
            return Ok(s);
        } else {
            return Err(err_msg.to_string());
        }
    }
}
