use crate::lexer::token::TokenKind;
use crate::parser::ast::*;
use crate::parser::parser::Parser;

impl Parser {
    pub(crate) fn parse_struct_decl(&mut self, name: String) -> Result<Stmt, String> {
        let mut settings: Vec<crate::parser::ast::Setting> = Vec::new();
        let mut constructor: Option<crate::parser::ast::ConstructorDecl> = None;
        let mut handles: Vec<crate::parser::ast::HandleMethods> = Vec::new();
        let mut handle_block: Vec<Stmt> = Vec::new();
        let mut public_block_ast: Vec<Stmt> = Vec::new();
        let mut private_block_ast: Vec<Stmt> = Vec::new();
        let mut static_block_ast: Vec<Stmt> = Vec::new();
        let mut name = name.clone();
        //adding the default settings to the struct scope
        settings.push(crate::parser::ast::Setting::Private);
        settings.push(crate::parser::ast::Setting::Public);
        settings.push(crate::parser::ast::Setting::Static);
        // adding allowed handles
        //we have display only
        handles.push(crate::parser::ast::HandleMethods::Display);
        if name.is_empty() {
            // جاي مباشر من parse_statement → لازم نستهلك 'struct' + الاسم + '{'
            self.advance(); // consume 'struct'
            name = self.get_sc_type("Expected struct name")?;
            // الـ struct syntax المباشر: `struct Node { ... }` بدون `->`
            if self.peek().kind == TokenKind::Arrow {
                self.advance(); // consume optional '->'
            }
            self.consume(TokenKind::LBrace, "Expected '{' to open struct body")?;
        }
        // now we are the same as the one being redirected by the scope parsing fn

        while !self.is_at_end() && self.peek().kind != TokenKind::RBrace {
            // we need to check if the token is valid for the setting
            let t = self.peek().kind.clone();
            //todo : check if the settings are in the settings vec
            if self.is_valid_setting(t.clone()) {
                // now need to know what is this section
                //====================================================================
                // constructor    _ () -> { ... }
                //====================================================================
                if t == TokenKind::Init {
                    match self.parse_constructor_decl() {
                        Ok(c) => constructor = c,
                        Err(e) => {
                            eprintln!("Syntax Error in scope constructor: {}", e);
                            self.synchronize();
                        }
                    }
                    continue;
                }
                //====================================================================
                // handle -> { fn1 , fn2 , ... }
                //====================================================================
                if t == TokenKind::Handle {
                    handle_block = self.parse_handle_block(handles.clone())?;
                    continue;
                }
                //====================================================================
                // public -> { ... }
                //====================================================================
                if self.peek().kind == TokenKind::Public {
                    public_block_ast = self.parse_field_block()?;
                    continue;
                }
                //====================================================================
                // private -> { ... }
                //====================================================================
                if t == TokenKind::Private {
                    private_block_ast = self.parse_field_block()?;
                    continue;
                }
                //====================================================================
                // static -> { ... }
                //====================================================================
                if t == TokenKind::Static {
                    static_block_ast = self.parse_field_block()?;
                    continue;
                }
            } else {
                print!("DEBUG: Invalid field found : {} , that is not allow in the array typed scope to use it \n\t - use custom typed scope with enable some setting it will work if it valid" , t.as_str());
                return Err(
                    ("Syntax Error: Invalid field  declaration at line {}, column {}").to_string(),
                );
            }
        }
        return Ok(Stmt::StructDecl {
            is_exported: false,
            name,
            handles,
            settings,
            public_block: public_block_ast,
            private_block: private_block_ast,
            handle_block,
            static_block: static_block_ast,
            constructor,
        });
    }
}
