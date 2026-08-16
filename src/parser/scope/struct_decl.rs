use std::collections::HashMap;

use crate::lexer::token::TokenKind;
use crate::parser::ast::*;
use crate::parser::parser::Parser;

impl Parser {
    pub(crate) fn parse_struct_decl(&mut self, name: String) -> Result<Stmt, String> {
        let mut settings: Vec<Setting> = Vec::new();
        let mut used_settings: Vec<Setting> = Vec::new();
        let mut constructor: Option<Vec<ConstructorDecl>> = None;
        let mut handles: Vec<HandleMethods> = Vec::new();
        let mut used_handles: Vec<HandleMethods> = Vec::new();
        let mut handle_block: Vec<Stmt> = Vec::new();
        let mut public_block_ast: Vec<Stmt> = Vec::new();
        let mut private_block_ast: Vec<Stmt> = Vec::new();
        let mut static_block_ast: Vec<Stmt> = Vec::new();
        let mut name = name.clone();

        let mut meta = TypeMetadata {
            name: name.clone(),      //[*]
            fields: HashMap::new(),  //[*] we will add all of variables in the struct
            constructor: None,       //[*]
            params: Vec::new(),      //[*]
            generics: Vec::new(),    //[*]
            methods: HashMap::new(), //[*]
            handles: Vec::new(),     //[*]
            vars: HashMap::new(),    //[]
            is_enum: false,
            variants: None,
        };
        settings.push(Setting::Private);
        settings.push(Setting::Public);
        settings.push(Setting::Static);
        // adding allowed handles
        //we have display only
        handles.push(HandleMethods::Display);
        if name.is_empty() {
            // جاي مباشر من parse_statement → لازم نستهلك 'struct' + الاسم + '{'
            self.advance(); // consume 'struct'
            name = self.get_identifier("Expected struct name")?;
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
                if t == TokenKind::Constructor && !used_settings.contains(&Setting::Constructor) {
                    settings.pop_if(|s| s == &Setting::Constructor);
                    used_settings.push(Setting::Constructor);
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
                // handle -> { fn1 , fn2 , ... }
                //====================================================================
                if t == TokenKind::Handle && !used_settings.contains(&Setting::Handle) {
                    settings.pop_if(|s| s == &Setting::Handle);
                    used_settings.push(Setting::Handle);
                    handle_block = self.parse_handle_block(&mut handles, &mut used_handles)?;
                    continue;
                }
                //====================================================================
                // public -> { ... }
                //====================================================================
                if self.peek().kind == TokenKind::Public
                    && !used_settings.contains(&Setting::Public)
                {
                    settings.pop_if(|s| s == &Setting::Public);
                    used_settings.push(Setting::Public);
                    public_block_ast =
                        self.parse_field_block(&mut meta, Visibility::Public, None)?;
                    continue;
                }
                //====================================================================
                // private -> { ... }
                //====================================================================
                if t == TokenKind::Private && !used_settings.contains(&Setting::Private) {
                    settings.pop_if(|s| s == &Setting::Private);
                    used_settings.push(Setting::Private);
                    private_block_ast =
                        self.parse_field_block(&mut meta, Visibility::Private, None)?;
                    continue;
                }
                //====================================================================
                // static -> { ... }
                //====================================================================
                if t == TokenKind::Static && !used_settings.contains(&Setting::Static) {
                    settings.pop_if(|s| s == &Setting::Static);
                    used_settings.push(Setting::Static);
                    static_block_ast =
                        self.parse_field_block(&mut meta, Visibility::Private, None)?;
                    continue;
                }
            } else {
                print!("DEBUG:[ Invalid field found : {} ] or already used in this scope , that is not allow in the array typed scope to use it \n\t - use custom typed scope with enable some setting it will work if it valid" , t.as_str());
                return Err(
                    ("Syntax Error: Invalid field  declaration at line {}, column {}").to_string(),
                );
            }
        }
        self.consume(TokenKind::RBrace, "Expected '}' to close struct body")?;
        meta.generics.clear();
        meta.params.clear();
        meta.handles = used_handles.clone();

        self.metadata.insert(name.clone(), meta);

        return Ok(Stmt::StructDecl {
            is_exported: false,
            name,
            handles: used_handles,
            settings,
            public_block: public_block_ast,
            private_block: private_block_ast,
            handle_block,
            static_block: static_block_ast,
            constructor,
        });
    }
}
