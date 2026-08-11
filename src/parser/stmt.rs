use crate::lexer::token::{Token, TokenKind};
use crate::parser::ast::*;
use crate::parser::parser::Parser;

impl Parser {
    pub(crate) fn parse_statement(&mut self) -> Result<Option<Stmt>, String> {
        let result = match &self.peek().kind {
            TokenKind::Use => self.parse_use_stmt(),
            TokenKind::Export => self.parse_exported_stmt(),
            TokenKind::Let | TokenKind::Const => self.parse_var_decl(),
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
            kind if self.is_type_token(kind) => self.parse_var_decl_bare(),
            _ => self.parse_expression_stmt(),
        };

        match result {
            Ok(stmt) => Ok(Some(stmt)),
            Err(err) => {
                eprintln!("Syntax Error: {}", err);
                self.synchronize();
                Err(format!("Syntax Error: {}", err))
            }
        }
    }

    pub(crate) fn parse_use_stmt(&mut self) -> Result<Stmt, String> {
        self.advance(); // consume 'use'

        let mut module_path = Vec::new();
        let mut imports = None;

        loop {
            let name = if let TokenKind::Identifier(n) = &self.peek().kind {
                let n = n.clone();
                self.advance();
                n
            } else if let Some(kw) = Self::keyword_as_identifier(&self.peek().kind.clone()) {
                self.advance();
                kw
            } else {
                return Err(
                    "Syntax Error: Expected module or import name in use statement".to_string(),
                );
            };

            module_path.push(name);

            if self.peek().kind == TokenKind::DoubleColon {
                self.advance();
                // Check if next is '{'
                if self.peek().kind == TokenKind::LBrace {
                    self.advance();
                    let mut selected = Vec::new();
                    if self.peek().kind != TokenKind::RBrace {
                        loop {
                            let n = if let TokenKind::Identifier(n) = &self.peek().kind {
                                let n = n.clone();
                                self.advance();
                                n
                            } else {
                                return Err("Syntax Error: Expected imported name".to_string());
                            };
                            selected.push(n);
                            if self.peek().kind == TokenKind::Comma {
                                self.advance();
                            } else {
                                break;
                            }
                        }
                    }
                    self.consume(TokenKind::RBrace, "Expected '}' after import list")?;
                    imports = Some(selected);
                    break; // After '{...}', use statement ends
                }
            } else {
                break; // No '::', so path is done
            }
        }

        self.consume(TokenKind::SemiColon, "Expected ';' after use statement")?;
        Ok(Stmt::Use {
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
            TokenKind::Let => self.parse_var_decl()?,
            kind => return Err(format!("Syntax Error: Cannot export '{:?}', only let, fn, scope, class, struct, and enum can be exported", kind)),
        };

        // Set is_exported flag to true
        match &mut stmt {
            Stmt::FnDecl { is_exported, .. } => *is_exported = true,
            Stmt::BlockDecl { is_exported, .. } => *is_exported = true,
            Stmt::CustomDecl { is_exported, .. } => *is_exported = true,
            Stmt::ClassDecl { is_exported, .. } => *is_exported = true,
            Stmt::StructDecl { is_exported, .. } => *is_exported = true,
            Stmt::ArrayDecl { is_exported, .. } => *is_exported = true,
            Stmt::StrDecl { is_exported, .. } => *is_exported = true,
            Stmt::EnumDecl { is_exported, .. } => *is_exported = true,
            Stmt::VarDecl { visibility, .. } => {
                *visibility = crate::parser::ast::Visibility::Public
            }
            _ => {}
        }

        Ok(stmt)
    }

    // يحلل تعريف المتغير: let int : 8 a = 5; أو let a : i8 = 5; أو const ...
    pub(crate) fn parse_var_decl(&mut self) -> Result<Stmt, String> {
        let is_const = self.peek().kind == TokenKind::Const;
        self.advance(); // نتخطى كلمة 'let' أو 'const'

        // 1. تحديد الـ Base Type والـ Size
        let type_node = match self.parse_type() {
            Ok(t) => Some(t),
            Err(e) => {
                if e == "Expected a type" {
                    None
                } else {
                    return Err(e);
                }
            }
        };

        // 3. قراءة اسم المتغير (Identifier)
        //    نقبل كمان بعض الـ keywords كـ identifiers في موضع الاسم (زي flag, length, etc.)
        let name = if let TokenKind::Identifier(n) = &self.peek().kind {
            let var_name = n.clone();
            self.advance(); // نتخطى الاسم
            var_name
        } else if let Some(kw_name) = Self::keyword_as_identifier(&self.peek().kind.clone()) {
            self.advance();
            kw_name
        } else {
            return Err(format!(
                "Syntax Error: Expected variable name at line {}, column {}",
                self.peek().line,
                self.peek().column
            ));
        };

        // 4. قبول التهيئة الاختيارية: let <type> <name>; أو let <type> <name> = <value>;
        let value = if self.peek().kind == TokenKind::Assign {
            self.advance(); // نتخطى الـ '='
            self.parse_expression()?
        } else {
            if self.peek().kind == TokenKind::Arrow {
                return Err(format!("Syntax Error: Expected '=' to assign value to '{}'. Use '->' for reassignment (set), not declaration (let).", name));
            }
            Expr::Identifier("__param__".to_string())
        };

        // 6. التأكد من وجود الفصلة المنقوطة ';'
        if self.peek().kind == TokenKind::SemiColon {
            self.advance(); // نتخطى الـ ';'
        } else {
            return Err("Syntax Error: Missing ';' at the end of declaration".to_string());
        }

        // لو كل حاجة تمام، نرجع الـ Node بتاعة الـ AST
        Ok(Stmt::VarDecl {
            visibility: crate::parser::ast::Visibility::Private,
            editability: if is_const {
                crate::parser::ast::Editability::NotEditable
            } else {
                crate::parser::ast::Editability::Editable
            },
            type_node,
            name,
            value,
        })
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
                Some(self.parse_block()?)
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
            crate::parser::ast::EitherBlock::Inline(stmts)
        } else {
            // scope_name(args) أو scope_name بدون أرغومنتس
            let expr = self.parse_expression()?;
            if self.peek().kind == TokenKind::SemiColon {
                self.advance();
            }
            crate::parser::ast::EitherBlock::External(expr)
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
            crate::parser::ast::EitherBlock::Inline(stmts)
        } else {
            let expr = self.parse_expression()?;
            if self.peek().kind == TokenKind::SemiColon {
                self.advance();
            }
            crate::parser::ast::EitherBlock::External(expr)
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
                        self.parse_block()?
                    } else {
                        return Err("Syntax Error: Expected '{' after '=>' in case".to_string());
                    };

                    body.push(Stmt::ScopeDecl {
                        is_exported: false,
                        is_const: false,
                        name: "case".to_string(),
                        scope_type: crate::parser::ast::ScopeType::Case,
                        params: vec![],
                        return_type: None,
                        flags: vec![],
                        settings: vec![],
                        events: vec![],
                        generic_block: vec![],
                        static_block: vec![],
                        handle_block: vec![],
                        custom_keyword: None,
                        statements: case_body,
                        public_block: vec![],
                        fields: vec![],
                        private_block: vec![],
                        return_value: Some(val),
                        constructor: None,
                    });
                } else if self.peek().kind == TokenKind::Underscore {
                    self.advance(); // consume '_'
                    self.consume(TokenKind::FatArrow, "Expected '=>' after default case")?;

                    let def_body = if self.peek().kind == TokenKind::LBrace {
                        self.advance();
                        self.parse_block()?
                    } else {
                        return Err(
                            "Syntax Error: Expected '{' after '=>' in default case".to_string()
                        );
                    };
                    // todo: add SwitchDecl for future updates
                    body.push(Stmt::ScopeDecl {
                        is_exported: false,
                        is_const: false,
                        name: "default".to_string(),
                        scope_type: crate::parser::ast::ScopeType::Case,
                        params: vec![],
                        return_type: None,
                        flags: vec![],
                        settings: vec![],
                        events: vec![],
                        generic_block: vec![],
                        static_block: vec![],
                        statements: def_body,
                        handle_block: vec![],
                        custom_keyword: None,
                        public_block: vec![],
                        fields: vec![],
                        private_block: vec![],
                        return_value: None,
                        constructor: None,
                    });
                } else {
                    return Err(
                        "Syntax Error: Expected 'case' or '_' inside switch block".to_string()
                    );
                }
            }
            self.consume(TokenKind::RBrace, "Expected '}' to close switch block")?;
            crate::parser::ast::EitherBlock::Inline(body)
        } else {
            let external_scope =
                self.get_identifier("Expected external scope name for switch cases")?;
            self.consume(
                TokenKind::SemiColon,
                "Expected ';' after external scope reference in switch",
            )?;
            crate::parser::ast::EitherBlock::External(crate::parser::ast::Expr::Identifier(
                external_scope,
            ))
        };

        Ok(Stmt::SwitchStmt { condition, cases })
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

        let init = if self.peek().kind == TokenKind::SemiColon {
            self.advance(); // skip ';'
            None
        } else {
            let stmt = if self.peek().kind == TokenKind::Let {
                self.parse_var_decl()?
            } else if self.peek().kind == TokenKind::Set {
                self.parse_reassign_stmt()?
            } else {
                self.parse_expression_stmt()?
            };
            // Note: parse_var_decl, parse_reassign_stmt, and parse_expression_stmt consume the ';'
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
            Some(self.parse_expression()?)
        };
        self.consume(TokenKind::RParen, "Expected ')' after for clauses")?;

        self.consume(TokenKind::Arrow, "Expected '->' after 'for' clauses")?;

        let body = if self.peek().kind == TokenKind::LBrace {
            self.advance();
            let stmts = self.parse_block()?;
            crate::parser::ast::EitherBlock::Inline(stmts)
        } else {
            let expr = self.parse_expression()?;
            if self.peek().kind == TokenKind::SemiColon {
                self.advance();
            }
            crate::parser::ast::EitherBlock::External(expr)
        };

        Ok(Stmt::ForStmt {
            init,
            condition,
            increment,
            body,
        })
    }

    pub(crate) fn parse_throw_stmt(&mut self) -> Result<Stmt, String> {
        self.advance(); // 'throw'

        // Optional 'new' (like: throw new error("..."))
        if self.peek().kind == TokenKind::New {
            self.advance();
        }

        let expr = self.parse_expression()?;
        self.consume(TokenKind::SemiColon, "Expected ';' after throw statement")?;
        Ok(Stmt::ThrowStmt(expr))
    }

    pub(crate) fn parse_expression_stmt(&mut self) -> Result<Stmt, String> {
        let expr = self.parse_expression()?;

        // --- Bare reassignment: x = 10; or this.x = 20; ---
        if self.peek().kind == TokenKind::Assign || self.peek().kind == TokenKind::Arrow {
            self.advance(); // consume '=' or '->'
            let value = self.parse_expression()?;
            self.consume(
                TokenKind::SemiColon,
                "Expected ';' after assignment statement",
            )?;
            return Ok(Stmt::ReassignStmt {
                target: expr,
                value,
            });
        }

        // --- Bare declaration for user-defined types: MyClass x = 10; ---
        // If the next token is an identifier, it means `expr` was actually the type!
        if let TokenKind::Identifier(var_name) = &self.peek().kind.clone() {
            let name = var_name.clone();
            self.advance(); // consume variable name

            let mut value = Expr::Identifier("__param__".to_string());
            if self.peek().kind == TokenKind::Assign {
                self.advance();
                value = self.parse_expression()?;
            }

            if self.peek().kind == TokenKind::SemiColon {
                self.advance();
            }

            // Extract base_type and size from expr
            let (base_type, size) = match expr {
                Expr::Identifier(t) => (Some(t), None),
                Expr::Call { callee, args } => {
                    // e.g. MyClass(32) parsed as Call
                    if let Expr::Identifier(t) = *callee {
                        if args.len() == 1 {
                            if let Expr::LiteralInt(s) = args[0] {
                                (Some(t), Some(s))
                            } else {
                                (Some(t), None)
                            }
                        } else {
                            (Some(t), None)
                        }
                    } else {
                        (None, None)
                    }
                }
                _ => (None, None),
            };

            return Ok(Stmt::VarDecl {
                visibility: crate::parser::ast::Visibility::Public,
                editability: crate::parser::ast::Editability::Editable,
                type_node: Some(crate::parser::ast::TypeNode::Simple(
                    crate::parser::ast::TypeRef {
                        base_type: base_type.unwrap_or_else(|| "unknown".to_string()),
                        size,
                    },
                )),
                name: name.to_string(),
                value,
            });
        }

        self.consume(
            TokenKind::SemiColon,
            "Expected ';' after expression statement",
        )?;
        Ok(Stmt::ExpressionStmt(expr))
    }

    pub(crate) fn parse_try_catch_stmt(&mut self) -> Result<Stmt, String> {
        self.advance(); // 'try'
        self.consume(TokenKind::Arrow, "Expected '->' after 'try'")?;
        self.consume(TokenKind::LBrace, "Expected '{' to open try block")?;
        let try_block = self.parse_block()?;

        self.consume(TokenKind::Catch, "Expected 'catch' after try block")?;
        self.consume(TokenKind::LParen, "Expected '(' after 'catch'")?;

        let catch_param = if let TokenKind::Identifier(n) = &self.peek().kind.clone() {
            let n = n.clone();
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
            let n = n.clone();
            self.advance();
            Ok(n)
        } else if let Some(kw) = Self::keyword_as_identifier(&self.peek().kind.clone()) {
            self.advance();
            Ok(kw)
        } else {
            Err(format!(
                "{} at line {}, column {}",
                err_msg,
                self.peek().line,
                self.peek().column
            ))
        }
    }
}
