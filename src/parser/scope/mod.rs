//we will move parsing the scopes here
// fn - function
// block - block
// class - class
// struct - struct
// custom - custom
// looped - looped
// case - case
// array - array
// str - str
use crate::lexer::token::TokenKind;
use crate::parser::ast::*;
use crate::parser::parser::Parser;
pub mod block;
pub mod builtins;
pub mod class;
pub mod custom;
pub mod enum_decl;
pub mod struct_decl;

impl Parser {
    pub fn get_handle_type(&self, t: TokenKind) -> HandleMethods {
        let name = HandleMethods::from_str(t.as_str());
        return name;
    }
    pub fn is_valid_handle(&mut self, handles: Vec<HandleMethods>, t: TokenKind) -> bool {
        eprintln!(
            "HANDLE CHECK: as_str={:?} kind={:?}",
            t.as_str(),
            self.peek().kind
        );
        let t_h = self.get_handle_type(t);
        for h in handles {
            if h == t_h {
                eprintln!("HANDLE CHECK: FOUND");
                return true;
            }
        }
        false
    }

    pub fn is_valid_setting(&mut self, t: TokenKind) -> bool {
        if let TokenKind::LabelName(_) = t {
            return true;
        }
        let mut all_settings: Vec<Setting> = Vec::new();
        all_settings.push(Setting::CustomIndexAccess);
        all_settings.push(Setting::CustomConstructor);
        all_settings.push(Setting::CustomIterator);
        all_settings.push(Setting::CustomDisplay);
        all_settings.push(Setting::CustomGeneric);
        all_settings.push(Setting::CustomOperators);
        all_settings.push(Setting::Param);
        all_settings.push(Setting::Private);
        all_settings.push(Setting::Public);
        all_settings.push(Setting::Static);
        all_settings.push(Setting::Length);
        all_settings.push(Setting::Extends);
        all_settings.push(Setting::Variants);
        all_settings.push(Setting::Leave);
        all_settings.push(Setting::Yield);
        all_settings.push(Setting::Goto);
        all_settings.push(Setting::Label);
        all_settings.push(Setting::Data);
        all_settings.push(Setting::Call);
        all_settings.push(Setting::Error);
        all_settings.push(Setting::Statement);
        all_settings.push(Setting::Constructor);
        all_settings.push(Setting::Handle);
        all_settings.push(Setting::Return);
        let t_s = Setting::from_token(t);
        for s in all_settings {
            if s == t_s {
                return true;
            }
        }
        false
    }

    pub(crate) fn parse_scope_decl(&mut self) -> Result<Stmt, String> {
        let token_scope_type = self.peek().kind.clone();
        let name;
        let res;
        let scope_type = match token_scope_type {
            TokenKind::TypeClass => {
                self.advance();
                if matches!(self.peek().kind, TokenKind::Identifier(_)) {
                    // we need to change the value of name
                    name = self.get_identifier("Expected scope name")?;
                    // consume the name then ->
                    self.consume(TokenKind::Arrow, "Expected '->' after scope name")?;
                    //{
                    self.advance();
                    ScopeType::Class
                } else {
                    return Err(format!(
                        "Syntax Error: Expected scope name after 'class' at line {}, column {}",
                        self.peek().line,
                        self.peek().column
                    ));
                }
            }
            TokenKind::TypeCustom => {
                self.advance(); // consume 'custom'
                if matches!(self.peek().kind, TokenKind::Identifier(_)) {
                    // we need to change the value of name
                    name = self.get_identifier("Expected scope name")?;
                    // consume the name then ->
                    self.consume(TokenKind::Arrow, "Expected '->' after scope name")?;
                    //{
                    self.advance();
                    // we expect a whitespace until we find the next token which will be type of scope
                    self.consume(
                        TokenKind::Enable,
                        "Expected  type  in the first of the scope body",
                    )?;
                    ScopeType::Custom
                } else {
                    return Err(format!(
                        "Syntax Error: Expected scope name after 'class' at line {}, column {}",
                        self.peek().line,
                        self.peek().column
                    ));
                }
            }
            TokenKind::TypeEnum => {
                self.advance(); // consume 'enum'
                if matches!(self.peek().kind, TokenKind::Identifier(_)) {
                    // we need to change the value of name
                    name = self.get_identifier("Expected scope name")?;
                    ScopeType::Enum
                } else {
                    return Err(
                        "Syntax Error: Expected scope name after 'enum' at line {}, column {}"
                            .to_string(),
                    );
                }
            }
            TokenKind::TypeStruct => {
                self.advance(); // consume 'struct'
                if matches!(self.peek().kind, TokenKind::Identifier(_)) {
                    // we need to change the value of name
                    name = self.get_identifier("Expected scope name")?;
                    ScopeType::Struct
                } else {
                    return Err(
                        "Syntax Error: Expected scope name after 'struct' at line {}, column {}"
                            .to_string(),
                    );
                }
            }
            TokenKind::TypeScope => {
                self.advance(); // consume 'scope'
                                // Check if next is 'name'
                if matches!(self.peek().kind, TokenKind::Identifier(_)) {
                    // we need to change the value of name
                    name = self.get_identifier("Expected scope name")?;
                    // consume the name then ->
                    self.consume(TokenKind::Arrow, "Expected '->' after scope name")?;
                    //{
                    self.advance();
                    self.consume(
                        TokenKind::TypeType,
                        "Expected  type  in the first of the scope body",
                    )?;
                    // consume -> and then the type
                    self.advance();
                    let type_node = self.peek().kind.clone();
                    // we expect one of the following types
                    //array , str , block , class , struct , enum , fn , custom
                    match type_node {
                        TokenKind::TypeBlock => {
                            self.advance();
                            self.consume(TokenKind::SemiColon, "Expected ';' after scope type")?;
                            ScopeType::Block
                        }
                        TokenKind::TypeClass => {
                            self.advance();
                            self.consume(TokenKind::SemiColon, "Expected ';' after scope type")?;
                            ScopeType::Class
                        }
                        TokenKind::TypeStruct => {
                            self.advance();
                            self.consume(TokenKind::SemiColon, "Expected ';' after scope type")?;
                            ScopeType::Struct
                        }
                        TokenKind::TypeEnum => {
                            self.advance();
                            self.consume(TokenKind::SemiColon, "Expected ';' after scope type")?;
                            ScopeType::Enum
                        }
                        TokenKind::Fn => {
                            self.advance();
                            self.consume(TokenKind::SemiColon, "Expected ';' after scope type")?;
                            ScopeType::Fn
                        }
                        TokenKind::TypeCustom => {
                            self.advance();
                            self.consume(TokenKind::SemiColon, "Expected ';' after scope type")?;
                            ScopeType::Custom
                        }
                        _ => {
                            return Err("Syntax Error: Expected scope type after 'type' at line {}, column {} /n the types are array , str , block , class , struct , enum , fn , custom".to_string());
                        }
                    }
                } else {
                    return Err(format!(
                        "Syntax Error: Expected scope name after 'scope' at line {}, column {}",
                        self.peek().line,
                        self.peek().column
                    ));
                }
            }
            _ => todo!(),
        };
        res = match scope_type {
            ScopeType::Block => self.parse_block_decl(name),
            ScopeType::Class => self.parse_class_decl(name), // */
            ScopeType::Custom => self.parse_custom_decl(name), //*  */
            ScopeType::Enum => self.parse_enum_decl(name),   //* */
            ScopeType::Fn => self.parse_fn_decl(name),       //*  */
            ScopeType::Struct => self.parse_struct_decl(name), //* */
            _ => {
                return Err("unknown scope type error".to_string());
            }
        };
        return res;
    }
    //====================================================================
}
