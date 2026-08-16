use std::collections::HashMap;

use crate::lexer::token::TokenKind;
use crate::parser::ast::*;
use crate::parser::parser::Parser;

impl Parser {
    pub(crate) fn parse_statement(&mut self) -> Result<Option<Stmt>, String> {
        eprintln!(
            "DISPATCH: {:?} at line {}",
            self.peek().kind,
            self.peek().line
        );
        let result = match &self.peek().kind {
            TokenKind::SemiColon => {
                self.advance();
                return Ok(None);
            }
            TokenKind::Import => self.parse_import_stmt(),
            TokenKind::Export => self.parse_exported_stmt(),
            TokenKind::Const
            | TokenKind::TypeInt
            | TokenKind::TypeFloat
            | TokenKind::TypeChar
            | TokenKind::TypeBool
            | TokenKind::TypeName
            | TokenKind::TypeType
            | TokenKind::TypeObject
            | TokenKind::TypeCustom => self.parse_var_decl(true, false),
            TokenKind::TypeBluePrint => self.parse_blueprint_decl(),
            TokenKind::Impl => self.parse_impl_decl(),
            TokenKind::Set => self.parse_reassign_stmt(),
            TokenKind::If => self.parse_if_stmt(),
            TokenKind::For => self.parse_for_stmt(),
            TokenKind::Loop => self.parse_loop_stmt(),
            TokenKind::While => self.parse_while_stmt(),
            TokenKind::Switch => self.parse_switch_stmt(),
            TokenKind::TypeScope => self.parse_scope_decl(),
            TokenKind::Fn => self.parse_fn_decl("".to_string()),
            TokenKind::TypeClass => self.parse_class_decl("".to_string()),
            TokenKind::TypeStruct => self.parse_struct_decl("".to_string()),
            TokenKind::TypeEnum => self.parse_enum_decl("".to_string()),
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
            TokenKind::Goto => self.parse_goto_stmt(),
            TokenKind::LabelName(_) => self.parse_label_decl(),
            _ => self.parse_expression_stmt(),
        };

        match result {
            Ok(stmt) => Ok(Some(stmt)),
            Err(err) => {
                let err_msg = if err.starts_with("Syntax Error:") {
                    err.clone()
                } else {
                    format!("Syntax Error: {}", err)
                };
                eprintln!("{}", err_msg);
                self.synchronize();
                Err(err_msg)
            }
        }
    }

    pub(crate) fn parse_import_stmt(&mut self) -> Result<Stmt, String> {
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

        Ok(Stmt::Import {
            module_path,
            imports,
        })
    }

    pub(crate) fn parse_exported_stmt(&mut self) -> Result<Stmt, String> {
        self.advance(); // consume 'export'

        // After export, we expect a valid exportable statement (fn, scope, let, class, struct, enum)
        let mut stmt = match &self.peek().kind {
            TokenKind::Fn => self.parse_fn_decl("".to_string())?,
            TokenKind::TypeScope => self.parse_scope_decl()?,
            TokenKind::TypeClass => self.parse_class_decl("".to_string())?,
            TokenKind::TypeStruct => self.parse_struct_decl("".to_string())?,
            TokenKind::TypeEnum => self.parse_enum_decl("".to_string())?,
            kind => return Err(format!("Syntax Error: Cannot export '{:?}', only let, fn, scope, class, struct, and enum can be exported", kind)),
        };

        // Set is_exported flag to true
        match &mut stmt {
            Stmt::FnDecl { is_exported, .. } => *is_exported = true,
            Stmt::BlockDecl { is_exported, .. } => *is_exported = true,
            Stmt::CustomDecl { is_exported, .. } => *is_exported = true,
            Stmt::ClassDecl { is_exported, .. } => *is_exported = true,
            Stmt::StructDecl { is_exported, .. } => *is_exported = true,
            Stmt::EnumDecl { is_exported, .. } => *is_exported = true,
            Stmt::VarDecl { visibility, .. } => *visibility = Visibility::Public,
            Stmt::ArrayDecl { visibility, .. } => *visibility = Visibility::Public,
            _ => {}
        }

        Ok(stmt)
    }

    pub(crate) fn parse_var_decl(&mut self, is_global: bool, no_semi: bool) -> Result<Stmt, String> {
        let is_const = if self.peek().kind == TokenKind::Const {
            self.advance();
            true
        } else {
            false
        };
        let mut var_meta: VarMetadata;
        let type_node = self.parse_type()?;
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
            self.consume(
                TokenKind::SemiColon,
                "Expected ';' after variable declaration",
            )?;
        }
        if size.is_none() {
            var_meta = VarMetadata {
                name: name.clone(),
                type_node: type_node.clone(),
                visibility: if is_global {
                    Visibility::Public
                } else {
                    Visibility::Private
                },
                editability: if is_const {
                    Editability::NotEditable
                } else {
                    Editability::Editable
                },
                is_array: false,
            };
            if is_global {
                self.var_metadata.insert(name.clone(), var_meta);
            }
            Ok(Stmt::VarDecl {
                visibility: Visibility::Private,
                editability: if is_const {
                    Editability::NotEditable
                } else {
                    Editability::Editable
                },
                type_node: Some(type_node),
                name,
                value,
            })
        } else {
            var_meta = VarMetadata {
                name: name.clone(),
                type_node: type_node.clone(),
                visibility: if is_global {
                    Visibility::Public
                } else {
                    Visibility::Private
                },
                editability: if is_const {
                    Editability::NotEditable
                } else {
                    Editability::Editable
                },
                is_array: true,
            };
            if is_global {
                self.var_metadata.insert(name.clone(), var_meta);
            }
            Ok(Stmt::ArrayDecl {
                visibility: Visibility::Private,
                editability: if is_const {
                    Editability::NotEditable
                } else {
                    Editability::Editable
                },
                type_node: Some(type_node),
                name,
                length: size.unwrap(),
                value,
            })
        }
    }

    // ====================================================
    // Control Flow Parsers
    // ====================================================

    // --- set <target> -> <value>; -------------------------
    // target: identifier  أو  property chain (obj.field.sub)
    pub(crate) fn parse_reassign_stmt(&mut self) -> Result<Stmt, String> {
        self.advance(); // 'set'

        // نقرأ الـ target كعبارة (Expression)
        // ده بيسمح بـ this.x أو arr[0] أو x
        let target = self.parse_expression()?;

        // نقرأ علامة التعيين (-> أو =)
        if self.peek().kind != TokenKind::Arrow && self.peek().kind != TokenKind::Assign {
            return Err(format!(
                "Syntax Error: Expected '->' or '=' after target in set statement at line {}, column {}",
                self.peek().line, self.peek().column
            ));
        }
        self.advance(); // نتخطى '->' أو '='

        let value = self.parse_expression()?;
        self.consume(TokenKind::SemiColon, "Expected ';' after set statement")?;

        Ok(Stmt::ReassignStmt { target, value })
    }

    // --- if (cond) { ... } else { ... } -------------------
    pub(crate) fn parse_if_stmt(&mut self) -> Result<Stmt, String> {
        self.advance(); // 'if'

        // الشرط جوا ()
        self.consume(TokenKind::LParen, "Expected '(' after 'if'")?;
        let condition = self.parse_expression()?;
        self.consume(TokenKind::RParen, "Expected ')' after if condition")?;

        // '->' اختياري قبل الـ body
        if self.peek().kind == TokenKind::Arrow {
            self.advance();
        }

        // then block
        self.consume(TokenKind::LBrace, "Expected '{' to open if body")?;
        let then_block = self.parse_block()?;
        self.consume(TokenKind::RBrace, "Expected '}' to close if body")?;

        // else block — اختياري
        let else_block = if self.peek().kind == TokenKind::Else {
            self.advance(); // 'else'

            // else if  أو  else { ... }
            if self.peek().kind == TokenKind::If {
                // else if: نقرأها كـ IfStmt جوا else block
                let nested = self.parse_if_stmt()?;
                Some(vec![nested])
            } else {
                if self.peek().kind == TokenKind::Arrow {
                    self.advance();
                }
                self.consume(TokenKind::LBrace, "Expected '{' after 'else'")?;
                let blk = self.parse_block()?;
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

    // --- loop N -> { ... }  أو  loop -> { ... } (infinite) --
    // أو  loop N -> scope_name()  /  loop -> scope_name()
    pub(crate) fn parse_loop_stmt(&mut self) -> Result<Stmt, String> {
        self.advance(); // 'loop'

        // لو اللي بعده مباشرة '->' ده infinite loop
        let count = if self.peek().kind == TokenKind::Arrow {
            None
        } else {
            Some(self.parse_expression()?)
        };

        self.consume(
            TokenKind::Arrow,
            "Expected '->' after loop count (use: loop N -> { } or loop N -> scope())",
        )?;

        // '->' متبوعة بـ '{' = inline block,  غير كدة = scope call
        let body = if self.peek().kind == TokenKind::LBrace {
            self.advance(); // '{'
            let stmts = self.parse_block()?;
            self.consume(TokenKind::RBrace, "Expected '}' to close loop body")?;
            EitherBlock::Inline(stmts)
        } else {
            // scope_name(args) أو scope_name بدون أرغومنتس
            let expr = self.parse_expression()?;
            if self.peek().kind == TokenKind::SemiColon {
                self.advance();
            }
            EitherBlock::External(expr)
        };

        Ok(Stmt::LoopStmt { count, body })
    }

    // --- while (cond) -> { ... }  أو  while (cond) -> scope_name() ---
    pub(crate) fn parse_while_stmt(&mut self) -> Result<Stmt, String> {
        self.advance(); // 'while'
        self.consume(TokenKind::LParen, "Expected '(' after 'while'")?;
        let condition = self.parse_expression()?;
        self.consume(TokenKind::RParen, "Expected ')' after while condition")?;

        self.consume(TokenKind::Arrow, "Expected '->' after while condition (use: while (cond) -> { } or while (cond) -> scope())")?;

        // '->' متبوعة بـ '{' = inline block, غير كدة = scope call
        let body = if self.peek().kind == TokenKind::LBrace {
            self.advance(); // '{'
            let stmts = self.parse_block()?;
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
                        let b = self.parse_block()?;
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
                        let b = self.parse_block()?;
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
        self.consume(TokenKind::SemiColon, "Expected ';' after del statement")?;
        Ok(Stmt::DelStmt(expr))
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
                | TokenKind::TypeName => self.parse_var_decl(false, false)?,
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
        }  else {
            let expr = self.parse_expression()?;
            let op = self.peek().kind.clone();
            if op == TokenKind::Arrow || op == TokenKind::Assign || op == TokenKind::PlusAssign || op == TokenKind::MinusAssign || op == TokenKind::MulAssign || op == TokenKind::DivAssign {
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
                Some(Box::new(Stmt::ReassignStmt { target: expr, value }))
            } else {
                Some(Box::new(Stmt::ExpressionStmt(expr)))
            }
        };
        self.consume(TokenKind::RParen, "Expected ')' after for clauses")?;

        self.consume(TokenKind::Arrow, "Expected '->' after 'for' clauses")?;

        let body = if self.peek().kind == TokenKind::LBrace {
            self.advance();
            let stmts = self.parse_block()?;
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
            | TokenKind::TypeBool
            | TokenKind::TypeName => self.parse_var_decl(false, true)?, // pass true for `no_semi`
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
            let stmts = self.parse_block()?;
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

    pub(crate) fn parse_label_decl(&mut self) -> Result<Stmt, String> {
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
                if let Some(stmt) = self.parse_statement()? {
                    body.push(stmt);
                }
            }
            self.consume(TokenKind::RBrace, "Expected '}' after label block")?;
        } else if self.peek().kind == TokenKind::SemiColon {
            self.advance(); // consume ';'
        } else {
            return Err(format!("Expected '{{' or ';' after label '{}'", label_name));
        }

        Ok(Stmt::LabelDecl {
            name: label_name,
            body,
        })
    }

    pub(crate) fn parse_goto_stmt(&mut self) -> Result<Stmt, String> {
        self.advance(); // consume 'goto'
        if self.peek().kind == TokenKind::Arrow {
            self.advance();
        }
        let target = if let TokenKind::LabelName(name) = self.peek().kind.clone() {
            self.advance();
            name
        } else if let TokenKind::Identifier(name) = self.peek().kind.clone() {
            self.advance();
            name
        } else {
            return Err("Expected label name after 'goto'".to_string());
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
                return self.parse_var_decl(true, false);
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
            if self.peek().kind == crate::lexer::token::TokenKind::Less {
                self.advance();
                self.parse_generic_list(&mut generics, &mut _size, original_scope_name == "array")?;
            }

            // اسم المتغير
            let name = self.get_identifier("Expected variable name after custom type keyword")?;

            // '->' للتعيين
            self.consume(
                crate::lexer::token::TokenKind::Arrow,
                "Expected '->' after variable name in custom keyword declaration",
            )?;
            let value = self.parse_expression()?;
            self.consume(
                crate::lexer::token::TokenKind::SemiColon,
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

            Ok(Stmt::VarDecl {
                visibility:  Visibility::Private,
                editability:  Editability::Editable,
                type_node: Some(type_node),
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
        let try_block = self.parse_block()?;
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
        let catch_block = self.parse_block()?;
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
