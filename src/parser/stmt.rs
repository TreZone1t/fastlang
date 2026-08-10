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
            TokenKind::Fn => self.parse_fn_decl(),
            TokenKind::TypeClass => self.parse_class_decl(),
            TokenKind::TypeStruct => self.parse_struct_decl(),
            TokenKind::TypeEnum => self.parse_enum_decl(),
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
            TokenKind::Fn => self.parse_fn_decl()?,
            TokenKind::TypeScope => self.parse_scope_decl()?,
            TokenKind::TypeClass => self.parse_class_decl()?,
            TokenKind::TypeStruct => self.parse_struct_decl()?,
            TokenKind::TypeEnum => self.parse_enum_decl()?,
            TokenKind::Let => self.parse_var_decl()?,
            kind => return Err(format!("Syntax Error: Cannot export '{:?}', only let, fn, scope, class, struct, and enum can be exported", kind)),
        };

        // Set is_exported flag to true
        match &mut stmt {
            Stmt::FnDecl { is_exported, .. } => *is_exported = true,
            Stmt::ScopeDecl { is_exported, .. } => *is_exported = true,
            Stmt::ClassDecl { is_exported, .. } => *is_exported = true,
            Stmt::StructDecl { is_exported, .. } => *is_exported = true,
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
        let (base_type, size) = match self.parse_type() {
            Ok((t, s)) => (Some(t), s),
            Err(e) => {
                if e == "Expected a type" {
                    (None, None)
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
            type_sized: Some(crate::parser::ast::TypeRef {
                base_type: base_type.unwrap_or_else(|| "unknown".to_string()),
                size,
                generics: Vec::new(),
            }),
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
                type_sized: Some(crate::parser::ast::TypeRef {
                    base_type: base_type.unwrap_or_else(|| "unknown".to_string()),
                    size,
                    generics: Vec::new(),
                }),
                name,
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
            catch_param,
            catch_block,
        })
    }

    /// يقرأ قائمة statements حتى '}' ويستهلك الـ '}'
    /// يُستعمل في كل block: if/else/loop/while body
    pub(crate) fn parse_block(&mut self) -> Result<Vec<Stmt>, String> {
        let mut stmts = Vec::new();
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

    pub(crate) fn parse_class_decl(&mut self) -> Result<Stmt, String> {
        self.advance(); // 'class'
        let name = self.get_identifier("Expected class name")?;

        let mut extends = None;
        if self.peek().kind == TokenKind::Extends {
            self.advance();
            extends = Some(self.get_identifier("Expected parent class name after 'extends'")?);
        }

        // -> is optional for class (allow both `class Foo { }` and `class Foo -> { }`)
        if self.peek().kind == TokenKind::Arrow {
            self.advance();
        }
        self.consume(TokenKind::LBrace, "Expected '{' to open class body")?;

        let (public_block, private_block, static_block, constructor) =
            self.parse_struct_class_body()?;

        Ok(Stmt::ClassDecl {
            is_exported: false,
            name,
            extends,
            public_block,
            private_block,
            static_block,
            constructor,
        })
    }

    pub(crate) fn parse_struct_decl(&mut self) -> Result<Stmt, String> {
        self.advance(); // 'struct'
        let name = self.get_identifier("Expected struct name")?;

        // -> is optional for struct (allow both `struct Foo { }` and `struct Foo -> { }`)
        if self.peek().kind == TokenKind::Arrow {
            self.advance();
        }
        self.consume(TokenKind::LBrace, "Expected '{' to open struct body")?;

        let (public_block, private_block, static_block, constructor) =
            self.parse_struct_class_body()?;

        Ok(Stmt::StructDecl {
            is_exported: false,
            name,
            public_block,
            private_block,
            static_block,
            constructor,
        })
    }

    pub(crate) fn parse_constructor_decl(
        &mut self,
    ) -> Result<crate::parser::ast::ConstructorDecl, String> {
        self.advance(); // '_'
        self.consume(TokenKind::LParen, "Expected '(' after constructor '_'")?;
        let mut params: Vec<crate::parser::ast::Param> = Vec::new();
        if self.peek().kind != TokenKind::RParen {
            loop {
                let (base_type, size) = self.parse_type()?;
                let name = self.get_identifier("Expected parameter name")?;

                params.push(crate::parser::ast::Param {
                    name,
                    base_type,
                    size,
                    generics: Vec::new(),
                });

                if self.peek().kind == TokenKind::Comma {
                    self.advance();
                } else {
                    break;
                }
            }
        }
        self.consume(TokenKind::RParen, "Expected ')' after constructor params")?;
        // Constructor is fn-like: -> is required before the body
        self.consume(
            TokenKind::Arrow,
            "Expected '->' after constructor signature '_(...)'",
        )?;
        self.consume(TokenKind::LBrace, "Expected '{' for constructor body")?;
        let body = self.parse_block()?;
        Ok(crate::parser::ast::ConstructorDecl {
            params,
            expected_types: vec![],
            body,
        })
    }

    pub(crate) fn parse_struct_class_body(
        &mut self,
    ) -> Result<
        (
            Vec<Stmt>,
            Vec<Stmt>,
            Vec<Stmt>,
            Option<crate::parser::ast::ConstructorDecl>,
        ),
        String,
    > {
        let mut public_block = Vec::new();
        let mut private_block = Vec::new();
        let mut static_block = Vec::new();
        let mut constructor = None;

        let mut has_public_block = false;
        let mut has_private_block = false;
        let mut has_static_block = false;

        while !self.is_at_end() && self.peek().kind != TokenKind::RBrace {
            let kind = self.peek().kind.clone();

            // --- Constructor: _() -> { ... } ---
            if kind == TokenKind::Underscore {
                constructor = Some(self.parse_constructor_decl()?);
                if self.peek().kind == TokenKind::SemiColon {
                    self.advance();
                }
                continue;
            }

            // --- Visibility Block or Inline property ---
            // public int x; OR public -> { int x; int y; }
            match kind {
                TokenKind::TypePublic | TokenKind::TypePrivate | TokenKind::TypeStatic => {
                    self.advance();

                    let is_public = kind == TokenKind::TypePublic;
                    let is_private = kind == TokenKind::TypePrivate;
                    let is_static = kind == TokenKind::TypeStatic;

                    if self.peek().kind == TokenKind::Arrow {
                        // Block form: public -> { ... }
                        self.advance();
                        if (is_public && has_public_block)
                            || (is_private && has_private_block)
                            || (is_static && has_static_block)
                        {
                            return Err(
                                "Syntax Error: Duplicate visibility block not allowed".to_string()
                            );
                        }
                        if is_public {
                            has_public_block = true;
                        }
                        if is_private {
                            has_private_block = true;
                        }
                        if is_static {
                            has_static_block = true;
                        }

                        self.consume(TokenKind::LBrace, "Expected '{' after visibility arrow")?;

                        while !self.is_at_end() && self.peek().kind != TokenKind::RBrace {
                            if self.peek().kind == TokenKind::Underscore {
                                constructor = Some(self.parse_constructor_decl()?);
                                if self.peek().kind == TokenKind::SemiColon {
                                    self.advance();
                                }
                                continue;
                            }

                            // parse statements (which can be var decls with 'let')
                            match self.parse_statement() {
                                Ok(Some(stmt)) => {
                                    if is_public {
                                        public_block.push(stmt);
                                    } else if is_private {
                                        private_block.push(stmt);
                                    } else {
                                        static_block.push(stmt);
                                    }
                                }
                                Ok(None) => {}
                                Err(err) => return Err(err),
                            }
                        }
                        self.consume(TokenKind::RBrace, "Expected '}' to close visibility block")?;
                        if self.peek().kind == TokenKind::SemiColon {
                            self.advance();
                        }
                    } else {
                        // Inline form: public int x;
                        match self.parse_var_decl_bare() {
                            Ok(stmt) => {
                                if is_public {
                                    public_block.push(stmt);
                                } else if is_private {
                                    private_block.push(stmt);
                                } else {
                                    static_block.push(stmt);
                                }
                            }
                            Err(e) => return Err(e),
                        }
                    }
                }
                _ => {
                    let line = self.peek().line;
                    let col = self.peek().column;
                    return Err(format!("Syntax Error: Unexpected token {:?} in struct/class body at line {}, column {}", kind, line, col));
                }
            }
        }
        self.consume(TokenKind::RBrace, "Expected '}' to close struct/class body")?;
        Ok((public_block, private_block, static_block, constructor))
    }

    pub(crate) fn parse_enum_decl(&mut self) -> Result<Stmt, String> {
        self.advance(); // 'enum'
        let name = self.get_identifier("Expected enum name")?;
        self.consume(TokenKind::Arrow, "Expected '->' after enum declaration")?;
        self.consume(TokenKind::LBrace, "Expected '{' to open enum body")?;

        let mut variants = Vec::new();
        while !self.is_at_end() && self.peek().kind != TokenKind::RBrace {
            let v_name = self.get_identifier("Expected enum variant name")?;
            let mut data_types = Vec::new();

            if self.peek().kind == TokenKind::LParen {
                self.advance();
                while !self.is_at_end() && self.peek().kind != TokenKind::RParen {
                    if let Ok((t, s)) = self.parse_type() {
                        let full_type = if let Some(sz) = s {
                            format!("{}({})", t, sz)
                        } else {
                            t
                        };
                        data_types.push(full_type);
                    } else {
                        return Err(
                            "Syntax Error: Expected type inside enum variant data".to_string()
                        );
                    }
                    if self.peek().kind == TokenKind::Comma {
                        self.advance();
                    }
                }
                self.consume(TokenKind::RParen, "Expected ')' after enum variant data")?;
            }

            variants.push(crate::parser::ast::EnumVariant {
                name: v_name,
                data_types,
            });

            if self.peek().kind == TokenKind::Comma {
                self.advance();
            } else if self.peek().kind != TokenKind::RBrace {
                return Err("Syntax Error: Expected ',' or '}' in enum body".to_string());
            }
        }
        self.consume(TokenKind::RBrace, "Expected '}' to close enum body")?;
        Ok(Stmt::EnumDecl {
            is_exported: false,
            name,
            variants,
        })
    }

    // ====================================================
    // Scope Declaration Parser
    // ====================================================
    //
    // Syntax:
    //   scope <name> -> {
    //       type      -> SomeType;           ← metadata
    //       param     -> { int a; int b; }   ← metadata (Fn/custom only)
    //       flag      <name>;                ← metadata
    //       event.<name> -> { ... }          ← metadata
    //       handle.<name> -> { ... }         ← metadata
    //       return    -> <expr>;             ← metadata (Fn/block/custom)
    //       statement -> { ... }             ← impl block (executable code)
    //   }
    //
    // الـ statement block هو الوحيد اللي بيحتوي على الكود التنفيذي.
    // أي statements خارج statement -> {} هي syntax error.
    // custom type: بيكسر معظم القواعد — الـ semantic analyzer هيتخطى enforcement.

    pub(crate) fn parse_scope_decl(&mut self) -> Result<Stmt, String> {
        self.advance(); // نتخطى كلمة 'scope'

        // 1. اسم الـ scope
        let name = if let TokenKind::Identifier(n) = &self.peek().kind.clone() {
            let n = n.clone();
            self.advance();
            n
        } else if let Some(kw) = Self::keyword_as_identifier(&self.peek().kind.clone()) {
            self.advance();
            kw
        } else {
            return Err(format!(
                "Syntax Error: Expected scope name after 'scope' at line {}, column {}",
                self.peek().line,
                self.peek().column
            ));
        };

        // 2. نتأكد من وجود '->'
        self.consume(TokenKind::Arrow, "Expected '->' after scope name")?;

        // 3. نتأكد من '{'
        self.consume(TokenKind::LBrace, "Expected '{' to open scope body")?;

        // --- متغيرات الـ scope body ---
        let mut scope_type: String = "block".to_string(); // الافتراضي
        let mut params: Vec<Stmt> = Vec::new();
        let return_type = None;
        let mut flags: Vec<crate::parser::ast::Flag> = Vec::new();
        let mut settings: Vec<crate::parser::ast::Setting> = Vec::new();
        let mut constructor: Option<crate::parser::ast::ConstructorDecl> = None;
        let mut events: Vec<crate::parser::ast::EventDecl> = Vec::new();
        let mut handles: Vec<crate::parser::ast::HandleDecl> = Vec::new();
        let mut return_value: Option<Expr> = None;
        let mut statements: Vec<Stmt> = Vec::new();
        let mut public_block_ast: Vec<Stmt> = Vec::new();
        let mut private_block_ast: Vec<Stmt> = Vec::new();
        let mut fields: Vec<crate::parser::ast::FieldDecl> = Vec::new();

        // 4. نقرأ محتوى الـ body حتى '}'
        while !self.is_at_end() && self.peek().kind != TokenKind::RBrace {
            // --- type -> SomeType; ---
            if self.peek().kind == TokenKind::TypeType {
                self.advance(); // 'type'
                self.consume(TokenKind::Arrow, "Expected '->' after 'type'")?;
                scope_type = self.parse_scope_type_expr()?;
                if self.peek().kind == TokenKind::SemiColon {
                    self.advance();
                }
                continue;
            }

            // --- param -> { int a; int b; } ---
            if self.peek().kind == TokenKind::TypeParam {
                self.advance(); // 'param'
                self.consume(TokenKind::Arrow, "Expected '->' after 'param'")?;
                self.consume(TokenKind::LBrace, "Expected '{' to open param block")?;

                while !self.is_at_end() && self.peek().kind != TokenKind::RBrace {
                    match self.parse_var_decl_bare() {
                        Ok(stmt) => params.push(stmt),
                        Err(e) => {
                            eprintln!("Syntax Error in scope param block: {}", e);
                            self.synchronize();
                        }
                    }
                }
                self.consume(TokenKind::RBrace, "Expected '}' to close param block")?;
                if self.peek().kind == TokenKind::SemiColon {
                    self.advance();
                }
                continue;
            }

            // --- flag -> { enable all; disable is_break; } ---
            if self.peek().kind == TokenKind::TypeFlag {
                self.advance(); // 'flag'

                // Allow older syntax: flag <name>;
                if let TokenKind::Identifier(flag_name) = &self.peek().kind {
                    let f = crate::parser::ast::Flag::from_str(flag_name);
                    if !flags.contains(&f) {
                        flags.push(f);
                    }
                    self.advance();
                    if self.peek().kind == TokenKind::SemiColon {
                        self.advance();
                    }
                    continue;
                }

                self.consume(TokenKind::Arrow, "Expected '->' after 'flag'")?;

                if self.peek().kind == TokenKind::LBracket {
                    // flag[is_return] syntax
                    self.advance(); // '['
                    let flag_name = if let TokenKind::Identifier(n) = &self.peek().kind.clone() {
                        let n = n.clone();
                        self.advance();
                        n
                    } else {
                        return Err(format!("Expected flag name at line {}", self.peek().line));
                    };
                    let f = crate::parser::ast::Flag::from_str(&flag_name);
                    if !flags.contains(&f) {
                        flags.push(f);
                    }
                    self.consume(TokenKind::RBracket, "Expected ']' after flag name")?;
                    if self.peek().kind == TokenKind::SemiColon {
                        self.advance();
                    }
                    continue;
                }

                self.consume(TokenKind::LBrace, "Expected '{' to open flag block")?;

                while !self.is_at_end() && self.peek().kind != TokenKind::RBrace {
                    let kind = self.peek().kind.clone();
                    match kind {
                        TokenKind::Enable | TokenKind::Disable => {
                            self.advance(); // enable/disable
                            let is_enable = kind == TokenKind::Enable;
                            let flag_name = if self.peek().kind == TokenKind::All {
                                self.advance();
                                "all".to_string()
                            } else if let TokenKind::Identifier(n) = &self.peek().kind.clone() {
                                let n = n.clone();
                                self.advance();
                                n
                            } else {
                                return Err(format!(
                                    "Expected flag name or 'all' at line {}",
                                    self.peek().line
                                ));
                            };

                            if is_enable {
                                let f = crate::parser::ast::Flag::from_str(&flag_name);
                                if !flags.contains(&f) {
                                    flags.push(f);
                                }
                            } else {
                                let f = crate::parser::ast::Flag::from_str(&flag_name);
                                flags.retain(|x| x != &f);
                            }

                            if self.peek().kind == TokenKind::SemiColon {
                                self.advance();
                            }
                        }
                        _ => {
                            return Err(format!(
                                "Unexpected token {:?} in flag block at line {}",
                                kind,
                                self.peek().line
                            ));
                        }
                    }
                }
                self.consume(TokenKind::RBrace, "Expected '}' to close flag block")?;
                continue;
            }

            // --- enable [length, size, ...] OR enable flag[is_break, ...]; ---
            if self.peek().kind == TokenKind::Enable {
                self.advance(); // 'enable'

                // Optional '->'
                if self.peek().kind == TokenKind::Arrow {
                    self.advance();
                }

                let is_flag = self.peek().kind == TokenKind::TypeFlag;
                if is_flag {
                    self.advance(); // consume 'flag'
                }

                let is_bracket = self.peek().kind == TokenKind::LBracket;
                let is_brace = self.peek().kind == TokenKind::LBrace;

                if is_bracket || is_brace {
                    self.advance();
                    while !self.is_at_end()
                        && self.peek().kind != TokenKind::RBracket
                        && self.peek().kind != TokenKind::RBrace
                    {
                        let name = if let TokenKind::Identifier(n) = &self.peek().kind.clone() {
                            let n = n.clone();
                            self.advance();
                            n
                        } else if let Some(kw) =
                            Self::keyword_as_identifier(&self.peek().kind.clone())
                        {
                            self.advance();
                            kw
                        } else {
                            return Err(format!("Expected name at line {}", self.peek().line));
                        };

                        if is_flag {
                            let f = crate::parser::ast::Flag::from_str(&name);
                            if !flags.contains(&f) {
                                flags.push(f);
                            }
                        } else {
                            let s = crate::parser::ast::Setting::from_str(&name);
                            if !settings.contains(&s) {
                                settings.push(s);
                            }
                        }

                        if self.peek().kind == TokenKind::Comma {
                            self.advance();
                        } else {
                            break;
                        }
                    }
                    if is_bracket {
                        self.consume(TokenKind::RBracket, "Expected ']' to close enable array")?;
                    } else {
                        self.consume(TokenKind::RBrace, "Expected '}' to close enable array")?;
                    }
                } else {
                    let name = if let TokenKind::Identifier(n) = &self.peek().kind.clone() {
                        let n = n.clone();
                        self.advance();
                        n
                    } else if let Some(kw) = Self::keyword_as_identifier(&self.peek().kind.clone())
                    {
                        self.advance();
                        kw
                    } else {
                        return Err(format!("Expected name at line {}", self.peek().line));
                    };
                    if is_flag {
                        let f = crate::parser::ast::Flag::from_str(&name);
                        if !flags.contains(&f) {
                            flags.push(f);
                        }
                    } else {
                        let s = crate::parser::ast::Setting::from_str(&name);
                        if !settings.contains(&s) {
                            settings.push(s);
                        }
                    }
                }

                if self.peek().kind == TokenKind::SemiColon {
                    self.advance();
                }
                continue;
            }

            // --- return -> expr; (Metadata) ---
            if self.peek().kind == TokenKind::Return {
                // Peek ahead to see if it's metadata (return ->) or a statement (return expr;)
                let mut is_metadata = false;
                let next_idx = self.current + 1;
                if next_idx < self.tokens.len() {
                    if self.tokens[next_idx].kind == TokenKind::Arrow {
                        is_metadata = true;
                    }
                }

                if is_metadata {
                    self.advance(); // 'return'
                    self.advance(); // '->'
                    let expr = if self.peek().kind == TokenKind::LParen {
                        self.advance(); // '('
                        let e = self.parse_expression()?;
                        self.consume(TokenKind::RParen, "Expected ')' after return expression")?;
                        e
                    } else {
                        self.parse_expression()?
                    };
                    if self.peek().kind == TokenKind::SemiColon {
                        self.advance();
                    }
                    return_value = Some(expr);
                    continue;
                }
                // If not metadata, fall through to parse as a statement
            }

            // --- event -> { <name> -> { ... } }  OR  handle -> { <name> -> { ... } } ---
            if self.peek().kind == TokenKind::TypeEvent || self.peek().kind == TokenKind::TypeHandle
            {
                let is_event = self.peek().kind == TokenKind::TypeEvent;
                self.advance(); // 'event' or 'handle'

                self.consume(TokenKind::Arrow, "Expected '->' after 'event'/'handle'")?;
                self.consume(TokenKind::LBrace, "Expected '{' to open event/handle block")?;

                while !self.is_at_end() && self.peek().kind != TokenKind::RBrace {
                    // اسم الـ trigger أو الـ flag
                    let trigger_name = if let TokenKind::Identifier(n) = &self.peek().kind.clone() {
                        let n = n.clone();
                        self.advance();
                        n
                    } else if let Some(kw) = Self::keyword_as_identifier(&self.peek().kind.clone())
                    {
                        self.advance();
                        kw
                    } else {
                        return Err(format!(
                            "Syntax Error: Expected name at line {}, column {}",
                            self.peek().line,
                            self.peek().column
                        ));
                    };

                    self.consume(TokenKind::Arrow, "Expected '->' after event/handle name")?;
                    self.consume(TokenKind::LBrace, "Expected '{' to open body")?;

                    let mut body: Vec<Stmt> = Vec::new();
                    while !self.is_at_end() && self.peek().kind != TokenKind::RBrace {
                        match self.parse_statement() {
                            Ok(Some(stmt)) => body.push(stmt),
                            Ok(None) => {
                                if !self.is_at_end() && self.peek().kind != TokenKind::RBrace {
                                    self.advance();
                                }
                            }
                            Err(err) => return Err(err),
                        }
                    }
                    self.consume(TokenKind::RBrace, "Expected '}' to close body")?;

                    if self.peek().kind == TokenKind::SemiColon {
                        self.advance();
                    }

                    if is_event {
                        events.push(crate::parser::ast::EventDecl { trigger_name, body });
                    } else {
                        handles.push(crate::parser::ast::HandleDecl {
                            target_flag: trigger_name,
                            body,
                        });
                    }
                }

                self.consume(
                    TokenKind::RBrace,
                    "Expected '}' to close event/handle block",
                )?;

                if self.peek().kind == TokenKind::SemiColon {
                    self.advance();
                }
                continue;
            }

            // ============================================================
            // _(params) -> { ... } (Constructor)
            // ============================================================
            if self.peek().kind == TokenKind::Underscore {
                match self.parse_constructor_decl() {
                    Ok(c) => constructor = Some(c),
                    Err(e) => {
                        eprintln!("Syntax Error in scope constructor: {}", e);
                        self.synchronize();
                    }
                }
                continue;
            }

            // ============================================================
            // add <type> <name>; (adds a scope field for custom scopes)
            // ============================================================
            if let TokenKind::Identifier(ref n) = self.peek().kind {
                if n == "add" {
                    self.advance(); // consume 'add'

                    match self.parse_var_decl_bare() {
                        Ok(stmt) => {
                            if let Stmt::VarDecl {
                                visibility,
                                editability,
                                type_sized,
                                name,
                                value,
                            } = stmt
                            {
                                fields.push(crate::parser::ast::FieldDecl {
                                    visibility,
                                    editability,
                                    type_sized,
                                    name,
                                    value: if let Expr::Identifier(ref s) = value {
                                        if s == "__param__" || s == "" {
                                            None
                                        } else {
                                            Some(value)
                                        }
                                    } else {
                                        Some(value)
                                    },
                                });
                            }
                        }
                        Err(e) => {
                            eprintln!("Syntax Error in 'add' component: {}", e);
                            self.synchronize();
                        }
                    }
                    continue;
                }
            }

            // ============================================================
            // public -> { ... }
            // ============================================================
            if self.peek().kind == TokenKind::TypePublic {
                self.advance(); // 'public'
                self.consume(TokenKind::Arrow, "Expected '->' after 'public'")?;
                self.consume(TokenKind::LBrace, "Expected '{' to open public block")?;

                while !self.is_at_end() && self.peek().kind != TokenKind::RBrace {
                    match self.parse_statement() {
                        Ok(Some(stmt)) => public_block_ast.push(stmt),
                        Ok(None) => {
                            if !self.is_at_end() && self.peek().kind != TokenKind::RBrace {
                                self.advance();
                            }
                        }
                        Err(err) => return Err(err),
                    }
                }

                self.consume(TokenKind::RBrace, "Expected '}' to close public block")?;
                if self.peek().kind == TokenKind::SemiColon {
                    self.advance();
                }
                continue;
            }

            // ============================================================
            // private -> { ... }
            // ============================================================
            if self.peek().kind == TokenKind::TypePrivate {
                self.advance(); // 'private'
                self.consume(TokenKind::Arrow, "Expected '->' after 'private'")?;
                self.consume(TokenKind::LBrace, "Expected '{' to open private block")?;

                while !self.is_at_end() && self.peek().kind != TokenKind::RBrace {
                    match self.parse_statement() {
                        Ok(Some(stmt)) => private_block_ast.push(stmt),
                        Ok(None) => {
                            if !self.is_at_end() && self.peek().kind != TokenKind::RBrace {
                                self.advance();
                            }
                        }
                        Err(err) => return Err(err),
                    }
                }

                self.consume(TokenKind::RBrace, "Expected '}' to close private block")?;
                if self.peek().kind == TokenKind::SemiColon {
                    self.advance();
                }
                continue;
            }

            // --- أي شيء تاني يعتبر عبارة برمجية (Statement) مباشرة ---
            match self.parse_statement() {
                Ok(Some(stmt)) => statements.push(stmt),
                Ok(None) => {
                    if !self.is_at_end() && self.peek().kind != TokenKind::RBrace {
                        self.advance();
                    }
                }
                Err(err) => return Err(err),
            }
        }

        // 5. نتأكد من '}'
        self.consume(TokenKind::RBrace, "Expected '}' to close scope body")?;

        let parsed_scope_type = if scope_type.starts_with("custom") {
            crate::parser::ast::ScopeType::Custom
        } else {
            match scope_type.as_str() {
                "fn" | "Fn" => crate::parser::ast::ScopeType::Fn,
                "looped" => crate::parser::ast::ScopeType::Looped,
                "array" => crate::parser::ast::ScopeType::Array,
                "str" | "string" => crate::parser::ast::ScopeType::String,
                _ => crate::parser::ast::ScopeType::Block,
            }
        };

        // params in ScopeDecl are Vec<Param>, but we gathered Stmt::VarDecl. We must map them.
        let mut mapped_params = Vec::new();
        for p in params {
            if let Stmt::VarDecl {
                type_sized, name, ..
            } = p
            {
                let (base_type, size) = type_sized
                    .map(|t| (t.base_type, t.size))
                    .unwrap_or(("unknown".to_string(), None));
                mapped_params.push(crate::parser::ast::Param {
                    base_type,
                    size,
                    name,
                    generics: Vec::new(),
                });
            }
        }

        Ok(Stmt::ScopeDecl {
            is_exported: false,
            is_const: false,
            name,
            scope_type: parsed_scope_type,
            params: mapped_params,
            return_type,
            flags,
            settings,
            events,
            generic_block: vec![],
            static_block: vec![],
            handle_block: vec![],
            custom_keyword: None,
            statements,
            public_block: public_block_ast,
            fields,
            private_block: private_block_ast,
            return_value,
            constructor,
        })
    }

    /// Parses traditional function syntax: fn name(a: int(32)) -> ret_type { ... }
    /// Desugars it into a ScopeDecl of type "Fn".
    pub(crate) fn parse_fn_decl(&mut self) -> Result<Stmt, String> {
        self.advance(); // 'fn'
        let name = self.get_identifier("Expected function name")?;

        self.consume(TokenKind::LParen, "Expected '(' after function name")?;
        let mut params = Vec::new();
        if self.peek().kind != TokenKind::RParen {
            loop {
                let p_name = self.get_identifier("Expected parameter name")?;
                self.consume(TokenKind::Colon, "Expected ':' after parameter name")?;
                let (base_type, size) = self.parse_type()?;

                params.push(crate::parser::ast::Param {
                    base_type,
                    size,
                    name: p_name,
                    generics: Vec::new(),
                });

                if self.peek().kind == TokenKind::Comma {
                    self.advance();
                } else {
                    break;
                }
            }
        }
        self.consume(TokenKind::RParen, "Expected ')' after parameters")?;

        // Optional return type
        let return_type = if self.peek().kind == TokenKind::Arrow {
            self.advance();
            let (base_type, size) = self.parse_type()?;
            Some(crate::parser::ast::TypeRef {
                base_type,
                size,
                generics: vec![],
            })
        } else {
            None
        };

        self.consume(TokenKind::LBrace, "Expected '{' to open function body")?;
        let statements = self.parse_block()?;

        Ok(Stmt::ScopeDecl {
            is_exported: false,
            is_const: false,
            name,
            scope_type: crate::parser::ast::ScopeType::Fn,
            params,
            return_type,
            flags: vec![crate::parser::ast::Flag::IsReturn],
            settings: Vec::new(),
            events: Vec::new(),
            generic_block: Vec::new(),
            static_block: Vec::new(),
            handle_block: Vec::new(),
            custom_keyword: None,
            statements,
            public_block: Vec::new(),
            fields: Vec::new(),
            private_block: Vec::new(),
            return_value: None,
            constructor: None,
        })
    }

    /// يقرأ قيمة `type -> ???` من جوا scope body.
    /// بيقبل: Fn, block, looped, custom, Struct, أو أي identifier
    pub(crate) fn parse_scope_type_expr(&mut self) -> Result<String, String> {
        match &self.peek().kind.clone() {
            // identifier بسيط: Fn, block, looped, global, etc.
            TokenKind::Identifier(name) => {
                let t = name.clone();
                self.advance();
                Ok(t)
            }
            // custom keyword token with optional ("name")
            TokenKind::TypeCustom => {
                self.advance();
                let mut suffix = String::new();
                if self.peek().kind == TokenKind::LParen {
                    self.advance();
                    if let TokenKind::String(ref s) = self.peek().kind.clone() {
                        suffix = format!("(\"{}\")", s);
                        self.advance();
                    } else if let TokenKind::Identifier(ref s) = self.peek().kind.clone() {
                        suffix = format!("(\"{}\")", s); // Handle box without quotes just in case
                        self.advance();
                    }
                    if self.peek().kind == TokenKind::RParen {
                        self.advance();
                    }
                }
                Ok(format!("custom{}", suffix))
            }
            // keyword as type name
            _ => {
                if let Some(kw) = Self::keyword_as_identifier(&self.peek().kind.clone()) {
                    self.advance();
                    return Ok(kw);
                }
                match &self.peek().kind.clone() {
                    TokenKind::Fn => {
                        self.advance();
                        Ok("Fn".to_string())
                    }
                    TokenKind::Loop => {
                        self.advance();
                        Ok("looped".to_string())
                    }
                    TokenKind::While => {
                        self.advance();
                        Ok("looped".to_string())
                    }
                    TokenKind::TypeStruct => {
                        self.advance();
                        Ok("Struct".to_string())
                    }
                    TokenKind::TypeArray => {
                        self.advance();
                        Ok("array".to_string())
                    }
                    TokenKind::TypeStr => {
                        self.advance();
                        Ok("str".to_string())
                    }
                    other => Err(format!(
                        "Syntax Error: Invalid scope type '{:?}' at line {}, column {}",
                        other,
                        self.peek().line,
                        self.peek().column
                    )),
                }
            }
        }
    }

    /// نسخة من parse_var_decl بدون 'let' prefix —
    /// بتُستخدم جوا param blocks اللي بتعمل تعريف بدون الكلمة المفتاحية.
    /// مثال: `int a;` أو `int : 8 a;`
    pub(crate) fn parse_var_decl_bare(&mut self) -> Result<Stmt, String> {
        let (base_type, size) = match self.parse_type() {
            Ok((t, s)) => (Some(t), s),
            Err(e) => return Err(e),
        };

        // اسم الـ param — identifier أو keyword-as-identifier
        let name = if let TokenKind::Identifier(n) = &self.peek().kind.clone() {
            let n = n.clone();
            self.advance();
            n
        } else if let Some(kw) = Self::keyword_as_identifier(&self.peek().kind.clone()) {
            self.advance();
            kw
        } else {
            return Err(format!(
                "Syntax Error: Expected parameter name at line {}, column {}",
                self.peek().line,
                self.peek().column
            ));
        };

        // '=' اختيارية لإعطاء قيمة مبدئية
        let mut value = Expr::Identifier("__param__".to_string());
        if self.peek().kind == TokenKind::Assign {
            self.advance();
            value = self.parse_expression()?;
        }

        // ';' اختيارية (وكمان ';' بعد '}' في  param -> { ... };  مقبولة)
        if self.peek().kind == TokenKind::SemiColon {
            self.advance();
        }

        Ok(Stmt::VarDecl {
            visibility: crate::parser::ast::Visibility::Private,
            editability: crate::parser::ast::Editability::Editable,
            type_sized: Some(crate::parser::ast::TypeRef {
                base_type: base_type.unwrap_or_else(|| "unknown".to_string()),
                size,
                generics: Vec::new(),
            }),
            name,
            value,
        })
    }
}
