use crate::frontend::lexer::token::TokenKind;
use crate::frontend::parser::ast::*;
use crate::frontend::parser::parser::Parser;
impl Parser {
    pub(crate) fn is_type_token(&self, kind: &TokenKind) -> bool {
        match kind {
            // Primitives — always valid as types
            TokenKind::TypeInt
            | TokenKind::TypeFloat
            | TokenKind::TypeBool
            | TokenKind::TypeChar
            | TokenKind::TypeLength
            | TokenKind::TypeType
            | TokenKind::TypeData
            | TokenKind::TypeVoid => true,
            // `scope` is the general type for any scope value
            TokenKind::Scope | TokenKind::TypeName => true,
            // Custom keywords registered dynamically via `keyword -> "...";` in scope bodies
            t => matches!(t, TokenKind::Identifier(n) if self.metadata.contains_key(n) || true), // todo: remove true
        }
    }

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
        self.consume(TokenKind::Greater, "Expected '>' after type parameters")?;
        Ok(())
    }

    pub(crate) fn parse_type(&mut self) -> Result<BaseType, String> {
        let kind = self.peek().kind.clone();
        
        match kind {
            // Primitives
            TokenKind::TypeInt => {
                self.advance(); // consume 'int'
                if self.peek().kind == TokenKind::LParen {
                    self.advance();
                    let size = if let TokenKind::Int(s) = self.peek().kind {
                        self.advance();
                        s
                    } else {
                        return Err("Syntax Error: Expected integer size for type int".to_string());
                    };
                    self.consume(TokenKind::RParen, "Expected ')' after type size")?;
                    
                    match size {
                        8 => Ok(BaseType::Int8),
                        16 => Ok(BaseType::Int16),
                        32 => Ok(BaseType::Int32),
                        64 => Ok(BaseType::Int64),
                        128 => Ok(BaseType::Int128),
                        _ => Err(format!("Syntax Error: Invalid size {} for int. Allowed: 8, 16, 32, 64, 128", size)),
                    }
                } else {
                    Ok(BaseType::Int32) // Default to Int32
                }
            },
            TokenKind::TypeFloat => {
                self.advance(); // consume 'float'
                if self.peek().kind == TokenKind::LParen {
                    self.advance();
                    let size = if let TokenKind::Int(s) = self.peek().kind {
                        self.advance();
                        s
                    } else {
                        return Err("Syntax Error: Expected integer size for type float".to_string());
                    };
                    self.consume(TokenKind::RParen, "Expected ')' after type size")?;
                    
                    match size {
                        32 => Ok(BaseType::Float32),
                        64 => Ok(BaseType::Float64),
                        _ => Err(format!("Syntax Error: Invalid size {} for float. Allowed: 32, 64", size)),
                    }
                } else {
                    Ok(BaseType::Float32) // Default to Float32
                }
            },
            TokenKind::TypeBool => { self.advance(); Ok(BaseType::Bool) },
            TokenKind::TypeChar => { self.advance(); Ok(BaseType::Char) },
            TokenKind::TypeVoid => { self.advance(); Ok(BaseType::Void) },
            TokenKind::TypeError => { self.advance(); Ok(BaseType::Error) },
            TokenKind::TypeLength => { self.advance(); Ok(BaseType::Unknown) },
            TokenKind::TypeData => { self.advance(); Ok(BaseType::Unknown) },
            
            TokenKind::TypeName => {
                self.advance();
                Ok(BaseType::Name(Box::new(BaseType::Unknown)))
            },
            TokenKind::TypeType => {
                self.advance();
                Ok(BaseType::Type(Box::new(BaseType::Unknown)))
            },
            
            TokenKind::Identifier(n) => {
                self.advance();
                
                // Parse generics if any
                let mut generics = Vec::new();
                if self.peek().kind == TokenKind::Less {
                    self.advance(); // '<'
                    self.parse_generic_list(&mut generics)?;
                }
                
                // Lookup in metadata
                if let Some(meta) = self.metadata.get(&n).cloned() {
                    let fields = Box::new(meta.fields.clone());
                    let methods = Box::new(meta.methods.clone());
                    
                    if meta.is_enum {
                        Ok(BaseType::Enum {
                            name: n.clone(),
                            variants: meta.variants.unwrap_or(Vec::new()),
                            methods,
                            generics,
                        })
                    } else {
                        if meta.constructor.is_some() {
                            Ok(BaseType::Class {
                                name: n.clone(),
                                fields,
                                methods,
                                constructor: meta.constructor,
                                generics,
                            })
                        } else {
                            Ok(BaseType::Custom {
                                name: n.clone(),
                                fields,
                                methods,
                                generics,
                                params: meta.params,
                            })
                        }
                    }
                } else {
                    // Unknown custom type at parse time
                    Ok(BaseType::Custom {
                        name: n.clone(),
                        fields: Box::new(std::collections::HashMap::new()),
                        methods: Box::new(std::collections::HashMap::new()),
                        generics,
                        params: Vec::new(),
                    })
                }
            },
            
            _ => {
                Err(format!(
                    "Syntax Error: Expected a type, found '{}'. at line {}, column {}",
                    kind.as_str(),
                    self.peek().line,
                    self.peek().column
                ))
            }
        }
    }

    /// Like `parse_type` but also handles a trailing `*` to produce `BaseType::Pointer(T)`.
    /// Used when parsing declarations like `int(32)* x` or `float(64)* arr[N]`.
    pub(crate) fn parse_type_with_pointer(&mut self) -> Result<(BaseType, bool), String> {
        let base = self.parse_type()?;
        if self.peek().kind == TokenKind::Multiply {
            self.advance(); // consume '*'
            Ok((BaseType::Pointer(Box::new(base)), true))
        } else {
            Ok((base, false))
        }
    }
    pub(crate) fn parse_expression(&mut self) -> Result<Expr, String> {
        self.parse_expr(0)
    }

    pub(crate) fn parse_expr(&mut self, min_bp: u8) -> Result<Expr, String> {
        let mut lhs = self.parse_prefix()?;

        // --- Infix / Postfix: استمر طالما في operators بقوة كافية ---
        loop {
            // نقطة الوصول (postfix): . و ()
            if let Some(postfix_bp) = self.postfix_binding_power() {
                if postfix_bp < min_bp {
                    break;
                }
                lhs = self.parse_postfix(lhs)?;

                continue;
            }

            // operator وسطي (infix): +, -, *, etc.
            if let Some((left_bp, right_bp)) = self.infix_binding_power() {
                if left_bp < min_bp {
                    break;
                }
                let op_str = self.current_op_str();
                self.advance(); // نتخطى الـ operator
                let rhs = self.parse_expr(right_bp)?;

                lhs = Expr::BinaryOp {
                    left: Box::new(lhs),
                    operator: op_str,
                    right: Box::new(rhs),
                };
                continue;
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

            // --- Identifier ---
            TokenKind::Identifier(name) => {
                let val = name.clone();
                self.advance();

                Ok(Expr::Identifier(val.to_string()))
            }
            TokenKind::TypeOf => {
                self.advance();
                self.consume(TokenKind::LParen, "Expected '(' after typeof")?;
                let target = self.parse_expression()?;
                self.consume(TokenKind::RParen, "Expected ')' after typeof target")?;
                Ok(Expr::TypeOf {
                    target: Box::new(target),
                })
            }
            TokenKind::SizeOf => {
                self.advance();
                self.consume(TokenKind::LParen, "Expected '(' after sizeof")?;
                let target = self.parse_expression()?;
                self.consume(TokenKind::RParen, "Expected ')' after sizeof target")?;
                Ok(Expr::SizeOf {
                    target: Box::new(target),
                })
            }
            TokenKind::Log => {
                self.advance();
                Ok(Expr::Identifier("log".to_string()))
            }
            TokenKind::ToString => {
                self.advance();
                Ok(Expr::Identifier("to_string".to_string()))
            }
            TokenKind::TypeError => {
                self.advance();
                Ok(Expr::Identifier("error".to_string()))
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
                let operand = self.parse_prefix()?;
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
            TokenKind::New => {
                self.advance(); // consume 'new'

                // Check if the next token is a primitive type keyword → ArrayAllocate
                let is_prim_type = matches!(
                    self.peek().kind,
                    TokenKind::TypeInt
                        | TokenKind::TypeFloat
                        | TokenKind::TypeChar
                        | TokenKind::TypeBool
                        | TokenKind::TypeVoid
                );

                if is_prim_type {
                    // Parse the element type (e.g. int(32))
                    let type_node = self.parse_type()?;

                    // Expect '[' for array allocation
                    if self.peek().kind == TokenKind::LBracket {
                        self.advance(); // '['
                        // elements / initializer list
                        let mut elements = Vec::new();
                        if self.peek().kind != TokenKind::RBracket {
                            elements.push(self.parse_expr(0)?);
                            while self.peek().kind == TokenKind::Comma {
                                self.advance();
                                elements.push(self.parse_expr(0)?);
                            }
                        }
                        self.consume(TokenKind::RBracket, "Expected ']' to close array allocation")?;

                        let length = Expr::LiteralInt(elements.len() as i64);
                        let init   = Expr::ArrayLiteral(elements);
                        return Ok(Expr::ArrayAllocate {
                            type_node,
                            size: Box::new(length.clone()),
                            length: Some(Box::new(init)),
                        });
                    } else {
                        // new int(32)(args)?  — unusual but handle gracefully
                        return Ok(Expr::ArrayAllocate {
                            type_node,
                            size: Box::new(Expr::LiteralInt(0)),
                            length: None,
                        });
                    }
                }

                // Otherwise: new SomeIdentifier(args)
                let target = self.parse_prefix()?;
                let mut args = Vec::new();
                if self.peek().kind == TokenKind::LParen {
                    self.advance();
                    if self.peek().kind != TokenKind::RParen {
                        loop {
                            args.push(self.parse_expression()?);
                            if self.peek().kind == TokenKind::Comma {
                                self.advance();
                            } else {
                                break;
                            }
                        }
                    }
                    self.consume(TokenKind::RParen, "Expected ')' after new arguments")?;
                }
                Ok(Expr::Instantiate {
                    target: Box::new(target),
                    args,
                })
            }

            // --- Copy: copy Target ---
            TokenKind::Copy => {
                self.advance();
                let target = self.parse_expr(9)?;
                Ok(Expr::Copy {
                    target: Box::new(target),
                })
            }

            // --- Modify: modify Target ---
            TokenKind::Modify => {
                self.advance();
                let target = self.parse_expr(9)?;
                Ok(Expr::Modify {
                    target: Box::new(target),
                })
            }

            // --- Grouped: (expr) ---
            TokenKind::LParen => {
                self.advance(); // نتخطى '('
                let inner = self.parse_expr(0)?;
                self.consume(
                    TokenKind::RParen,
                    "Expected ')' to close grouped expression",
                )?;
                Ok(inner)
            }

            // --- Object Literals: { stmt; stmt; } ---
            TokenKind::LBrace => {
                self.advance(); // نتخطى '{'
                let stmts = self.parse_block("object".to_string())?;
                self.consume(TokenKind::RBrace, "Expected '}' after object literal")?;
                Ok(Expr::ObjectLiteral(stmts))
            }

            // --- Keywords used as identifier expressions (e.g. `flag && check`) ---
            other => {
                //debug
                print!("DEBUG: Unexpected token '{}' in expression", other.as_str());
                return Err(format!(
                    "Syntax Error: Unexpected token '{}' in expression",
                    other.as_str()
                ));
            }
        }
    }

    /// يرجع الـ left binding power للـ postfix operators (. و ())
    /// None لو اللي قدامنا مش postfix operator.
    pub(crate) fn postfix_binding_power(&self) -> Option<u8> {
        match &self.peek().kind {
            TokenKind::Dot => Some(20),         // property access: obj.field
            TokenKind::DoubleColon => Some(20), // static access: Class::field
            TokenKind::LParen => Some(20),      // function call:   foo(...)
            TokenKind::LBracket => Some(20),    // array indexing: arr[0]
            TokenKind::PlusPlus => Some(21),    // postfix ++
            TokenKind::MinusMinus => Some(21),  // postfix --
            _ => None,
        }
    }

    /// يقرأ postfix operation على الـ lhs اللي اتبنى قبل كده.
    pub(crate) fn parse_postfix(&mut self, lhs: Expr) -> Result<Expr, String> {
        match &self.peek().kind.clone() {
            // --- Property Access: lhs.identifier ---
            TokenKind::Dot => {
                self.advance();

                let mut prop = String::new();
                if let TokenKind::Identifier(name) = &self.peek().kind.clone() {
                    prop = name.to_string();
                    self.advance();
                } else {
                    let kw_name = self.peek().kind.clone().as_str().to_string();
                    prop = kw_name.to_string();
                    self.advance();
                }
                Ok(Expr::PropertyAccess {
                    object: Box::new(lhs),
                    property: prop,
                })
            }

            // --- Namespace Access: lhs::identifier ---
            TokenKind::DoubleColon => {
                //todo : update check if the visibility is static in analyzer at least
                self.advance();
                let prop = if let TokenKind::Identifier(name) = &self.peek().kind.clone() {
                    let n = name.clone();
                    self.advance();
                    n
                } else {
                    return Err(format!(
                        "Syntax Error: Expected name after '::' at line {}, column {}",
                        self.peek().line,
                        self.peek().column
                    ));
                };

                let namespace = if let Expr::Identifier(n) = lhs {
                    n
                } else {
                    return Err(
                        "Syntax Error: Expected namespace identifier before '::'".to_string()
                    );
                };

                Ok(Expr::NamespaceAccess {
                    namespace,
                    property: Box::new(Expr::Identifier(prop.to_string())),
                })
            }

            // --- Function Call: lhs(arg1, arg2, ...) ---
            TokenKind::LParen => {
                self.advance(); // نتخطى '('
                let mut args = Vec::new();

                // اقرأ الـ arguments لو مش قائمة فاضية
                if self.peek().kind != TokenKind::RParen {
                    args.push(self.parse_expr(0)?);
                    while self.peek().kind == TokenKind::Comma {
                        self.advance(); // نتخطى ','
                        args.push(self.parse_expr(0)?);
                    }
                }

                self.consume(
                    TokenKind::RParen,
                    "Expected ')' to close function call argument list",
                )?;
                Ok(Expr::Call {
                    callee: Box::new(lhs),
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

            // --- Array Indexing: lhs[index] ---
            TokenKind::LBracket => {
                self.advance();
                let index = self.parse_expr(0)?;
                self.consume(TokenKind::RBracket, "Expected ']' after array index")?;
                Ok(Expr::IndexAccess {
                    object: Box::new(lhs),
                    index: Box::new(index),
                })
            }

            other => Err(format!(
                "Internal error: parse_postfix called with non-postfix token '{:?}'",
                other
            )),
        }
    }

    /// يرجع (left_bp, right_bp) للـ infix operators.
    /// left_bp   = الـ binding power للجانب الأيسر (تحديد ما إذا كان الـ operator يسرق الـ lhs).
    /// right_bp  = الـ binding power اللي بنمرره للـ parse_expr الـ recursive للجانب الأيمن.
    /// None لو اللي قدامنا مش infix operator.
    pub(crate) fn infix_binding_power(&self) -> Option<(u8, u8)> {
        match &self.peek().kind {
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
            other => format!("{:?}", other),
        }
    }
}
