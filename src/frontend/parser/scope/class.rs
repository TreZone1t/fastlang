use std::collections::HashMap;

use cranelift_codegen::isa::x64::args::CC::S;

use crate::frontend::lexer::token::TokenKind;
use crate::frontend::parser::ast::*;
use crate::frontend::parser::parser::Parser;

impl Parser {
    pub(crate) fn parse_class_decl(&mut self) -> Result<Decl, String> {
        let mut enabled_settings: Vec<Setting> = Vec::new();
        let mut used_settings: Vec<Setting> = Vec::new();
        let mut constructor: Option<Vec<ConstructorDecl>> = None;
        let mut enabled_handles: Vec<HandleMethods> = Vec::new();
        let mut used_handles: Vec<HandleMethods> = Vec::new();
        let mut handle_block: Vec<Decl> = Vec::new();
        let mut public_block: Vec<Decl> = Vec::new();
        let mut private_block: Vec<Decl> = Vec::new();
        let mut static_block: Vec<Decl> = Vec::new();
        let mut generics: Vec<BaseType> = Vec::new();
        let mut length: i64 = 0;

        //we need to ensure no duplicated extends
        let mut extends = None;
        let mut has_extends = false;
        self.advance(); // 'class'
        let name = self.get_identifier("Expected class name")?;
        //====================================================================
        // generic <> :  class name<...> extends <name> -> { ... }
        //====================================================================
        if self.peek().kind == TokenKind::Less {
            self.parse_generics(&mut generics)?;
        }
        let mut meta = TypeMetadata {
            name: name.clone(),
            fields: HashMap::new(),
            constructor: None,
            params: Vec::new(),
            generics: Vec::new(),
            methods: HashMap::new(),
            handles: Vec::new(),
            vars: HashMap::new(),
            is_enum: false,
            variants: None,
        };
        //adding the default settings to the class scope
        enabled_settings.push(Setting::CustomIndexAccess);
        enabled_settings.push(Setting::Private);
        enabled_settings.push(Setting::Public);
        enabled_settings.push(Setting::Static);
        enabled_settings.push(Setting::Extends);
        // adding allowed handles
        //we have display , iterator , next , length , size
        enabled_handles.push(HandleMethods::IndexAccess);
        enabled_handles.push(HandleMethods::Display);
        enabled_handles.push(HandleMethods::Iterator);
        enabled_handles.push(HandleMethods::Next);
        enabled_handles.push(HandleMethods::Length);
        if self.peek().kind == TokenKind::Extends {
            self.advance();
            extends = Some(self.get_identifier("Expected parent class name after 'extends'")?);
            has_extends = true;
        }
        while !self.is_at_end() && self.peek().kind != TokenKind::RBrace {
            // we need to check if the token is valid for the setting
            let t = self.peek().kind.clone();
            if self.is_valid_setting(t.clone()) {
                // now need to know what is this section
                //====================================================================
                // constructor    _ () -> { ... }
                //====================================================================
                if t == TokenKind::Constructor && !used_settings.contains(&Setting::Constructor) {
                    match self.parse_constructor_decl(&mut meta) {
                        Ok(c) => constructor = c,
                        Err(e) => {
                            eprintln!("Syntax Error in scope constructor: {}", e);
                            self.synchronize();
                        }
                    }
                    used_settings.push(Setting::Constructor);
                    continue;
                }
                //====================================================================
                // handle -> { fn1 , fn2 , ... }
                //====================================================================
                if t == TokenKind::Handle && !used_settings.contains(&Setting::Handle) {
                    handle_block =
                        self.parse_handle_block(&mut enabled_handles, &mut used_handles)?;
                    used_settings.push(Setting::Handle);
                    continue;
                }
                //====================================================================
                // public -> { ... }
                //====================================================================
                if self.peek().kind == TokenKind::Public
                    && !used_settings.contains(&Setting::Public)
                {
                    public_block =
                        self.parse_field_block(&mut meta, Visibility::Public, generics.clone())?;
                    used_settings.push(Setting::Public);
                    continue;
                }
                //====================================================================
                // private -> { ... }
                //====================================================================
                if t == TokenKind::Private && !used_settings.contains(&Setting::Private) {
                    private_block =
                        self.parse_field_block(&mut meta, Visibility::Private, generics.clone())?;
                    used_settings.push(Setting::Private);
                    continue;
                }
                //====================================================================
                // static -> { ... }
                //====================================================================
                if t == TokenKind::Static && !used_settings.contains(&Setting::Static) {
                    static_block =
                        self.parse_field_block(&mut meta, Visibility::Static, generics.clone())?;
                    used_settings.push(Setting::Static);
                    continue;
                }
                //====================================================================
                // length -> <value>;
                //====================================================================
                if t == TokenKind::TypeLength && !used_settings.contains(&Setting::Length) {
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
                    meta.vars.insert(
                        "length".to_string(),
                        VarMetadata {
                            name: "length".to_string(),
                            type_node: BaseType::Int32,
                            visibility: Visibility::Public,
                            editability: Editability::Editable,
                            scope: ScopeType::Custom,
                            is_heaped: false,
                            is_array: false,
                        },
                    );
                    used_settings.push(Setting::Length);
                    continue;
                }

                //====================================================================
                // extends -> <name>;
                //====================================================================
                if t == TokenKind::Extends && !used_settings.contains(&Setting::Extends) {
                    if has_extends {
                        return Err("Syntax Error: Class can only have one extends and you already have one".to_string());
                    } else {
                        self.advance(); // 'extends'
                        self.consume(TokenKind::Arrow, "Expected '->' after 'extends'")?;
                        extends = Some(
                            self.get_identifier("Expected parent class name after 'extends'")?,
                        );
                        used_settings.push(Setting::Extends);
                        continue;
                    }
                }
            } else {
                print!("DEBUG: Invalid field found : {} , that is not allow in the array typed scope to use it \n\t - use custom typed scope with enable some setting it will work if it valid" , t.as_str());
                return Err(format!(
                    "Syntax Error: Invalid field ''{:?}'' declaration at line {}, column {}",
                    t,
                    self.peek().line,
                    self.peek().column
                ));
            }
        }
        //first we need to warn the user if he is forget to use a setting he added or a handle method he added
        for e in used_settings {
            if !enabled_settings.contains(&e) {
                eprintln!(
                    "WARNING: you added a setting and not used => {} in the custom scope '{}'",
                    e.as_str(),
                    name
                );
            }
        }
        for e in used_handles {
            if !enabled_handles.contains(&e) {
                eprintln!("WARNING: you added a handle method and not used => {} in the custom scope '{}'", e.as_str(), name);
            }
        }
        for stmt in &handle_block {
            //that will add the implemented handle methods to the metadata
            if let Decl::FnDecl {
                name: fn_name,
                params,
                return_type,
                ..
            } = stmt
            {
                let fn_type = FnType {
                    name: fn_name.clone(),
                    params: params.clone(),
                    return_type: return_type.clone(),
                };
                meta.methods.insert(fn_name.clone(), fn_type);
            }
        }

        self.metadata.insert(name.clone(), meta);
        Ok(Decl::ClassDecl {
            is_exported: false,
            name,
            extends,
            handles: enabled_handles,
            settings: enabled_settings,
            length,
            public_block,
            private_block,
            static_block,
            generics,
            handle_block,
            constructor,
        })
    }
}
