use crate::lexer::token::{Token, TokenKind};
use crate::parser::ast::*;

// Fallback token returned when `current` runs past the end of the stream.
// Explicit `const` instead of an inline `&Token { .. }` literal so we don't
// depend on rvalue static-promotion rules to make the borrow-check work.
const EOF_TOKEN: Token = Token {
    kind: TokenKind::EOF,
    line: 0,
    column: 0,
};

pub struct Parser {
    tokens: Vec<Token>,
    current: usize,
}

impl Parser {
    pub fn new(tokens: Vec<Token>) -> Self {
        Parser { tokens, current: 0 }
    }

    fn peek(&self) -> &Token {
        // NOTE: EOF placeholder for out-of-range access carries dummy position 0,0.
        // Should not happen in practice since is_at_end() gates the main loops.
        self.tokens.get(self.current).unwrap_or(&EOF_TOKEN)
    }

    fn previous(&self) -> &Token {
        &self.tokens[self.current - 1]
    }

    fn is_at_end(&self) -> bool {
        self.peek().kind == TokenKind::EOF
    }

    fn advance(&mut self) -> &Token {
        if !self.is_at_end() {
            self.current += 1;
        }
        self.previous()
    }

    /// Keywords that can legally appear as identifiers in name positions
    /// (variable names, field names, parameter names).
    /// e.g. `let bool flag = ...` where 'flag' is a keyword we registered.
    fn keyword_as_identifier(kind: &TokenKind) -> Option<String> {
        match kind {
            // Context-type keywords that users can also use as names
            TokenKind::TypeFlag => Some("flag".to_string()),
            TokenKind::TypeLength => Some("length".to_string()),
            TokenKind::TypeSize => Some("size".to_string()),
            TokenKind::TypeParam => Some("param".to_string()),
            TokenKind::TypeType => Some("type".to_string()),
            TokenKind::TypeInit => Some("init".to_string()),
            TokenKind::TypeEvent => Some("event".to_string()),
            TokenKind::TypeHandle => Some("handle".to_string()),
            TokenKind::TypeName => Some("name".to_string()),
            TokenKind::TypeCustom => Some("custom".to_string()),
            TokenKind::TypeError => Some("error".to_string()),
            TokenKind::TypeBlock => Some("block".to_string()),
            TokenKind::Fn => Some("fn".to_string()),
            TokenKind::TypeStruct => Some("struct".to_string()),
            TokenKind::Class => Some("class".to_string()),
            TokenKind::Enum => Some("enum".to_string()),
            TokenKind::Log => Some("log".to_string()),
            // booleans as identifiers
            TokenKind::Bool(b) => Some(if *b { "true" } else { "false" }.to_string()),
            _ => None,
        }
    }

    pub fn parse_program(&mut self) -> Program {
        let mut statements = Vec::new();

        while !self.is_at_end() {
            if let Some(stmt) = self.parse_statement() {
                statements.push(stmt);
            }
        }

        Program { statements }
    }

    fn consume(&mut self, expected: TokenKind, error_message: &str) -> Result<&Token, String> {
        if core::mem::discriminant(&self.peek().kind) == core::mem::discriminant(&expected) {
            Ok(self.advance())
        } else {
            // هنا هنضيف مستقبلاً اللوجيك اللي بيشاور على السطر وبيطبع الـ Hints (زي IF و print)
            // NOTE: self.peek().line / .column are now available for exactly this.
            Err(format!(
                "{} (at line {}, column {})",
                error_message,
                self.peek().line,
                self.peek().column
            ))
        }
    }

    // الدالة دي بترجع البارسر لوعيه بعد ما يلاقي غلطة عشان الكومبايلر ميكراشش
    fn synchronize(&mut self) {
        self.advance();

        while !self.is_at_end() {
            if self.previous().kind == TokenKind::SemiColon {
                return;
            }

            match &self.peek().kind {
                TokenKind::Let
                | TokenKind::Set
                | TokenKind::If
                | TokenKind::Else
                | TokenKind::While
                | TokenKind::Loop
                | TokenKind::Break
                | TokenKind::Continue
                | TokenKind::Return
                | TokenKind::Fn
                | TokenKind::TypeScope
                | TokenKind::Class
                | TokenKind::Enum
                | TokenKind::TypeStruct => {
                    return;
                }
                _ => {
                    self.advance();
                }
            }
        }
    }

    // ----------------------------------------------------
    // تحليل الجمل (Statement Parsing)
    // ----------------------------------------------------

    // الدالة دي بتحدد إحنا هنقرأ أي نوع من الأوامر
    fn parse_statement(&mut self) -> Option<Stmt> {
        let result = match &self.peek().kind {
            TokenKind::Use => self.parse_use_stmt(),
            TokenKind::Export => self.parse_exported_stmt(),
            TokenKind::Let => self.parse_var_decl(),
            TokenKind::Set => self.parse_reassign_stmt(),
            TokenKind::If => self.parse_if_stmt(),
            TokenKind::For => self.parse_for_stmt(),
            TokenKind::Loop => self.parse_loop_stmt(),
            TokenKind::While => self.parse_while_stmt(),
            TokenKind::TypeScope => self.parse_scope_decl(),
            TokenKind::Fn => self.parse_fn_decl(),
            TokenKind::Class => self.parse_class_decl(),
            TokenKind::TypeStruct => self.parse_struct_decl(),
            TokenKind::Enum => self.parse_enum_decl(),
            TokenKind::Break => {
                self.advance();
                if self.peek().kind == TokenKind::SemiColon {
                    self.advance();
                }
                Ok(Stmt::BreakStmt)
            }
            TokenKind::Continue => {
                self.advance();
                if self.peek().kind == TokenKind::SemiColon {
                    self.advance();
                }
                Ok(Stmt::ContinueStmt)
            }
            TokenKind::Return => {
                self.advance();
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
            TokenKind::Throw => self.parse_throw_stmt(),
            TokenKind::Try => self.parse_try_catch_stmt(),
            kind if Self::is_type_token(kind) => self.parse_var_decl_bare(),
            _ => self.parse_expression_stmt(),
        };

        match result {
            Ok(stmt) => Some(stmt),
            Err(err) => {
                println!("Syntax Error: {}", err); // هنطور شكل الطباعة دي قدام
                self.synchronize(); // تخطى العك ده وشوف السطر اللي بعده
                None
            }
        }
    }

    fn parse_use_stmt(&mut self) -> Result<Stmt, String> {
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

    fn parse_exported_stmt(&mut self) -> Result<Stmt, String> {
        self.advance(); // consume 'export'

        // After export, we expect a valid exportable statement (fn, scope, let, class, struct, enum)
        let mut stmt = match &self.peek().kind {
            TokenKind::Fn => self.parse_fn_decl()?,
            TokenKind::TypeScope => self.parse_scope_decl()?,
            TokenKind::Class => self.parse_class_decl()?,
            TokenKind::TypeStruct => self.parse_struct_decl()?,
            TokenKind::Enum => self.parse_enum_decl()?,
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
            Stmt::VarDecl { is_exported, .. } => *is_exported = true,
            _ => {}
        }

        Ok(stmt)
    }

    // تحليل تعريف المتغير: let int : 8 a = 5; أو let a : i8 = 5;
    fn parse_var_decl(&mut self) -> Result<Stmt, String> {
        self.advance(); // نتخطى كلمة 'let' اللي دخلتنا الدالة دي أصلاً

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

        // 4. التأكد من وجود علامة '='
        if self.peek().kind == TokenKind::Assign {
            self.advance(); // نتخطى الـ '='
        } else {
            // هنا ممكن نحط الـ Hint بتاعنا لو كتب -> بدل = بالغلط
            if self.peek().kind == TokenKind::Arrow {
                return Err(format!("Syntax Error: Expected '=' to assign value to '{}'. Use '->' for reassignment (set), not declaration (let).", name));
            }
            return Err(format!(
                "Syntax Error: Expected '=' after variable name '{}'",
                name
            ));
        }

        // 5. قراءة القيمة (Expression)
        let value = self.parse_expression()?;

        // 6. التأكد من وجود الفصلة المنقوطة ';'
        if self.peek().kind == TokenKind::SemiColon {
            self.advance(); // نتخطى الـ ';'
        } else {
            return Err("Syntax Error: Missing ';' at the end of declaration".to_string());
        }

        // لو كل حاجة تمام، نرجع الـ Node بتاعة الـ AST
        Ok(Stmt::VarDecl {
            is_exported: false,
            is_static: false, // مؤقتاً لحد ما نضيف دعم الكلمات دي
            is_const: false,
            base_type,
            size,
            name,
            value,
        })
    }

    // ====================================================
    // Control Flow Parsers
    // ====================================================

    // --- set <target> -> <value>; -------------------------
    // target: identifier  أو  property chain (obj.field.sub)
    fn parse_reassign_stmt(&mut self) -> Result<Stmt, String> {
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
    fn parse_if_stmt(&mut self) -> Result<Stmt, String> {
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
    fn parse_loop_stmt(&mut self) -> Result<Stmt, String> {
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
            crate::parser::ast::LoopBody::Inline(stmts)
        } else {
            // scope_name(args) أو scope_name بدون أرغومنتس
            let expr = self.parse_expression()?;
            if self.peek().kind == TokenKind::SemiColon {
                self.advance();
            }
            crate::parser::ast::LoopBody::ScopeCall(expr)
        };

        Ok(Stmt::LoopStmt { count, body })
    }

    // --- while (cond) -> { ... }  أو  while (cond) -> scope_name() ---
    fn parse_while_stmt(&mut self) -> Result<Stmt, String> {
        self.advance(); // 'while'

        self.consume(TokenKind::LParen, "Expected '(' after 'while'")?;
        let condition = self.parse_expression()?;
        self.consume(TokenKind::RParen, "Expected ')' after while condition")?;

        self.consume(TokenKind::Arrow, "Expected '->' after while condition (use: while (cond) -> { } or while (cond) -> scope())")?;

        // '->' متبوعة بـ '{' = inline block, غير كدة = scope call
        let body = if self.peek().kind == TokenKind::LBrace {
            self.advance(); // '{'
            let stmts = self.parse_block()?;
            crate::parser::ast::LoopBody::Inline(stmts)
        } else {
            let expr = self.parse_expression()?;
            if self.peek().kind == TokenKind::SemiColon {
                self.advance();
            }
            crate::parser::ast::LoopBody::ScopeCall(expr)
        };

        Ok(Stmt::WhileStmt { condition, body })
    }

    fn parse_for_stmt(&mut self) -> Result<Stmt, String> {
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
            crate::parser::ast::LoopBody::Inline(stmts)
        } else {
            let expr = self.parse_expression()?;
            if self.peek().kind == TokenKind::SemiColon {
                self.advance();
            }
            crate::parser::ast::LoopBody::ScopeCall(expr)
        };

        Ok(Stmt::ForStmt {
            init,
            condition,
            increment,
            body,
        })
    }

    fn parse_throw_stmt(&mut self) -> Result<Stmt, String> {
        self.advance(); // 'throw'

        // Optional 'new' (like: throw new error("..."))
        if self.peek().kind == TokenKind::New {
            self.advance();
        }

        let expr = self.parse_expression()?;
        self.consume(TokenKind::SemiColon, "Expected ';' after throw statement")?;
        Ok(Stmt::ThrowStmt(expr))
    }

    fn parse_expression_stmt(&mut self) -> Result<Stmt, String> {
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
                is_exported: false,
                is_static: false,
                is_const: false,
                base_type,
                size,
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

    fn parse_try_catch_stmt(&mut self) -> Result<Stmt, String> {
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
    fn parse_block(&mut self) -> Result<Vec<Stmt>, String> {
        let mut stmts = Vec::new();
        while !self.is_at_end() && self.peek().kind != TokenKind::RBrace {
            if let Some(stmt) = self.parse_statement() {
                stmts.push(stmt);
            } else if !self.is_at_end() && self.peek().kind != TokenKind::RBrace {
                self.advance(); // تفادي infinite loop عند الـ error
            }
        }
        self.consume(TokenKind::RBrace, "Expected '}' to close block")?;
        Ok(stmts)
    }

    // ====================================================
    // OOP & Enum Parsers
    // ====================================================

    fn get_identifier(&mut self, err_msg: &str) -> Result<String, String> {
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

    fn parse_class_decl(&mut self) -> Result<Stmt, String> {
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

    fn parse_struct_decl(&mut self) -> Result<Stmt, String> {
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

    fn parse_constructor_decl(&mut self) -> Result<crate::parser::ast::ConstructorDecl, String> {
        self.advance(); // '_'
        self.consume(TokenKind::LParen, "Expected '(' after constructor '_'")?;
        let mut params: Vec<crate::parser::ast::Param> = Vec::new();
        if self.peek().kind != TokenKind::RParen {
            loop {
                let name = self.get_identifier("Expected parameter name")?;
                self.consume(TokenKind::Colon, "Expected ':' after parameter name")?;

                let (base_type, size) = self.parse_type()?;
                params.push(crate::parser::ast::Param {
                    name,
                    base_type,
                    size,
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

    fn parse_struct_class_body(
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
                            if let Some(stmt) = self.parse_statement() {
                                if is_public {
                                    public_block.push(stmt);
                                } else if is_private {
                                    private_block.push(stmt);
                                } else {
                                    static_block.push(stmt);
                                }
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

    fn parse_enum_decl(&mut self) -> Result<Stmt, String> {
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

    fn parse_scope_decl(&mut self) -> Result<Stmt, String> {
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
        let mut flags: Vec<String> = Vec::new();
        let mut settings: Vec<String> = Vec::new();
        let mut constructor: Option<crate::parser::ast::ConstructorDecl> = None;
        let mut events: Vec<crate::parser::ast::EventDecl> = Vec::new();
        let mut handles: Vec<crate::parser::ast::HandleDecl> = Vec::new();
        let mut return_value: Option<Expr> = None;
        let mut statements: Vec<Stmt> = Vec::new();
        let mut public_block_ast: Vec<Stmt> = Vec::new();
        let mut private_block_ast: Vec<Stmt> = Vec::new();
        let mut fields: Vec<Stmt> = Vec::new();

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
                if let TokenKind::Identifier(n) = &self.peek().kind.clone() {
                    let n = n.clone();
                    self.advance();
                    if self.peek().kind == TokenKind::SemiColon {
                        self.advance();
                    }
                    flags.push(format!("+{}", n));
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
                    flags.push(format!("+{}", flag_name));
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
                                flags.push(format!("+{}", flag_name));
                            } else {
                                flags.push(format!("-{}", flag_name));
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
                            flags.push(format!("+{}", name));
                        } else {
                            settings.push(format!("+{}", name));
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
                        flags.push(format!("+{}", name));
                    } else {
                        settings.push(format!("+{}", name));
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
                        if let Some(stmt) = self.parse_statement() {
                            body.push(stmt);
                        } else {
                            self.advance();
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
                        Ok(stmt) => fields.push(stmt),
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
                    if let Some(stmt) = self.parse_statement() {
                        public_block_ast.push(stmt);
                    } else if !self.is_at_end() && self.peek().kind != TokenKind::RBrace {
                        self.advance(); // تفادي infinite loop
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
                    if let Some(stmt) = self.parse_statement() {
                        private_block_ast.push(stmt);
                    } else if !self.is_at_end() && self.peek().kind != TokenKind::RBrace {
                        self.advance(); // تفادي infinite loop
                    }
                }

                self.consume(TokenKind::RBrace, "Expected '}' to close private block")?;
                if self.peek().kind == TokenKind::SemiColon {
                    self.advance();
                }
                continue;
            }

            // --- أي شيء تاني يعتبر عبارة برمجية (Statement) مباشرة ---
            if let Some(stmt) = self.parse_statement() {
                statements.push(stmt);
            } else if !self.is_at_end() && self.peek().kind != TokenKind::RBrace {
                self.advance(); // تفادي infinite loop لو فيه خطأ
            }
        }

        // 5. نتأكد من '}'
        self.consume(TokenKind::RBrace, "Expected '}' to close scope body")?;

        // نحدد is_custom بناءً على الـ scope_type
        let is_custom = scope_type == "custom";

        Ok(Stmt::ScopeDecl {
            is_exported: false,
            name,
            scope_type,
            is_custom,
            params,
            return_type,
            flags,
            settings,
            events,
            handles,
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
    fn parse_fn_decl(&mut self) -> Result<Stmt, String> {
        self.advance(); // 'fn'
        let name = self.get_identifier("Expected function name")?;

        self.consume(TokenKind::LParen, "Expected '(' after function name")?;
        let mut params = Vec::new();
        if self.peek().kind != TokenKind::RParen {
            loop {
                let p_name = self.get_identifier("Expected parameter name")?;
                self.consume(TokenKind::Colon, "Expected ':' after parameter name")?;
                let (base_type, size) = self.parse_type()?;

                params.push(Stmt::VarDecl {
                    is_exported: false,
                    is_static: false,
                    is_const: false,
                    base_type: Some(base_type),
                    size,
                    name: p_name,
                    value: Expr::Identifier("__param__".to_string()),
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
            Some(crate::parser::ast::TypeRef { base_type, size })
        } else {
            None
        };

        self.consume(TokenKind::LBrace, "Expected '{' to open function body")?;
        let statements = self.parse_block()?;

        Ok(Stmt::ScopeDecl {
            is_exported: false,
            name,
            scope_type: "fn".to_string(),
            is_custom: false,
            params,
            return_type,
            flags: Vec::new(),
            settings: Vec::new(),
            events: Vec::new(),
            handles: Vec::new(),
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
    fn parse_scope_type_expr(&mut self) -> Result<String, String> {
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
    fn parse_var_decl_bare(&mut self) -> Result<Stmt, String> {
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
            is_exported: false,
            is_static: false,
            is_const: false,
            base_type,
            size,
            name,
            value,
        })
    }

    fn is_type_token(kind: &TokenKind) -> bool {
        match kind {
            TokenKind::TypeInt
            | TokenKind::TypeFloat
            | TokenKind::TypeStr
            | TokenKind::TypeArray
            | TokenKind::TypeBool
            | TokenKind::TypeChar
            | TokenKind::TypeName
            | TokenKind::TypeLength
            | TokenKind::TypeSize
            | TokenKind::TypeScope
            | TokenKind::TypeFlag
            | TokenKind::TypeParam
            | TokenKind::TypeType
            | TokenKind::TypeBluePrint
            | TokenKind::TypeInit
            | TokenKind::TypeStatic
            | TokenKind::TypePublic
            | TokenKind::TypePrivate
            | TokenKind::TypeEvent
            | TokenKind::TypeHandle
            | TokenKind::TypeCustom
            | TokenKind::TypeStruct
            | TokenKind::TypeVoid
            | TokenKind::TypeString
            | TokenKind::TypeBlock
            | TokenKind::TypeObject => true,
            _ => false,
        }
    }

    /// Helper موحد — يقرأ token يمثل type ويرجع اسمه كـ String.
    /// يقبل:
    ///   Primitives: int, float, str, bool, char
    ///   Context/Magic types: name, length, size, scope, flag, param,
    ///                        type, blueprint, init, static, public, private,
    ///                        event, handle, custom
    /// يرجع None لو الـ token الحالي مش type أصلاً.
    fn parse_type(&mut self) -> Result<(String, Option<i64>), String> {
        let type_name = match &self.peek().kind {
            TokenKind::TypeInt => "int",
            TokenKind::TypeFloat => "float",
            TokenKind::TypeStr => "str",
            TokenKind::TypeArray => "array",
            TokenKind::TypeBool => "bool",
            TokenKind::TypeChar => "char",
            TokenKind::TypeName => "name",
            TokenKind::TypeLength => "length",
            TokenKind::TypeSize => "size",
            TokenKind::TypeScope => "scope",
            TokenKind::TypeFlag => "flag",
            TokenKind::TypeParam => "param",
            TokenKind::TypeType => "type",
            TokenKind::TypeBluePrint => "blueprint",
            TokenKind::TypeInit => "init",
            TokenKind::TypeStatic => "static",
            TokenKind::TypePublic => "public",
            TokenKind::TypePrivate => "private",
            TokenKind::TypeEvent => "event",
            TokenKind::TypeHandle => "handle",
            TokenKind::TypeCustom => "custom",
            TokenKind::TypeStruct => "struct",
            TokenKind::TypeVoid => "void",
            TokenKind::TypeString => "string",
            TokenKind::TypeBlock => "block",
            TokenKind::TypeObject => "object",
            TokenKind::Identifier(n) => return Ok((n.clone(), None)), // allow user-defined types (classes/structs)
            _ => return Err("Expected a type".to_string()),
        };
        let result = type_name.to_string();
        self.advance();

        // Enforce sizes for int and float
        let mut size = None;
        if result == "int" || result == "float" {
            if self.peek().kind == TokenKind::LParen {
                self.advance();
                if let TokenKind::Int(s) = self.peek().kind {
                    if result == "int" && ![8, 16, 32, 64, 128].contains(&s) {
                        return Err(format!(
                            "Syntax Error: Invalid size {} for int. Allowed: 8, 16, 32, 64, 128",
                            s
                        ));
                    }
                    if result == "float" && ![32, 64].contains(&s) {
                        return Err(format!(
                            "Syntax Error: Invalid size {} for float. Allowed: 32, 64",
                            s
                        ));
                    }
                    size = Some(s);
                    self.advance();
                } else {
                    return Err(format!(
                        "Syntax Error: Expected integer size for type {}",
                        result
                    ));
                }
                self.consume(TokenKind::RParen, "Expected ')' after type size")?;
            } else {
                return Err(format!(
                    "Syntax Error: Type '{}' requires a size, e.g., {}(32)",
                    result, result
                ));
            }
        } else if result == "array" {
            if self.peek().kind == TokenKind::LParen {
                self.advance();
                let (inner_type, inner_size) = self.parse_type()?;
                let formatted_inner = if let Some(s) = inner_size {
                    format!("{}({})", inner_type, s)
                } else {
                    inner_type
                };
                self.consume(TokenKind::RParen, "Expected ')' after array inner type")?;
                return Ok((format!("array({})", formatted_inner), None));
            } else {
                return Err(
                    "Syntax Error: array type requires an inner type, e.g., array(int(32))"
                        .to_string(),
                );
            }
        } else if self.peek().kind == TokenKind::LParen {
            // Optional size for other types (e.g. str(256))
            self.advance();
            if let TokenKind::Int(s) = self.peek().kind {
                size = Some(s);
                self.advance();
            }
            self.consume(TokenKind::RParen, "Expected ')' after type size")?;
        }

        Ok((result, size))
    }

    // ====================================================
    // Expression Parser — Pratt / Top-Down Operator Precedence
    // ====================================================
    //
    // فكرة Pratt: كل operator ليه "binding power" (قوة ربط).
    // parse_expr(min_bp) بتاكل operators طالما قوتها أكبر من min_bp.
    // ده بيحل مشكلة الـ precedence بشكل طبيعي.
    //
    // المستويات:
    //   or/||       -> 1
    //   and/&&      -> 2
    //   == / !=     -> 3
    //   < / >       -> 4
    //   + / -       -> 5
    //   * / / / %   -> 6
    //   Unary ! -   -> prefix, right-associative (bp = 7)
    //   . () []     -> postfix, left-associative (bp = 8)

    fn parse_expression(&mut self) -> Result<Expr, String> {
        self.parse_expr(0)
    }

    /// الدالة الأساسية لـ Pratt Parser.
    /// `min_bp`: أدنى binding power مقبول في الجانب الأيمن.
    fn parse_expr(&mut self, min_bp: u8) -> Result<Expr, String> {
        // --- Prefix: اقرأ الـ left-hand side أولاً ---
        let mut lhs = self.parse_prefix()?;

        // --- Infix / Postfix: استمر طالما في operators بقوة كافية ---
        loop {
            // نقطة الوصول (postfix): . و ()
            if let Some(postfix_bp) = self.postfix_binding_power() {
                if postfix_bp < min_bp {
                    break;
                }
                lhs = self.parse_postfix(lhs)?;
                continue;
            }

            // operator وسطي (infix): +, -, *, etc.
            if let Some((left_bp, right_bp)) = self.infix_binding_power() {
                if left_bp < min_bp {
                    break;
                }
                let op_str = self.current_op_str();
                self.advance(); // نتخطى الـ operator
                let rhs = self.parse_expr(right_bp)?;
                lhs = Expr::BinaryOp {
                    left: Box::new(lhs),
                    operator: op_str,
                    right: Box::new(rhs),
                };
                continue;
            }

            break;
        }

        Ok(lhs)
    }

    /// يقرأ prefix expressions: literals، identifiers، unary ops، grouped.
    fn parse_prefix(&mut self) -> Result<Expr, String> {
        let line = self.peek().line;
        let col = self.peek().column;

        match &self.peek().kind.clone() {
            // --- Literals ---
            TokenKind::Super => {
                self.advance();
                Ok(Expr::Super)
            }
            TokenKind::This => {
                self.advance();
                Ok(Expr::This)
            }
            TokenKind::Global => {
                self.advance();
                Ok(Expr::Global)
            }
            TokenKind::Int(v) => {
                let val = *v;
                self.advance();
                Ok(Expr::LiteralInt(val))
            }
            TokenKind::Float(v) => {
                let val = *v;
                self.advance();
                Ok(Expr::LiteralFloat(val))
            }
            TokenKind::String(s) => {
                let val = s.clone();
                self.advance();
                Ok(Expr::LiteralString(val))
            }
            TokenKind::Bool(b) => {
                let val = *b;
                self.advance();
                Ok(Expr::LiteralBool(val))
            }

            // --- Identifier ---
            TokenKind::Identifier(name) => {
                let val = name.clone();
                self.advance();
                Ok(Expr::Identifier(val))
            }

            // --- Unary: !expr ---
            TokenKind::Not => {
                self.advance();
                let operand = self.parse_expr(7)?; // right-binding power = 7
                Ok(Expr::UnaryOp {
                    operator: "!".to_string(),
                    operand: Box::new(operand),
                })
            }

            // --- Unary: -expr ---
            TokenKind::Minus => {
                self.advance();
                let operand = self.parse_expr(7)?;
                Ok(Expr::UnaryOp {
                    operator: "-".to_string(),
                    operand: Box::new(operand),
                })
            }

            // --- Arrays: [1, 2, 3] ---
            TokenKind::LBracket => {
                self.advance();
                let mut elements = Vec::new();
                if self.peek().kind != TokenKind::RBracket {
                    elements.push(self.parse_expr(0)?);
                    while self.peek().kind == TokenKind::Comma {
                        self.advance();
                        elements.push(self.parse_expr(0)?);
                    }
                }
                self.consume(TokenKind::RBracket, "Expected ']' to close array literal")?;
                Ok(Expr::ListLiteral(elements))
            }

            // --- Instantiate: new/copy/modify Target(args) ---
            TokenKind::New | TokenKind::Copy | TokenKind::Modify => {
                let op = match self.peek().kind {
                    TokenKind::New => "new",
                    TokenKind::Copy => "copy",
                    TokenKind::Modify => "modify",
                    _ => unreachable!(),
                }
                .to_string();
                self.advance();

                // Target can be an identifier (like Counter), or another expression
                let target = self.parse_expr(9)?; // Bind tightly to the target (higher than postfix 8)

                // Optional arguments
                let mut args = Vec::new();
                if self.peek().kind == TokenKind::LParen {
                    self.advance();
                    if self.peek().kind != TokenKind::RParen {
                        args.push(self.parse_expr(0)?);
                        while self.peek().kind == TokenKind::Comma {
                            self.advance();
                            args.push(self.parse_expr(0)?);
                        }
                    }
                    self.consume(TokenKind::RParen, "Expected ')' to close arguments")?;
                }

                Ok(Expr::Instantiate {
                    op,
                    target: Box::new(target),
                    args,
                })
            }

            // --- Grouped: (expr) ---
            TokenKind::LParen => {
                self.advance(); // نتخطى '('
                let inner = self.parse_expr(0)?;
                self.consume(
                    TokenKind::RParen,
                    "Expected ')' to close grouped expression",
                )?;
                Ok(inner)
            }

            // --- Object Literals: { stmt; stmt; } ---
            TokenKind::LBrace => {
                self.advance(); // نتخطى '{'
                let stmts = self.parse_block()?;
                Ok(Expr::ObjectLiteral(stmts))
            }

            // --- Keywords used as identifier expressions (e.g. `flag && check`) ---
            other => {
                if let Some(kw_name) = Self::keyword_as_identifier(other) {
                    self.advance();
                    Ok(Expr::Identifier(kw_name))
                } else {
                    Err(format!(
                        "Syntax Error: Unexpected token '{:?}' in expression at line {}, column {}",
                        other, line, col
                    ))
                }
            }
        }
    }

    /// يرجع الـ left binding power للـ postfix operators (. و ())
    /// None لو اللي قدامنا مش postfix operator.
    fn postfix_binding_power(&self) -> Option<u8> {
        match &self.peek().kind {
            TokenKind::Dot => Some(8),        // property access: obj.field
            TokenKind::LParen => Some(8),     // function call:   foo(...)
            TokenKind::LBracket => Some(8),   // array indexing: arr[0]
            TokenKind::PlusPlus => Some(9),   // postfix ++
            TokenKind::MinusMinus => Some(9), // postfix --
            _ => None,
        }
    }

    /// يقرأ postfix operation على الـ lhs اللي اتبنى قبل كده.
    fn parse_postfix(&mut self, lhs: Expr) -> Result<Expr, String> {
        match &self.peek().kind.clone() {
            // --- Property Access: lhs.identifier ---
            TokenKind::Dot => {
                self.advance(); // نتخطى '.'
                                // نقبل identifiers وكمان keywords كـ field names (زي .length, .size)
                let prop = if let TokenKind::Identifier(name) = &self.peek().kind.clone() {
                    let n = name.clone();
                    self.advance();
                    n
                } else if let Some(kw_name) = Self::keyword_as_identifier(&self.peek().kind.clone())
                {
                    self.advance();
                    kw_name
                } else {
                    return Err(format!(
                        "Syntax Error: Expected field name after '.' at line {}, column {}",
                        self.peek().line,
                        self.peek().column
                    ));
                };
                Ok(Expr::PropertyAccess {
                    object: Box::new(lhs),
                    property: prop,
                })
            }

            // --- Namespace Access: lhs::identifier ---
            TokenKind::DoubleColon => {
                self.advance(); // نتخطى '::'
                let prop = if let TokenKind::Identifier(name) = &self.peek().kind.clone() {
                    let n = name.clone();
                    self.advance();
                    n
                } else {
                    return Err(format!(
                        "Syntax Error: Expected name after '::' at line {}, column {}",
                        self.peek().line,
                        self.peek().column
                    ));
                };

                let namespace = if let Expr::Identifier(n) = lhs {
                    n
                } else {
                    return Err(
                        "Syntax Error: Expected namespace identifier before '::'".to_string()
                    );
                };

                Ok(Expr::NamespaceAccess {
                    namespace,
                    property: Box::new(Expr::Identifier(prop)),
                })
            }

            // --- Function Call: lhs(arg1, arg2, ...) ---
            TokenKind::LParen => {
                self.advance(); // نتخطى '('
                let mut args = Vec::new();

                // اقرأ الـ arguments لو مش قائمة فاضية
                if self.peek().kind != TokenKind::RParen {
                    args.push(self.parse_expr(0)?);
                    while self.peek().kind == TokenKind::Comma {
                        self.advance(); // نتخطى ','
                        args.push(self.parse_expr(0)?);
                    }
                }

                self.consume(
                    TokenKind::RParen,
                    "Expected ')' to close function call argument list",
                )?;
                Ok(Expr::Call {
                    callee: Box::new(lhs),
                    args,
                })
            }

            TokenKind::PlusPlus => {
                self.advance();
                Ok(Expr::PostfixUpdate {
                    left: Box::new(lhs),
                    operator: "++".to_string(),
                })
            }

            TokenKind::MinusMinus => {
                self.advance();
                Ok(Expr::PostfixUpdate {
                    left: Box::new(lhs),
                    operator: "--".to_string(),
                })
            }

            // --- Array Indexing: lhs[index] ---
            TokenKind::LBracket => {
                self.advance();
                let index = self.parse_expr(0)?;
                self.consume(TokenKind::RBracket, "Expected ']' after array index")?;
                Ok(Expr::IndexAccess {
                    object: Box::new(lhs),
                    index: Box::new(index),
                })
            }

            other => Err(format!(
                "Internal error: parse_postfix called with non-postfix token '{:?}'",
                other
            )),
        }
    }

    /// يرجع (left_bp, right_bp) للـ infix operators.
    /// left_bp   = الـ binding power للجانب الأيسر (تحديد ما إذا كان الـ operator يسرق الـ lhs).
    /// right_bp  = الـ binding power اللي بنمرره للـ parse_expr الـ recursive للجانب الأيمن.
    /// None لو اللي قدامنا مش infix operator.
    fn infix_binding_power(&self) -> Option<(u8, u8)> {
        match &self.peek().kind {
            TokenKind::Or => Some((1, 2)),  // left-associative
            TokenKind::And => Some((3, 4)), // left-associative
            TokenKind::Eq => Some((5, 6)),  // left-associative
            TokenKind::NotEq => Some((5, 6)),
            TokenKind::Less => Some((7, 8)),
            TokenKind::Greater => Some((7, 8)),
            TokenKind::Plus => Some((9, 10)), // left-associative
            TokenKind::Minus => Some((9, 10)),
            TokenKind::Multiply => Some((11, 12)),
            TokenKind::Divide => Some((11, 12)),
            TokenKind::Mod => Some((11, 12)),
            _ => None,
        }
    }

    /// يرجع string representation للـ operator اللي قدامنا حالياً.
    /// يُستدعى قبل advance() في parse_expr.
    fn current_op_str(&self) -> String {
        match &self.peek().kind {
            TokenKind::Plus => "+".to_string(),
            TokenKind::Minus => "-".to_string(),
            TokenKind::Multiply => "*".to_string(),
            TokenKind::Divide => "/".to_string(),
            TokenKind::Mod => "%".to_string(),
            TokenKind::Eq => "==".to_string(),
            TokenKind::NotEq => "!=".to_string(),
            TokenKind::Less => "<".to_string(),
            TokenKind::Greater => ">".to_string(),
            TokenKind::And => "&&".to_string(),
            TokenKind::Or => "||".to_string(),
            other => format!("{:?}", other),
        }
    }
}
