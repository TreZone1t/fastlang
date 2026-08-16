use std::collections::HashMap;

use crate::lexer::token::TokenKind;
use crate::parser::ast::*;
use crate::parser::parser::Parser;

impl Parser {
    pub(crate) fn parse_class_decl(&mut self, name: String) -> Result<Stmt, String> {
        let mut settings: Vec<Setting> = Vec::new();
        let mut constructor: Option<Vec<ConstructorDecl>> = None;
        let mut handles: Vec<HandleMethods> = Vec::new();
        let mut used_handles: Vec<HandleMethods> = Vec::new();
        let mut handle_block: Vec<Stmt> = Vec::new();
        let mut public_block: Vec<Stmt> = Vec::new();
        let mut private_block: Vec<Stmt> = Vec::new();
        let mut static_block: Vec<Stmt> = Vec::new();
        let mut generics: Option<Vec<TypeNode>> = None;
        let mut length: i64 = 0;

        //we need to ensure no duplicated extends
        let mut extends = None;
        let mut has_extends = false;
        let mut name = name.clone();

        let mut meta = TypeMetadata {
            name: name.clone(),
            fields: HashMap::new(),
            constructor: None,
            params: Vec::new(),
            generics: Vec::new(),
            methods: HashMap::new(),
            handles: Vec::new(),
            vars: HashMap::new(),
         is_enum: false, variants: None, };
        //adding the default settings to the class scope
        settings.push(Setting::CustomIndexAccess);
        settings.push(Setting::Private);
        settings.push(Setting::Public);
        settings.push(Setting::Static);
        settings.push(Setting::Extends);
        // adding allowed handles
        //we have display , iterator , next , length , size
        handles.push(HandleMethods::IndexAccess);
        handles.push(HandleMethods::Display);
        handles.push(HandleMethods::Iterator);
        handles.push(HandleMethods::Next);
        handles.push(HandleMethods::Length);
        if name != "" {
            //we not been redirect by the scope parsing fn
            name = self.get_identifier("Expected class name")?;
            // if we have extends :
            if self.peek().kind == TokenKind::Extends {
                self.advance();
                extends = Some(self.get_identifier("Expected parent class name after 'extends'")?);
                has_extends = true;
            }
            self.consume(TokenKind::Arrow, "Expected '->' to open class body")?;
            self.consume(TokenKind::LBrace, "Expected '{' to open class body")?;
        }
        while !self.is_at_end() && self.peek().kind != TokenKind::RBrace {
            // we need to check if the token is valid for the setting
            let t = self.peek().kind.clone();
            if self.is_valid_setting(t.clone()) {
                // now need to know what is this section
                //====================================================================
                // constructor    _ () -> { ... }
                //====================================================================
                if t == TokenKind::Constructor {
                    match self.parse_constructor_decl(&mut meta) {
                        Ok(c) => constructor = c,
                        Err(e) => {
                            eprintln!("Syntax Error in scope constructor: {}", e);
                            self.synchronize();
                        }
                    }
                    continue;
                }
                //====================================================================
                // generic -> { ... }
                //====================================================================
                if t == TokenKind::Generic {
                    self.parse_generics(&mut generics)?;
                    continue;
                }
                //====================================================================
                // handle -> { fn1 , fn2 , ... }
                //====================================================================
                if t == TokenKind::Handle {
                    handle_block = self.parse_handle_block(&mut handles, &mut used_handles)?;
                    continue;
                }
                //====================================================================
                // public -> { ... }
                //====================================================================
                if self.peek().kind == TokenKind::Public {
                    public_block =
                        self.parse_field_block(&mut meta, Visibility::Public, generics.clone())?;
                    continue;
                }
                //====================================================================
                // private -> { ... }
                //====================================================================
                if t == TokenKind::Private {
                    private_block =
                        self.parse_field_block(&mut meta, Visibility::Private, generics.clone())?;
                    continue;
                }
                //====================================================================
                // static -> { ... }
                //====================================================================
                if t == TokenKind::Static {
                    static_block =
                        self.parse_field_block(&mut meta, Visibility::Static, generics.clone())?;
                }
                //====================================================================
                // length -> <value>;
                //====================================================================
                if t == TokenKind::TypeLength {
                    self.advance(); // consume 'length'
                    self.consume(TokenKind::Arrow, "Expected '->' after 'length'")?;
                    let value = self.parse_expression()?;
                    self.consume(TokenKind::SemiColon, "Expected ';' after length value")?;
                    length = match value {
                        Expr::LiteralInt(i) => i,
                        _ => {
                            return Err(
                                "Syntax Error: Expected integer value for length".to_string()
                            )
                        }
                    };
                    continue;
                }

                //====================================================================
                // extends -> <name>;
                //====================================================================
                if t == TokenKind::Extends {
                    if has_extends {
                        return Err("Syntax Error: Class can only have one extends and you already have one".to_string());
                    } else {
                        self.advance(); // 'extends'
                        self.consume(TokenKind::Arrow, "Expected '->' after 'extends'")?;
                        extends = Some(
                            self.get_identifier("Expected parent class name after 'extends'")?,
                        );
                        continue;
                    }
                }
            } else {
                print!("DEBUG: Invalid field found : {} , that is not allow in the array typed scope to use it \n\t - use custom typed scope with enable some setting it will work if it valid" , t.as_str());
                return Err(
                    ("Syntax Error: Invalid field  declaration at line {}, column {}").to_string(),
                );
            }
        }

        self.metadata.insert(name.clone(), meta);

        return Ok(Stmt::ClassDecl {
            is_exported: false,
            name,
            extends,
            handles,
            settings,
            length,
            public_block,
            private_block,
            static_block,
            generics,
            handle_block,
            constructor,
        });
    }
}
