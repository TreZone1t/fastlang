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
            // Custom keyword: `my_list<int(32)> items -> [1, 2];`
            TokenKind::Identifier(id) => self.parse_expression_or_reassignment(),
            TokenKind::MadeUpType(id) => {
                todo!()
            }
            kind if self.is_type_token(kind) => self.parse_var_decl(),
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

    pub(crate) fn parse_use_stmt(&mut self) -> Result<Stmt, String> {
        self.advance(); // consume 'use'
        let mut module_path: Vec<String> = Vec::new();
        let mut imports: Option<Vec<String>> = None;

        module_path.push(self.get_identifier("Expected module name after 'use'")?);

        while self.peek().kind == TokenKind::DoubleColon {
            self.advance();

            if self.peek().kind == TokenKind::LBrace {
                self.advance();
                let mut selected: Vec<String> = Vec::new();

                if !self.is_at_end() && self.peek().kind != TokenKind::RBrace {
                    loop {
                        selected.push(self.get_identifier("Expected import name in use list")?);

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
            }

            module_path.push(self.get_identifier("Expected module name after '::'")?);
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
        let mut is_const = false;
        if self.peek().kind == TokenKind::Const {
            //const type name -> value;
            is_const = true;
        }
        if self.peek().kind == TokenKind::Let {
            //let type name = value;
            if is_const {
                return Err("Syntax Error: Unexpected 'let' after 'const'".to_string());
            }
            self.advance();
            let type_node = self.parse_type()?;
            let name = self.get_identifier("Expected variable name after 'let'")?;
            let mut value = Expr::Identifier("__param__".to_string());

            if self.peek().kind == TokenKind::Assign {
                self.advance();
                value = self.parse_expression()?;
                self.consume(
                    TokenKind::SemiColon,
                    "Expected ';' after variable declaration",
                )?;
            } else {
                self.consume(
                    TokenKind::SemiColon,
                    "Expected ';' after variable declaration",
                )?;
            }
            return Ok(Stmt::VarDecl {
                visibility: Visibility::Private,
                editability: if is_const {
                    Editability::NotEditable
                } else {
                    Editability::Editable
                },
                type_node: Some(type_node),
                name,
                value,
            });
        } else {
            let type_node = self.parse_type()?;

            let name = self.get_identifier("Expected variable name after type")?;

            let mut value = Expr::Identifier("__param__".to_string());

            if self.peek().kind == TokenKind::Arrow {
                self.advance();
                value = self.parse_expression()?;
                self.consume(
                    TokenKind::SemiColon,
                    "Expected ';' after variable declaration",
                )?;
            } else {
                self.consume(
                    TokenKind::SemiColon,
                    "Expected ';' after variable declaration",
                )?;
            }
            return Ok(Stmt::VarDecl {
                visibility: Visibility::Private,
                editability: if is_const {
                    Editability::NotEditable
                } else {
                    Editability::Editable
                },
                type_node: Some(type_node),
                name,
                value,
            });
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
                        self.parse_block()?
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

    /// يُعالج سطراً يبدأ بـ Identifier عادي (مش custom keyword):
    /// إما إعادة تعيين: `items -> [1];`  أو  expression: `foo.bar();`
    pub(crate) fn parse_expression_or_reassignment(&mut self) -> Result<Stmt, String> {
        // Look-ahead: `TypeName varName ->` يعني VarDecl بـ user-defined type
        // مثل `Node n -> new Node(temp);` أو `Node<T> n -> ...;`
        if let TokenKind::Identifier(type_name) = self.peek().kind.clone() {
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
                return self.parse_var_decl();
            }
        }

        let expr = self.parse_expression()?;

        // إعادة تعيين: target -> value;   أو   target = value;
        if self.peek().kind == TokenKind::Arrow || self.peek().kind == TokenKind::Assign {
            self.advance(); // consume '->' or '='
            let value = self.parse_expression()?;
            self.consume(TokenKind::SemiColon, "Expected ';' after reassignment")?;
            return Ok(Stmt::ReassignStmt {
                target: expr,
                value,
            });
        }

        self.consume(
            TokenKind::SemiColon,
            "Expected ';' after expression statement",
        )?;
        Ok(Stmt::ExpressionStmt(expr))
    }

    /// يُعالج تعريف متغير باستخدام Custom Keyword:
    /// `my_list<int(32)> items -> [1, 2];`
    /// حيث `keyword_name` = "my_list" و `original_scope_name` = "array"
    pub(crate) fn parse_custom_keyword_var_decl(
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
            crate::parser::ast::TypeNode::Simple(crate::parser::ast::TypeRef {
                base_type: keyword_name,
                size: None,
            })
        } else {
            crate::parser::ast::TypeNode::Generic(crate::parser::ast::Generic {
                base_type: keyword_name,
                generics,
            })
        };

        Ok(Stmt::VarDecl {
            visibility: crate::parser::ast::Visibility::Private,
            editability: crate::parser::ast::Editability::Editable,
            type_node: Some(type_node),
            name,
            value,
        })
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
    pub(crate) fn get_sc_type(&mut self, err_msg: &str) -> Result<String, String> {
        if let TokenKind::MadeUpType(n) = &self.peek().kind.clone() {
            let s = n.to_string();
            self.advance();
            return Ok(s);
        } else {
            return Err(err_msg.to_string());
        }
    }
}
