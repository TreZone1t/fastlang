use crate::frontend::lexer::token::TokenKind;
use crate::frontend::parser::ast::*;
use crate::frontend::parser::parser::Parser;

impl Parser {
    pub(crate) fn parse_blueprint_decl(&mut self) -> Result<Decl, String> {
        self.consume(TokenKind::TypeBluePrint, "Expected 'blueprint'")?; // Consume 'blueprint'
        let name = self.get_identifier("Expected blueprint name")?;

        let mut generics = Vec::new();
        if self.peek().kind == TokenKind::Less {
            self.parse_generics(&mut generics)?;
        }

        self.consume(TokenKind::Arrow, "Expected '->'")?;

        let definition = if self.peek().kind == TokenKind::LBrace {
            self.advance();
            // Parse Explicit definition: { int(32) x; int(32) y; }
            let mut fields = Vec::new();
            while self.peek().kind != TokenKind::RBrace && !self.is_at_end() {
                let mut is_static = false;
                if self.peek().kind == TokenKind::Static {
                    self.advance();
                    is_static = true;
                }
                let type_node = self.parse_type()?;
                let field_name = self.get_identifier("Expected field name")?;
                self.consume(TokenKind::SemiColon, "Expected ';'")?;
                fields.push(BlueprintField {
                    is_static,
                    name: field_name,
                    type_node,
                });
            }
            self.consume(TokenKind::RBrace, "Expected '}'")?;
            BlueprintDef::Explicit(fields)
        } else if self.peek().kind == TokenKind::Assign {
            self.advance();
            // Parse FromExistingObject or FromTemporaryObject
            // Not needed for the current test, but let's implement later if needed.
            return Err(
                "Syntax Error: Only explicit blueprint definitions are currently supported."
                    .to_string(),
            );
        } else {
            return Err(
                "Syntax Error: Expected '{' after '->' in blueprint definition.".to_string(),
            );
        };

        // Match optional semicolon after blueprint block (like tests/20_blueprint.fs: blueprint Point -> {int(32) x; int(32) y;};)
        if self.peek().kind == TokenKind::SemiColon {
            self.consume(TokenKind::SemiColon, "Expected ';'")?;
        }

        let mut meta = TypeMetadata {
            name: name.clone(),
            fields: std::collections::HashMap::new(),
            constructor: None,
            params: Vec::new(),
            generics: Vec::new(),
            methods: std::collections::HashMap::new(),
            handles: Vec::new(),
            vars: std::collections::HashMap::new(),
            is_enum: false,
            variants: None,
        };

        if let BlueprintDef::Explicit(ref fields) = definition {
            for field in fields {
                meta.fields
                    .insert(field.name.clone(), field.type_node.clone());
            }
        }
        self.metadata.insert(name.clone(), meta);

        Ok(Decl::BlueprintDecl {
            is_exported: false,
            name,
            generics,
            definition,
        })
    }

    pub(crate) fn parse_impl_decl(&mut self) -> Result<Decl, String> {
        self.consume(TokenKind::Impl, "Expected 'impl'")?; // Consume 'impl'
        let target = self.get_identifier("Expected target name")?;
        self.consume(TokenKind::Arrow, "Expected '->'")?; // Consume '->'
        self.consume(TokenKind::LBrace, "Expected '{'")?;

        let mut methods: Vec<Decl> = Vec::new();
        while self.peek().kind != TokenKind::RBrace && !self.is_at_end() {
            // Parse functions inside impl
            let stmt = self.parse_statement(ScopeType::Impl)?;
            if let Some(Stmt::Declaration(s)) = stmt {
                methods.push(s);
            } else if let Some(_) = stmt {
                return Err("Expected declaration inside impl block".to_string());
            }
        }
        self.consume(TokenKind::RBrace, "Expected '}'")?;

        // Match optional semicolon after impl block
        if self.peek().kind == TokenKind::SemiColon {
            self.consume(TokenKind::SemiColon, "Expected ';'")?;
        }

        Ok(Decl::ImplDecl { target, methods })
    }
}
