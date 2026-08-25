use crate::frontend::lexer::token::TokenKind;
use crate::frontend::parser::ast::*;
use crate::frontend::parser::parser::Parser;
use std::collections::HashMap;

impl Parser {
    /// Parses: `machine Name -> { @label -> { ... } ... handle -> { fn call(...) ... fn leave()... } }`
    ///
    /// A `machine` is a pure state machine construct:
    /// - Only `@labels` and `goto -> @label` and `leave` are allowed at the top level.
    /// - `@init` label (if present) runs only ONCE (guarded by `__initialized` flag).
    /// - Persistent data is stored via `this.field = value` inside any label.
    /// - Only `call` and `leave` handles are allowed.
    pub(crate) fn parse_machine_decl(&mut self) -> Result<Decl, String> {
        let is_exported = false; // handled by caller (export keyword)

        self.advance(); // consume 'machine'

        let name = self.get_identifier("Expected machine name after 'machine'")?;

        // optional generics (ignored for now)
        let mut _generics: Vec<BaseType> = Vec::new();
        if self.peek().kind == TokenKind::Less {
            self.parse_generics(&mut _generics)?;
        }

        self.consume(TokenKind::Arrow, "Expected '->' after machine name")?;
        self.consume(TokenKind::LBrace, "Expected '{' to open machine body")?;

        let mut labels: HashMap<String, Decl> = HashMap::new();
        let mut handle_block: Vec<Decl> = Vec::new();

        while !self.is_at_end() && self.peek().kind != TokenKind::RBrace {
            match self.peek().kind.clone() {
                TokenKind::LabelName(_) => {
                    let label = self.parse_label_decl(ScopeType::Label)?;
                    if let Decl::LabelDecl { ref name, .. } = label {
                        labels.insert(name.clone(), label);
                    }
                }
                TokenKind::Handle => {
                    self.advance(); // consume 'handle'
                    let mut _used: Vec<HandleMethods> = Vec::new();
                    let block = self.parse_handle_body(&mut _used)?;
                    // Only allow `call` and `leave` handles in machine
                    for h in &block {
                        if let Decl::FnDecl { name: fn_name, .. } = h {
                            if fn_name != "call" && fn_name != "leave" {
                                return Err(format!(
                                    "Syntax Error: machine '{}' handle only allows 'call' and 'leave', found '{}'",
                                    name, fn_name
                                ));
                            }
                        }
                    }
                    handle_block = block;
                }
                TokenKind::Export => {
                    // 'export machine' handled at top level, skip here
                    self.advance();
                }
                _ => {
                    return Err(format!(
                        "Syntax Error: Unexpected token '{}' inside machine '{}' at line {}. \
                        Only @labels and handle block are allowed.",
                        self.peek().kind.as_str(),
                        name,
                        self.peek().line
                    ));
                }
            }
        }

        self.consume(TokenKind::RBrace, "Expected '}' to close machine body")?;

        Ok(Decl::MachineDecl {
            is_exported,
            name,
            labels,
            handle_block,
        })
    }
}
