use std::collections::HashMap;

use crate::frontend::lexer::token::TokenKind;
use crate::frontend::parser::ast::*;
use crate::frontend::parser::parser::Parser;

impl Parser {
    pub(crate) fn parse_custom_decl(&mut self) -> Result<Decl, String> {
        let mut used_settings: Vec<Setting> = Vec::new();
        let mut used_handles: Vec<HandleMethods> = Vec::new();

        let mut public_block: Vec<Decl> = Vec::new();
        let mut private_block: Vec<Decl> = Vec::new();
        let mut static_block: Vec<Decl> = Vec::new();
        let mut handle_block: Vec<Decl> = Vec::new();
        let mut constructor: Option<Vec<ConstructorDecl>> = None;
        let mut labels_map: HashMap<String, Decl> = HashMap::new();
        let mut data: Option<Expr> = None;

        self.advance(); // consume 'custom'
        let name = self.get_identifier("Expected custom name")?;

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

        if self.peek().kind == TokenKind::Arrow {
            self.advance();
        }
        self.consume(TokenKind::LBrace, "Expected '{' to open custom body")?;

        let allowed_settings = [
            Setting::Constructor,
            Setting::Public,
            Setting::Private,
            Setting::Static,
            Setting::Handle,
            Setting::Data,
            Setting::Label,
        ];

        while !self.is_at_end() && self.peek().kind != TokenKind::RBrace {
            let t = self.peek().kind.clone();
            let setting = match &t {
                TokenKind::LabelName(_) => Setting::Label,
                _ => Setting::from_token(t.clone()),
            };

            if !allowed_settings.contains(&setting) {
                return Err(
                    format!(
                        "Syntax Error: Invalid or forbidden setting '{}' in custom scope '{}' at line {}, column {}",
                        TokenKind::as_str(&t),
                        name,
                        self.peek().line,
                        self.peek().column
                    )
                );
            }

            if setting != Setting::Label && used_settings.contains(&setting) {
                return Err(
                    format!(
                        "Syntax Error: Duplicate '{}' block in custom '{}' at line {}, column {}",
                        TokenKind::as_str(&t),
                        name,
                        self.peek().line,
                        self.peek().column
                    )
                );
            }

            match t {
                TokenKind::Constructor => {
                    match self.parse_constructor_decl(&mut meta) {
                        Ok(c) => {
                            constructor = c;
                        }
                        Err(e) => {
                            eprintln!("Syntax Error in scope constructor: {}", e);
                            self.synchronize();
                        }
                    }
                    used_settings.push(Setting::Constructor);
                }
                TokenKind::Public => {
                    public_block = self.parse_field_block(&mut meta, Visibility::Public)?;
                    used_settings.push(Setting::Public);
                }
                TokenKind::Private => {
                    private_block = self.parse_field_block(&mut meta, Visibility::Private)?;
                    used_settings.push(Setting::Private);
                }
                TokenKind::Static => {
                    static_block = self.parse_field_block(&mut meta, Visibility::Static)?;
                    used_settings.push(Setting::Static);
                }
                TokenKind::Handle => {
                    self.advance(); // consume 'handle'
                    handle_block = self.parse_handle_body(&mut used_handles)?;
                    used_settings.push(Setting::Handle);
                }
                TokenKind::TypeData => {
                    self.advance(); // consume 'data'
                    self.consume(TokenKind::Arrow, "Expected '->' after 'data'")?;
                    let data_expr = self.parse_expression()?;
                    data = Some(data_expr);
                    self.consume(TokenKind::SemiColon, "Expected ';' after data expression")?;
                    used_settings.push(Setting::Data);
                }
                TokenKind::LabelName(ref l_name) => {
                    let label_name = l_name.clone();
                    let label_block = self.parse_label_decl(ScopeType::Custom)?;
                    labels_map.insert(label_name, label_block);
                    if !used_settings.contains(&Setting::Label) {
                        used_settings.push(Setting::Label);
                    }
                }
                _ => {
                    return Err(
                        format!(
                            "Syntax Error: Unexpected token '{:?}' in custom scope '{}' at line {}, column {}",
                            t,
                            name,
                            self.peek().line,
                            self.peek().column
                        )
                    );
                }
            }
        }

        if self.peek().kind == TokenKind::RBrace {
            self.advance();
        } else {
            return Err("Syntax Error: Expected '}' after custom scope body".to_string());
        }

        if self.peek().kind == TokenKind::SemiColon {
            self.advance();
        }

        meta.handles = used_handles.clone();
        if let Some(ref data_expr) = data {
            let inferred_base_type = match data_expr {
                Expr::LiteralInt(_) => BaseType::Int32,
                Expr::LiteralFloat(_) => BaseType::Float32,
                Expr::LiteralString(_) => BaseType::Unknown,
                Expr::LiteralChar(_) => BaseType::Char,
                Expr::LiteralBool(_) => BaseType::Bool,
                Expr::NamespaceAccess { ref namespace, .. } => BaseType::from_str(&namespace),
                Expr::Instantiate { ref target, .. } => {
                    if let Expr::Identifier(ref n) = **target {
                        BaseType::from_str(n)
                    } else {
                        BaseType::Unknown
                    }
                }
                Expr::Identifier(ref var_name) => {
                    if let Some(var_meta) = meta.vars.get(var_name) {
                        var_meta.type_node.clone()
                    } else {
                        BaseType::Unknown
                    }
                }
                Expr::PropertyAccess { ref object, ref property } => {
                    if let Expr::This = **object {
                        if let Some(var_meta) = meta.vars.get(property) {
                            var_meta.type_node.clone()
                        } else {
                            BaseType::Unknown
                        }
                    } else {
                        BaseType::Unknown
                    }
                }
                _ => BaseType::Unknown,
            };
            meta.fields.insert("data".to_string(), inferred_base_type);
        }

        for f in &public_block {
            if let Decl::VarDecl { name, type_node, .. } = f {
                meta.fields.insert(name.clone(), type_node.clone());
            } else if let Decl::FnDecl { name, params, return_type, .. } = f {
                meta.methods.insert(name.clone(), FnType {
                    name: name.clone(),
                    params: params.clone(),
                    return_type: return_type.clone(),
                });
            }
        }
        for f in &private_block {
            if let Decl::VarDecl { name, type_node, .. } = f {
                meta.fields.insert(name.clone(), type_node.clone());
            } else if let Decl::FnDecl { name, params, return_type, .. } = f {
                meta.methods.insert(name.clone(), FnType {
                    name: name.clone(),
                    params: params.clone(),
                    return_type: return_type.clone(),
                });
            }
        }

        self.metadata.insert(name.clone(), meta);

        Ok(Decl::CustomDecl {
            is_exported: false,
            name,
            settings: Some(used_settings),
            handles: Some(used_handles),
            data,
            public_block: Some(public_block),
            private_block: Some(private_block),
            static_block: Some(static_block),
            labels: if labels_map.is_empty() {
                None
            } else {
                Some(labels_map)
            },
            handle_block: Some(handle_block),
            constructor,
        })
    }
}
