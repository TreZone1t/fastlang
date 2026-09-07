use crate::frontend::lexer::token::TokenKind;
use crate::frontend::parser::ast::*;
use crate::frontend::parser::parser::Parser;

impl Parser {
    pub(crate) fn parse_generic_list(
        &mut self,
        generics: &mut Vec<BaseType>,
    ) -> Result<(), String> {
        if self.peek().kind != TokenKind::Greater {
            generics.push(self.parse_type()?);

            while self.peek().kind == TokenKind::Comma {
                self.advance();
                generics.push(self.parse_type()?);
            }
        }
        Ok(())
    }

    pub(crate) fn parse_type(&mut self) -> Result<BaseType, String> {
        let mut base_type = self.parse_base_type()?;

        while self.peek().kind == TokenKind::Multiply || self.peek().kind == TokenKind::LBracket {
            if self.peek().kind == TokenKind::Multiply {
                self.advance();
                base_type = BaseType::Pointer(Box::new(base_type));
            } else if self.peek().kind == TokenKind::LBracket {
                if let Some(after_tok) = self.token_after_bracket() {
                    if matches!(after_tok.kind, TokenKind::Identifier(_)) {
                        return Err(format!(
                            "Syntax Error: Invalid array declaration syntax at line {}. In FastLang, declare arrays using 'Type name[]' or 'array<Type> name', not 'Type[] name'.",
                            after_tok.line
                        ));
                    }
                }
                if !self.is_array_type_bracket() {
                    break;
                }
                self.advance();
                let mut len = None;
                if self.peek().kind == TokenKind::Comma {
                    self.advance();
                }
                if self.peek().kind != TokenKind::RBracket {
                    len = Some(self.parse_expression()?);
                }
                self.consume(TokenKind::RBracket, "Expected ']' after array type")?;
                base_type = BaseType::Array {
                    base_type: Box::new(base_type),
                    size: Box::new(len),
                };
            }
        }

        Ok(base_type)
    }

    fn token_after_bracket(&self) -> Option<&crate::frontend::lexer::token::Token> {
        let mut offset = 1;
        let mut depth = 1;
        while let Some(tok) = self.tokens.get(self.current + offset) {
            match &tok.kind {
                TokenKind::LBracket => depth += 1,
                TokenKind::RBracket => {
                    depth -= 1;
                    if depth == 0 {
                        return self.tokens.get(self.current + offset + 1);
                    }
                }
                _ => {}
            }
            offset += 1;
        }
        None
    }

    fn is_array_type_bracket(&self) -> bool {
        let mut offset = 1;
        let mut depth = 1;
        let mut has_comma = false;
        while let Some(tok) = self.tokens.get(self.current + offset) {
            match &tok.kind {
                TokenKind::LBracket => {
                    depth += 1;
                }
                TokenKind::RBracket => {
                    depth -= 1;
                    if depth == 0 {
                        offset += 1;
                        break;
                    }
                }
                TokenKind::Comma if depth == 1 => {
                    has_comma = true;
                }
                TokenKind::SemiColon | TokenKind::Assign | TokenKind::Arrow if depth == 0 => {
                    break;
                }
                _ => {}
            }
            offset += 1;
        }
        if has_comma {
            return false;
        }
        if let Some(after_tok) = self.tokens.get(self.current + offset) {
            if after_tok.kind == TokenKind::Assign || after_tok.kind == TokenKind::Arrow {
                return false;
            }
        }
        true
    }

    pub(crate) fn parse_base_type(&mut self) -> Result<BaseType, String> {
        let kind = self.peek().kind.clone();

        match kind {
            // Primitives
            TokenKind::TypeInt(default_size) => {
                self.advance(); // consume 'int' / 'int32'
                if self.peek().kind == TokenKind::Less {
                    self.advance();
                    let size = match self.peek().kind {
                        TokenKind::Int(s) => {
                            self.advance();
                            s
                        }
                        TokenKind::UInt(s) => {
                            self.advance();
                            s as i128
                        }
                        _ => {
                            return Err("Syntax Error: Expected integer size for type int<size>"
                                .to_string());
                        }
                    };
                    self.consume(TokenKind::Greater, "Expected '>' after type size")?;
                    match size {
                        8 => Ok(BaseType::Int(Size::S8)),
                        16 => Ok(BaseType::Int(Size::S16)),
                        32 => Ok(BaseType::Int(Size::S32)),
                        64 => Ok(BaseType::Int(Size::S64)),
                        128 => Ok(BaseType::Int(Size::S128)),
                        _ => Err(format!(
                            "Syntax Error: Invalid size {} for int. Allowed: 8, 16, 32, 64, 128",
                            size
                        )),
                    }
                } else {
                    match default_size {
                        8 => Ok(BaseType::Int(Size::S8)),
                        16 => Ok(BaseType::Int(Size::S16)),
                        32 => Ok(BaseType::Int(Size::S32)),
                        64 => Ok(BaseType::Int(Size::S64)),
                        128 => Ok(BaseType::Int(Size::S128)),
                        _ => Ok(BaseType::Int(Size::S32)),
                    }
                }
            }
            TokenKind::TypeUInt(default_size) => {
                self.advance(); // consume 'uint' / 'uint32' / 'byte'
                if self.peek().kind == TokenKind::Less {
                    self.advance();
                    let size = match self.peek().kind {
                        TokenKind::Int(s) => {
                            self.advance();
                            s
                        }
                        TokenKind::UInt(s) => {
                            self.advance();
                            s as i128
                        }
                        _ => {
                            return Err("Syntax Error: Expected integer size for type uint<size>"
                                .to_string());
                        }
                    };
                    self.consume(TokenKind::Greater, "Expected '>' after type size")?;
                    match size {
                        8 => Ok(BaseType::UInt(Size::S8)),
                        16 => Ok(BaseType::UInt(Size::S16)),
                        32 => Ok(BaseType::UInt(Size::S32)),
                        64 => Ok(BaseType::UInt(Size::S64)),
                        128 => Ok(BaseType::UInt(Size::S128)),
                        _ => Err(format!(
                            "Syntax Error: Invalid size {} for uint. Allowed: 8, 16, 32, 64, 128",
                            size
                        )),
                    }
                } else {
                    match default_size {
                        8 => Ok(BaseType::UInt(Size::S8)),
                        16 => Ok(BaseType::UInt(Size::S16)),
                        32 => Ok(BaseType::UInt(Size::S32)),
                        64 => Ok(BaseType::UInt(Size::S64)),
                        128 => Ok(BaseType::UInt(Size::S128)),
                        _ => Ok(BaseType::UInt(Size::S32)),
                    }
                }
            }
            TokenKind::TypeUSize => {
                self.advance();
                Ok(BaseType::USize)
            }
            TokenKind::TypeISize => {
                self.advance();
                Ok(BaseType::ISize)
            }
            TokenKind::TypeFloat(default_size) => {
                self.advance(); // consume 'float' / 'float32'
                if self.peek().kind == TokenKind::Less {
                    self.advance();
                    let size = match self.peek().kind {
                        TokenKind::Int(s) => {
                            self.advance();
                            s
                        }
                        TokenKind::UInt(s) => {
                            self.advance();
                            s as i128
                        }
                        _ => {
                            return Err(
                                "Syntax Error: Expected size for type float<size>".to_string()
                            );
                        }
                    };
                    self.consume(TokenKind::Greater, "Expected '>' after type size")?;
                    match size {
                        32 => Ok(BaseType::Float(Size::S32)),
                        64 => Ok(BaseType::Float(Size::S64)),
                        128 => Ok(BaseType::Float(Size::S128)),
                        _ => Err(format!(
                            "Syntax Error: Invalid size {} for float. Allowed: 32, 64, 128",
                            size
                        )),
                    }
                } else {
                    match default_size {
                        32 => Ok(BaseType::Float(Size::S32)),
                        64 => Ok(BaseType::Float(Size::S64)),
                        128 => Ok(BaseType::Float(Size::S128)),
                        _ => Ok(BaseType::Float(Size::S32)),
                    }
                }
            }
            TokenKind::TypeBool => {
                self.advance();
                Ok(BaseType::Bool)
            }
            TokenKind::TypeChar => {
                self.advance();
                Ok(BaseType::Char)
            }
            TokenKind::TypeStr => {
                self.advance();
                Ok(BaseType::Str)
            }
            TokenKind::TypeVoid => {
                self.advance();
                Ok(BaseType::Void)
            }
            TokenKind::TypeName => {
                self.advance();
                let mut name_type = BaseType::Unknown;
                if self.peek().kind == TokenKind::Less {
                    self.advance();
                    let mut generics = Vec::new();
                    self.parse_generic_list(&mut generics)?;
                    self.consume(TokenKind::Greater, "Expected '>' after name type parameter")?;
                    name_type = if generics.len() == 1 {
                        generics.into_iter().next().unwrap()
                    } else {
                        BaseType::Generic(generics)
                    };
                }
                Ok(BaseType::Name(Box::new(name_type)))
            }
            TokenKind::TypeModify => {
                self.advance(); // modify
                let modify_inner = if self.peek().kind == TokenKind::Less {
                    self.advance(); // '<'
                    let mut generics = Vec::new();
                    self.parse_generic_list(&mut generics)?;
                    self.consume(TokenKind::Greater, "Expected '>' after modify types")?;
                    if generics.len() == 1 {
                        generics.into_iter().next().unwrap()
                    } else if generics.is_empty() {
                        BaseType::Unknown
                    } else {
                        BaseType::Generic(generics)
                    }
                } else {
                    BaseType::Unknown
                };
                Ok(BaseType::Modify(Box::new(modify_inner)))
            }
            TokenKind::TypeCopy => {
                self.advance(); // copy
                let copy_inner = if self.peek().kind == TokenKind::Less {
                    self.advance(); // '<'
                    let mut generics = Vec::new();
                    self.parse_generic_list(&mut generics)?;
                    self.consume(TokenKind::Greater, "Expected '>' after copy types")?;
                    if generics.len() == 1 {
                        generics.into_iter().next().unwrap()
                    } else if generics.is_empty() {
                        BaseType::Unknown
                    } else {
                        BaseType::Generic(generics)
                    }
                } else {
                    BaseType::Unknown
                };
                Ok(BaseType::Copy(Box::new(copy_inner)))
            }
            TokenKind::TypeType => {
                self.advance();
                Ok(BaseType::Type(Box::new(BaseType::Unknown)))
            }
            TokenKind::Flag => {
                self.advance();
                Ok(BaseType::Flag)
            }
            TokenKind::TypeMethod
            | TokenKind::TypeFn
            | TokenKind::Fn
            | TokenKind::TypeMicro
            | TokenKind::TypeLambda => {
                let kind_str = match &self.peek().kind {
                    TokenKind::TypeMethod => "method",
                    TokenKind::TypeMicro => "micro",
                    TokenKind::TypeLambda => "lambda",
                    _ => "fn",
                };
                self.advance();

                let mut name = None;
                if self.peek().kind == TokenKind::DoubleColon {
                    self.advance(); // '::'
                    if let TokenKind::Identifier(id) = &self.peek().kind {
                        name = Some(id.clone());
                        self.advance();
                    }
                }

                let mut params = Vec::new();
                let mut return_type = Box::new(BaseType::Unknown);

                if self.peek().kind == TokenKind::Less {
                    self.advance(); // consume '<'
                    if self.peek().kind == TokenKind::LParen {
                        self.advance(); // consume '('
                        while !self.is_at_end() && self.peek().kind != TokenKind::RParen {
                            if self.peek().kind == TokenKind::TypeVoid {
                                self.advance();
                            } else {
                                params.push(self.parse_type()?);
                            }
                            if self.peek().kind == TokenKind::Comma {
                                self.advance();
                            } else if self.peek().kind != TokenKind::RParen {
                                return Err(format!(
                                    "Expected ',' or ')' in {} parameter types",
                                    kind_str
                                ));
                            }
                        }
                        self.consume(
                            TokenKind::RParen,
                            &format!("Expected ')' after {} parameter types", kind_str),
                        )?;
                        if self.peek().kind == TokenKind::Comma
                            || self.peek().kind == TokenKind::Arrow
                        {
                            self.advance(); // consume ',' or '->'
                            if self.peek().kind != TokenKind::Greater {
                                return_type = Box::new(self.parse_type()?);
                            }
                        }
                    } else if self.peek().kind != TokenKind::Greater {
                        return_type = Box::new(self.parse_type()?);
                    }
                    self.consume(
                        TokenKind::Greater,
                        &format!("Expected '>' after {} type", kind_str),
                    )?;
                }

                match kind_str {
                    "method" => Ok(BaseType::Method {
                        name,
                        params,
                        return_type,
                        mode: ExecutionMode::Runtime,
                    }),
                    "micro" => Ok(BaseType::Micro {
                        name,
                        params,
                        return_type,
                        mode: ExecutionMode::Runtime,
                    }),
                    "lambda" => Ok(BaseType::Lambda {
                        name,
                        params,
                        return_type,
                    }),
                    _ => Ok(BaseType::Fn {
                        name,
                        params,
                        return_type,
                        mode: ExecutionMode::Runtime,
                    }),
                }
            }
            TokenKind::TypeBlock => {
                self.advance();
                let mut name = String::new();
                if self.peek().kind == TokenKind::DoubleColon {
                    self.advance(); // '::'
                    if let TokenKind::Identifier(id) = &self.peek().kind {
                        name = id.clone();
                        self.advance();
                    }
                }
                if self.peek().kind == TokenKind::Less {
                    self.advance(); // '<'
                    let _ = self.parse_type()?;
                    self.consume(TokenKind::Greater, "Expected '>' after block type")?;
                }
                Ok(BaseType::Block {
                    name,
                    fields: Box::new(std::collections::HashMap::new()),
                    methods: Box::new(std::collections::HashMap::new()),
                })
            }
            TokenKind::TypeClass
            | TokenKind::TypeStruct
            | TokenKind::TypeBluePrint
            | TokenKind::TypeEnum => {
                let token_kind = self.peek().kind.clone();
                self.advance();
                let mut obj_name = String::new();
                if self.peek().kind == TokenKind::DoubleColon {
                    self.advance(); // '::'
                    if let TokenKind::Identifier(id) = &self.peek().kind {
                        obj_name = id.clone();
                        self.advance();
                    }
                }
                let mut generics = Vec::new();
                if self.peek().kind == TokenKind::Less {
                    self.advance(); // '<'
                    self.parse_generic_list(&mut generics)?;
                    self.consume(
                        TokenKind::Greater,
                        "Expected '>' after generic type parameter",
                    )?;
                }
                match token_kind {
                    TokenKind::TypeClass => Ok(BaseType::Class {
                        name: obj_name,
                        fields: Box::new(std::collections::HashMap::new()),
                        methods: Box::new(std::collections::HashMap::new()),
                        constructor: None,
                        generics,
                    }),
                    TokenKind::TypeStruct => Ok(BaseType::Struct {
                        name: obj_name,
                        fields: Box::new(std::collections::HashMap::new()),
                        methods: Box::new(std::collections::HashMap::new()),
                        generics,
                    }),
                    TokenKind::TypeBluePrint => Ok(BaseType::Blueprint {
                        name: obj_name,
                        fields: Box::new(std::collections::HashMap::new()),
                        methods: Box::new(std::collections::HashMap::new()),
                        generics,
                    }),
                    _ => Ok(BaseType::Enum {
                        name: obj_name,
                        variants: Vec::new(),
                        methods: Box::new(std::collections::HashMap::new()),
                        generics,
                    }),
                }
            }
            TokenKind::Identifier(n) => {
                self.advance();
                if self.current_generics.contains(&n) {
                    return Ok(BaseType::GenericParam(n));
                }
                let mut generics = Vec::new();
                if self.peek().kind == TokenKind::Less {
                    self.advance(); // '<'
                    self.parse_generic_list(&mut generics)?;
                    self.consume(
                        TokenKind::Greater,
                        "Expected '>' after generic type parameter",
                    )?;
                }

                if let Some(meta) = self.metadata.get(&n).cloned() {
                    let fields = Box::new(meta.fields.clone());
                    let methods = Box::new(meta.methods.clone());

                    if meta.ty.as_str().starts_with("enum") {
                        Ok(BaseType::Enum {
                            name: n.clone(),
                            variants: meta.variants.unwrap_or(Vec::new()),
                            methods,
                            generics,
                        })
                    } else if meta.ty.as_str().starts_with("class") {
                        Ok(BaseType::Class {
                            name: n.clone(),
                            fields,
                            methods,
                            constructor: meta.constructor,
                            generics,
                        })
                    } else if meta.ty.as_str().starts_with("struct") {
                        Ok(BaseType::Struct {
                            name: n.clone(),
                            fields,
                            methods,
                            generics,
                        })
                    } else if meta.ty.as_str().starts_with("blueprint") {
                        Ok(BaseType::Blueprint {
                            name: n.clone(),
                            fields,
                            methods,
                            generics,
                        })
                    } else {
                        Ok(BaseType::Blueprint {
                            name: n.clone(),
                            fields,
                            methods,
                            generics,
                        })
                    }
                } else {
                    Ok(BaseType::Blueprint {
                        name: n.clone(),
                        fields: Box::new(std::collections::HashMap::new()),
                        methods: Box::new(std::collections::HashMap::new()),
                        generics,
                    })
                }
            }
            _ => Err(format!(
                "Syntax Error: Expected a type, found '{}'. at line {}, column {}",
                self.peek().kind.as_str(),
                self.peek().line,
                self.peek().column
            )),
        }
    }

    pub(crate) fn parse_expression(&mut self) -> Result<Expr, String> {
        self.parse_expr(0)
    }

    pub(crate) fn parse_expr(&mut self, min_bp: u8) -> Result<Expr, String> {
        let lhs = self.parse_prefix()?;
        self.parse_expr_with_lhs(lhs, min_bp)
    }

    pub(crate) fn parse_expr_with_lhs(
        &mut self,
        mut lhs: Expr,
        min_bp: u8,
    ) -> Result<Expr, String> {
        // --- Infix / Postfix
        loop {
            if let Some(postfix_bp) = self.postfix_binding_power() {
                if postfix_bp < min_bp {
                    break;
                }
                lhs = self.parse_postfix(lhs)?;

                continue;
            }

            if self.peek().kind == TokenKind::As {
                let as_bp = 14;
                if as_bp < min_bp {
                    break;
                }
                self.advance(); // consume 'as'
                let target_type = self.parse_type()?;
                lhs = Expr::Cast {
                    expr: Box::new(lhs),
                    target_type,
                };
                continue;
            }

            if let Some((left_bp, right_bp)) = self.infix_binding_power() {
                if left_bp < min_bp {
                    break;
                }
                let op_str = self.current_op_str();
                self.advance();
                let rhs = self.parse_expr(right_bp)?;

                lhs = Expr::BinaryOp {
                    left: Box::new(lhs),
                    operator: op_str,
                    right: Box::new(rhs),
                };
                continue;
            }

            if let Expr::LiteralString(ref mut s1) = lhs {
                if let TokenKind::String(ref s2) = self.peek().kind {
                    s1.push_str(s2);
                    self.advance();
                    continue;
                }
            }

            break;
        }

        Ok(lhs)
    }

    pub(crate) fn parse_prefix(&mut self) -> Result<Expr, String> {
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
            TokenKind::UInt(v) => {
                let val = *v;
                self.advance();

                Ok(Expr::LiteralUInt(val))
            }
            TokenKind::Float(v) => {
                let val = *v;
                self.advance();

                Ok(Expr::LiteralFloat(val))
            }
            TokenKind::String(s) => {
                let val = s.clone();
                self.advance();

                Ok(Expr::LiteralString(val.to_string()))
            }
            TokenKind::Char(c) => {
                let val = *c;
                self.advance();

                Ok(Expr::LiteralChar(val))
            }
            TokenKind::Bool(b) => {
                let val = *b;
                self.advance();

                Ok(Expr::LiteralBool(val))
            }
            TokenKind::TypeVoid => {
                self.advance();
                Ok(Expr::LiteralVoid)
            }
            TokenKind::Undefined => {
                self.advance();
                Ok(Expr::LiteralUndefined)
            }
            TokenKind::TypeInt(_)
            | TokenKind::TypeUInt(_)
            | TokenKind::TypeUSize
            | TokenKind::TypeISize
            | TokenKind::TypeFloat(_)
            | TokenKind::TypeChar
            | TokenKind::TypeBool
            | TokenKind::TypeType => {
                let t = self.parse_type()?;
                Ok(Expr::Identifier(t.as_str()))
            }

            TokenKind::Fn => {
                self.advance(); // consume 'fn'
                                // Optional name or '_' (e.g. fn _(...) or fn(...))
                if let TokenKind::Identifier(ref name) = self.peek().kind {
                    if name == "_"
                        || self
                            .peek_ahead(1)
                            .map(|t| t.kind == TokenKind::LParen)
                            .unwrap_or(false)
                    {
                        self.advance();
                    }
                } else if self.peek().kind == TokenKind::Underscore {
                    self.advance();
                }

                self.consume(TokenKind::LParen, "Expected '(' for lambda parameter list")?;
                let mut params = Vec::new();
                if self.peek().kind != TokenKind::RParen {
                    loop {
                        let p = self.parse_single_param()?;
                        params.push(p);
                        if self.peek().kind == TokenKind::Comma {
                            self.advance();
                        } else {
                            break;
                        }
                    }
                }
                self.consume(
                    TokenKind::RParen,
                    "Expected ')' after lambda parameter list",
                )?;

                let mut return_type = None;
                if self.peek().kind == TokenKind::Arrow {
                    self.advance();
                    return_type = Some(self.parse_type()?);
                }

                self.consume(TokenKind::LBrace, "Expected '{' to open lambda body")?;
                let mut body = Vec::new();
                while !self.is_at_end() && self.peek().kind != TokenKind::RBrace {
                    if let Some(stmt) = self.parse_statement(ScopeType::Fn)? {
                        body.push(stmt);
                    }
                }
                self.consume(TokenKind::RBrace, "Expected '}' to close lambda body")?;

                Ok(Expr::Lambda {
                    params,
                    return_type,
                    body,
                })
            }

            // --- Identifier ---
            TokenKind::Identifier(name) => {
                let val: String = name.clone();
                self.advance();
                Ok(Expr::Identifier(val.to_string()))
            }
            TokenKind::LabelName(name) => {
                let val: String = name.clone();
                self.advance();
                Ok(Expr::Identifier(val))
            }
            TokenKind::ToString => {
                self.advance();
                Ok(Expr::Identifier("to_string".to_string()))
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

            // --- Prefix: ++expr ---
            TokenKind::PlusPlus => {
                self.advance();
                let operand = self.parse_expr(7)?;
                Ok(Expr::PrefixUpdate {
                    right: Box::new(operand),
                    operator: "++".to_string(),
                })
            }
            // --- Prefix: --expr ---
            TokenKind::MinusMinus => {
                self.advance();
                let operand = self.parse_expr(7)?;
                Ok(Expr::PrefixUpdate {
                    right: Box::new(operand),
                    operator: "--".to_string(),
                })
            }
            // --- Unary: &expr ---
            TokenKind::Ampersand => {
                self.advance();
                let operand = self.parse_expr(7)?;
                Ok(Expr::UnaryOp {
                    operator: "&".to_string(),
                    operand: Box::new(operand),
                })
            }
            TokenKind::Multiply => {
                self.advance();
                let operand = self.parse_expr(7)?;
                Ok(Expr::UnaryOp {
                    operator: "*".to_string(),
                    operand: Box::new(operand),
                })
            }
            TokenKind::TypeModify => {
                self.advance();
                let operand = self.parse_expr(7)?;
                Ok(Expr::UnaryOp {
                    operator: "modify".to_string(),
                    operand: Box::new(operand),
                })
            }
            TokenKind::TypeCopy => {
                self.advance();
                let operand = self.parse_expr(7)?;
                Ok(Expr::UnaryOp {
                    operator: "copy".to_string(),
                    operand: Box::new(operand),
                })
            }
            TokenKind::DotDotDot => {
                self.advance();
                let operand = self.parse_expr(7)?;
                Ok(Expr::Spread(Box::new(operand)))
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

                Ok(Expr::ArrayLiteral(elements))
            }

            // --- new T(...) or new T[...] ---
            // Handles:
            //   new SomeClass(args)      → Instantiate
            //   new int(32)[1,2,3,4,5]  → ArrayAllocate
            // --- new T(...) or new T[...] ---
            TokenKind::New => {
                self.advance(); // consume 'new'

                let type_node = self.parse_base_type()?;

                // Check for dynamic array allocation with length: new T[](length)
                if self.peek().kind == TokenKind::LBracket
                    && self
                        .peek_ahead(1)
                        .map(|t| t.kind == TokenKind::RBracket)
                        .unwrap_or(false)
                {
                    self.advance(); // consume '['
                    self.advance(); // consume ']'
                    self.consume(
                        TokenKind::LParen,
                        "Expected '(' after '[]' for array length",
                    )?;
                    let len_expr = self.parse_expr(0)?;
                    self.consume(TokenKind::RParen, "Expected ')' after array length")?;
                    return Ok(Expr::New {
                        type_node: BaseType::Array {
                            base_type: Box::new(type_node),
                            size: Box::new(Some(len_expr)),
                        },
                        target: Box::new(Expr::Default(None)),
                    });
                }

                let target = match self.peek().kind {
                    TokenKind::LBracket => {
                        self.advance(); // consume '['
                        let mut elements = Vec::new();
                        if self.peek().kind != TokenKind::RBracket {
                            elements.push(self.parse_expr(0)?);
                            while self.peek().kind == TokenKind::Comma {
                                self.advance(); // consume ','
                                elements.push(self.parse_expr(0)?);
                            }
                        }
                        self.consume(
                            TokenKind::RBracket,
                            "Expected ']' to close array allocation",
                        )?;
                        Expr::ArrayLiteral(elements)
                    }
                    TokenKind::LParen => {
                        self.advance(); // consume '('
                        let mut args = Vec::new();
                        if self.peek().kind != TokenKind::RParen {
                            args.push(self.parse_expr(0)?);
                            while self.peek().kind == TokenKind::Comma {
                                self.advance(); // consume ','
                                args.push(self.parse_expr(0)?);
                            }
                        }
                        self.consume(
                            TokenKind::RParen,
                            "Expected ')' after constructor arguments",
                        )?;

                        if args.is_empty() {
                            Expr::Default(None)
                        } else {
                            Expr::Instantiate {
                                target: Box::new(Expr::Identifier(type_node.as_str())),
                                args,
                            }
                        }
                    }
                    _ => Expr::Default(None),
                };
                Ok(Expr::New {
                    type_node,
                    target: Box::new(target),
                })
            }

            // --- Grouped: (expr) or Tuple/Stack: (expr, expr, ...) or Default: () or Lambda: (params) -> { ... } or () -> expr ---
            TokenKind::LParen => {
                if self.is_lambda_ahead() {
                    self.parse_lambda_expr()
                } else {
                    self.advance(); // (
                    if self.peek().kind == TokenKind::RParen {
                        self.advance();
                        // () represents default / unit / void
                        return Ok(Expr::LiteralVoid);
                    }
                    let first = self.parse_expr(0)?;
                    if self.peek().kind == TokenKind::Comma {
                        let mut elements = vec![first];
                        while self.peek().kind == TokenKind::Comma {
                            self.advance(); // consume ','
                            if self.peek().kind == TokenKind::RParen {
                                break;
                            }
                            elements.push(self.parse_expr(0)?);
                        }
                        self.consume(
                            TokenKind::RParen,
                            "Expected ')' to close tuple/stack expression",
                        )?;
                        Ok(Expr::ArrayLiteral(elements))
                    } else {
                        self.consume(
                            TokenKind::RParen,
                            "Expected ')' to close grouped expression",
                        )?;
                        Ok(first)
                    }
                }
            }

            // --- Object Literals: { stmt; stmt; } ---
            TokenKind::LBrace => {
                self.advance(); //{
                let stmts = self.parse_block("object".to_string())?;
                self.consume(TokenKind::RBrace, "Expected '}' after object literal")?;
                Ok(Expr::ObjectLiteral(stmts))
            }

            // --- Pipe Lambdas: |x: int, y: int| x + y ---
            TokenKind::Pipe => {
                self.advance(); // consume opening '|'
                let mut params = Vec::new();
                while !self.is_at_end() && self.peek().kind != TokenKind::Pipe {
                    let is_untyped = if let TokenKind::Identifier(_) = &self.peek().kind {
                        self.peek_ahead(1)
                            .map(|t| t.kind == TokenKind::Comma || t.kind == TokenKind::Pipe)
                            .unwrap_or(false)
                    } else {
                        false
                    };

                    let p = if is_untyped {
                        let name = self.get_identifier("Expected parameter name in lambda")?;
                        Param {
                            name,
                            type_node: BaseType::Unknown,
                            default_value: None,
                            is_variadic: false,
                        }
                    } else {
                        self.parse_single_param()?
                    };

                    params.push(p);
                    if self.peek().kind == TokenKind::Comma {
                        self.advance();
                    } else if self.peek().kind != TokenKind::Pipe {
                        return Err("Expected ',' or '|' in lambda parameters".to_string());
                    }
                }
                self.consume(TokenKind::Pipe, "Expected '|' to close lambda parameters")?;

                let body_expr = self.parse_expression()?;
                Ok(Expr::Lambda {
                    params,
                    return_type: None,
                    body: vec![Stmt::ReturnStmt(Some(body_expr))],
                })
            }
            TokenKind::Underscore => {
                self.advance();
                Ok(Expr::Identifier("_".to_string()))
            }
            TokenKind::Handle => {
                self.advance();
                Ok(Expr::Identifier("handle".to_string()))
            }
            // --- Keywords used as identifier expressions (e.g. `flag && check`) ---
            // !!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!
            other => {
                //debug
                println!(
                    "DEBUG: Unexpected token '{:?}' in expression at line {}, col {}",
                    other,
                    self.peek().line,
                    self.peek().column
                );
                return Err(format!(
                    "Syntax Error: Unexpected token '{:?}' in expression at line {}, col {}",
                    other,
                    self.peek().line,
                    self.peek().column
                ));
            }
        }
    }

    pub(crate) fn is_struct_instantiation_ahead(&self) -> bool {
        if self.peek().kind != TokenKind::LBrace {
            return false;
        }
        let idx = self.current + 1;
        if idx < self.tokens.len() {
            if self.tokens[idx].kind == TokenKind::RBrace
                || self.tokens[idx].kind == TokenKind::DotDot
            {
                return true;
            }
            if matches!(&self.tokens[idx].kind, TokenKind::Identifier(_)) {
                if let Some(next_tok) = self.tokens.get(idx + 1) {
                    if matches!(
                        next_tok.kind,
                        TokenKind::Assign | TokenKind::Colon | TokenKind::Comma | TokenKind::RBrace
                    ) {
                        return true;
                    }
                }
            }
        }
        false
    }

    pub(crate) fn is_generic_call_ahead(&self) -> bool {
        let mut offset = 1;
        let mut depth = 1;
        while let Some(tok) = self.tokens.get(self.current + offset) {
            match &tok.kind {
                TokenKind::Less => depth += 1,
                TokenKind::Greater => {
                    depth -= 1;
                    if depth == 0 {
                        if let Some(next_tok) = self.tokens.get(self.current + offset + 1) {
                            return matches!(
                                next_tok.kind,
                                TokenKind::LParen | TokenKind::Dot | TokenKind::DoubleColon
                            );
                        }
                        return false;
                    }
                }
                TokenKind::SemiColon
                | TokenKind::LBrace
                | TokenKind::RBrace
                | TokenKind::Assign
                | TokenKind::Arrow => return false,
                _ => {}
            }
            offset += 1;
        }
        false
    }

    pub(crate) fn postfix_binding_power(&self) -> Option<u8> {
        match &self.peek().kind {
            TokenKind::Dot => Some(20),         // property access: obj.field
            TokenKind::DoubleColon => Some(20), // static access: Class::field
            TokenKind::LParen => Some(20),      // function call:   foo(...)
            TokenKind::Less => {
                if self.is_generic_call_ahead() {
                    Some(20) // generic function call or type member access: foo<T>(...) or foo<T>.bar
                } else {
                    None
                }
            }
            TokenKind::LBracket => Some(20),    // array indexing: arr[0]
            TokenKind::LBrace => {
                if self.is_struct_instantiation_ahead() {
                    Some(20) // object instantiation: TypeName { ... }
                } else {
                    None
                }
            }
            TokenKind::PlusPlus => Some(21),   // postfix ++
            TokenKind::MinusMinus => Some(21), // postfix --
            _ => None,
        }
    }

    pub(crate) fn parse_postfix(&mut self, lhs: Expr) -> Result<Expr, String> {
        match &self.peek().kind.clone() {
            // --- Generic Function Call or Type Access: lhs<T1, T2>(...) or lhs<T1, T2>.prop ---
            TokenKind::Less => {
                self.advance(); // consume '<'
                let mut generics = Vec::new();
                self.parse_generic_list(&mut generics)?;
                self.consume(TokenKind::Greater, "Expected '>' after generic type arguments")?;
                if self.peek().kind == TokenKind::LParen {
                    self.advance(); // consume '('
                    let mut args = Vec::new();
                    if self.peek().kind != TokenKind::RParen {
                        args.push(self.parse_expr(0)?);
                        while self.peek().kind == TokenKind::Comma {
                            self.advance();
                            args.push(self.parse_expr(0)?);
                        }
                    }
                    self.consume(
                        TokenKind::RParen,
                        "Expected ')' to close function call argument list",
                    )?;
                    Ok(Expr::Call {
                        callee: Box::new(lhs),
                        generics,
                        args,
                    })
                } else {
                    let gen_str = generics
                        .iter()
                        .map(|g| g.as_str())
                        .collect::<Vec<_>>()
                        .join(", ");
                    match lhs {
                        Expr::Identifier(name) => {
                            Ok(Expr::Identifier(format!("{}<{}>", name, gen_str)))
                        }
                        Expr::NamespaceAccess {
                            namespace,
                            property,
                        } => {
                            if let Expr::Identifier(prop_name) = *property {
                                Ok(Expr::NamespaceAccess {
                                    namespace,
                                    property: Box::new(Expr::Identifier(format!(
                                        "{}<{}>",
                                        prop_name, gen_str
                                    ))),
                                })
                            } else {
                                Err("Syntax Error: Unexpected property expression before generic arguments".to_string())
                            }
                        }
                        _ => Err("Syntax Error: Generic type arguments can only follow an identifier or namespace access".to_string()),
                    }
                }
            }

            // --- Property Access: lhs.identifier ---
            TokenKind::Dot => {
                self.advance();

                let prop = if let TokenKind::Identifier(name) = &self.peek().kind.clone() {
                    let p = name.to_string();
                    self.advance();
                    p
                } else {
                    let kw_name = self.peek().kind.clone().as_str().to_string();
                    self.advance();
                    kw_name
                };
                Ok(Expr::PropertyAccess {
                    object: Box::new(lhs),
                    property: prop,
                })
            }

            // --- Namespace Access: lhs::identifier ---
            TokenKind::DoubleColon => {
                self.advance();
                let prop = self.get_handle_identifier("Expected member name after '::'")?;

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
                self.advance(); // consume '('
                let mut args = Vec::new();

                // Parse arguments if argument list is not empty
                if self.peek().kind != TokenKind::RParen {
                    args.push(self.parse_expr(0)?);
                    while self.peek().kind == TokenKind::Comma {
                        self.advance(); // consume ','
                        args.push(self.parse_expr(0)?);
                    }
                }

                self.consume(
                    TokenKind::RParen,
                    "Expected ')' to close function call argument list",
                )?;
                Ok(Expr::Call {
                    callee: Box::new(lhs),
                    generics: Vec::new(),
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

            // --- Array Indexing: lhs[index] or lhs[i, j] ---
            TokenKind::LBracket => {
                self.advance();
                let mut indices = Vec::new();
                if self.peek().kind != TokenKind::RBracket {
                    indices.push(self.parse_expr(0)?);
                    while self.peek().kind == TokenKind::Comma {
                        self.advance();
                        indices.push(self.parse_expr(0)?);
                    }
                }
                self.consume(TokenKind::RBracket, "Expected ']' after array index")?;
                Ok(Expr::IndexAccess {
                    object: Box::new(lhs),
                    indices,
                })
            }

            // --- TypeName { ... } instantiation ---
            TokenKind::LBrace => {
                self.advance();
                let stmts = self.parse_block("object".to_string())?;
                self.consume(TokenKind::RBrace, "Expected '}' after object literal")?;
                Ok(Expr::Instantiate {
                    target: Box::new(lhs),
                    args: vec![Expr::ObjectLiteral(stmts)],
                })
            }

            other => Err(format!(
                "Internal error: parse_postfix called with non-postfix token '{:?}'",
                other
            )),
        }
    }

    pub(crate) fn infix_binding_power(&self) -> Option<(u8, u8)> {
        match &self.peek().kind {
            TokenKind::Pipe => Some((1, 2)),
            TokenKind::Or => Some((1, 2)),  // left-associative
            TokenKind::And => Some((3, 4)), // left-associative
            TokenKind::Eq => Some((5, 6)),  // left-associative
            TokenKind::NotEq => Some((5, 6)),
            TokenKind::Less => Some((7, 8)),
            TokenKind::Greater => Some((7, 8)),
            TokenKind::LessEq => Some((7, 8)),
            TokenKind::GreaterEq => Some((7, 8)),
            TokenKind::Plus => Some((9, 10)), // left-associative
            TokenKind::Minus => Some((9, 10)),
            TokenKind::Multiply => Some((11, 12)),
            TokenKind::Divide => Some((11, 12)),
            TokenKind::Mod => Some((11, 12)),
            _ => None,
        }
    }
    pub(crate) fn current_op_str(&self) -> String {
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
            TokenKind::GreaterEq => ">=".to_string(),
            TokenKind::LessEq => "<=".to_string(),
            TokenKind::And => "&&".to_string(),
            TokenKind::Or => "||".to_string(),
            TokenKind::Pipe => "|".to_string(),
            other => format!("{:?}", other),
        }
    }

    pub(crate) fn is_lambda_ahead(&self) -> bool {
        let mut offset = 0;
        let mut depth = 0;
        while let Some(tok) = self.tokens.get(self.current + offset) {
            match &tok.kind {
                TokenKind::LParen => {
                    depth += 1;
                }
                TokenKind::RParen => {
                    depth -= 1;
                    if depth == 0 {
                        if let Some(next_tok) = self.tokens.get(self.current + offset + 1) {
                            return next_tok.kind == TokenKind::Arrow;
                        }
                        return false;
                    }
                }
                TokenKind::SemiColon if depth == 0 => {
                    return false;
                }
                _ => {}
            }
            offset += 1;
        }
        false
    }

    fn parse_lambda_expr(&mut self) -> Result<Expr, String> {
        self.consume(TokenKind::LParen, "Expected '(' at start of lambda")?;
        let mut params = Vec::new();
        while !self.is_at_end() && self.peek().kind != TokenKind::RParen {
            let p = self.parse_single_param()?;
            params.push(p);
            if self.peek().kind == TokenKind::Comma {
                self.advance();
            } else if self.peek().kind != TokenKind::RParen {
                return Err("Expected ',' or ')' in lambda parameters".to_string());
            }
        }
        self.consume(TokenKind::RParen, "Expected ')' after lambda parameters")?;
        self.consume(TokenKind::Arrow, "Expected '->' after lambda parameters")?;

        if self.peek().kind == TokenKind::LBrace {
            self.advance();
            let body = self.parse_block("lambda".to_string())?;
            self.consume(TokenKind::RBrace, "Expected '}' to close lambda body")?;
            Ok(Expr::Lambda {
                params,
                return_type: None,
                body,
            })
        } else {
            let expr = self.parse_expression()?;
            Ok(Expr::Lambda {
                params,
                return_type: None,
                body: vec![Stmt::ExpressionStmt(expr)],
            })
        }
    }
}
