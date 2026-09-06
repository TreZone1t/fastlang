use crate::frontend::lexer::token::TokenKind;
use crate::frontend::parser::ast::*;
use crate::frontend::parser::parser::Parser;

impl Parser {
    /// Parses: `machine Name -> { @label -> { ... } ... handle -> { fn call(...) ... fn leave()... } }`
    ///
    /// A `machine` is a pure state machine construct:
    /// - Only `@labels` and `goto -> @label` and `leave` are allowed at the top level.
    /// - `@init` label (if present) runs only ONCE (guarded by `__initialized` flag).
    /// - Persistent data is stored via `this.field = value` inside any label.
    /// - Only `call` and `leave` handles are allowed.
    pub(crate) fn parse_machine_decl(&mut self) -> Result<Decl, String> {
        self.advance(); // consume 'machine'

        // optional return type via generics syntax: machine<int32> name
        let mut return_type = BaseType::Unknown;
        if self.peek().kind == TokenKind::Less {
            let mut generics = Vec::new();
            self.parse_generics(&mut generics)?;
            if !generics.is_empty() {
                return_type = generics[0].clone();
            }
        }

        let name = self.get_identifier("Expected machine name after 'machine' or '<type>'")?;

        // optional return type via arrow syntax: machine name -> int32
        if self.peek().kind == TokenKind::Arrow {
            self.advance(); // consume '->'
            if self.peek().kind != TokenKind::LBrace {
                return_type = self.parse_type()?;
                // optional arrow before brace
                if self.peek().kind == TokenKind::Arrow {
                    self.advance();
                }
            }
        }
        self.consume(TokenKind::LBrace, "Expected '{' to open machine body")?;

        let mut labels: Vec<Decl> = Vec::new();
        let mut handle_block: Vec<Decl> = Vec::new();

        while !self.is_at_end() && self.peek().kind != TokenKind::RBrace {
            match self.peek().kind.clone() {
                TokenKind::LabelName(_) => {
                    let label = self.parse_label_decl(ScopeType::Label)?;
                    if let Decl::LabelDecl { name: ref lbl_name, .. } = label {
                        if labels.iter().any(|l| matches!(l, Decl::LabelDecl { name: n, .. } if n == lbl_name)) {
                            return Err(format!("Syntax Error: Duplicate label '{}' in machine '{}'", lbl_name, name));
                        }
                        labels.push(label);
                    }
                }
                TokenKind::Handle => {
                    self.advance(); // consume 'handle'
                    let mut _used: Vec<HandleMethods> = Vec::new();
                    let allowed = self.get_allowed_handle(&BaseType::Machine {
                        name: name.clone(),
                        fields: Box::new(std::collections::HashMap::new()),
                        methods: Box::new(std::collections::HashMap::new()),
                        labels: Box::new(std::collections::HashMap::new()),
                        mode: ExecutionMode::Runtime,
                    })?;
                    let block = self.parse_handle_body(&mut _used, allowed)?;
                    // Only allow `call` and `leave` handles in machine
                    /*  for h in &block {
                        if let Decl::FnDecl { name: fn_name, .. } = h {
                            if fn_name != "call" && fn_name != "leave" {
                                return Err(
                                    format!(
                                        "Syntax Error: machine '{}' handle only allows 'call' and 'leave', found '{}'",
                                        name,
                                        fn_name
                                    )
                                );
                            }
                        }
                    }
                    */
                    handle_block = block;
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
            visibility: Visibility::Private,
            name,
            return_type,
            labels,
            handle_block,
        })
    }
}
