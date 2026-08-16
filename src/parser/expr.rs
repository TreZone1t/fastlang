use crate::lexer::token::TokenKind;
use crate::parser::ast::*;
use crate::parser::parser::Parser;
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
            TokenKind::TypeScope | TokenKind::TypeName => true,
            // Custom keywords registered dynamically via `keyword -> "...";` in scope bodies
            t => matches!(t, TokenKind::Identifier(n) if self.metadata.contains_key(n) || true), // todo: remove true
        }
    }

    pub(crate) fn parse_generic_list(
        &mut self,
        generics: &mut Vec<TypeNode>,
        size: &mut Option<i64>,
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

    pub(crate) fn parse_type(&mut self) -> Result<TypeNode, String> {
        let result: BaseType = match &self.peek().kind {
            // Primitives
            TokenKind::TypeInt => BaseType::Int,
            TokenKind::TypeFloat => BaseType::Float,
            TokenKind::TypeBool => BaseType::Bool,
            TokenKind::TypeChar => BaseType::Char,
            // Magic value types
            TokenKind::TypeName => {
                //todo : improve this
                BaseType::Name(Box::new(BaseType::Unknown))
            }
            TokenKind::TypeLength => BaseType::Unknown,

            TokenKind::TypeData => BaseType::Unknown,

            TokenKind::TypeType => BaseType::Type(Box::new(BaseType::Unknown)),
            TokenKind::TypeVoid => BaseType::Void,
            TokenKind::TypeError => BaseType::Error,

            TokenKind::Identifier(n) => {
                BaseType::Custom(n.to_string())
            }
            _ => {
                return Err(format!(
                    "Syntax Error: Expected a type, found '{}'. at line {}, column {}",
                    self.peek().kind.as_str(),
                    self.peek().line,
                    self.peek().column
                ))
            }
        };
        self.advance();

        let mut size = None;
        let mut generics = Vec::new();

        if result == BaseType::Int || result == BaseType::Float {
            if self.peek().kind == TokenKind::LParen {
                self.advance();
                if let TokenKind::Int(s) = self.peek().kind {
                    if result == BaseType::Int && ![8, 16, 32, 64, 128].contains(&s) {
                        return Err(format!(
                            "Syntax Error: Invalid size {} for int. Allowed: 8, 16, 32, 64, 128",
                            s
                        ));
                    }
                    if result == BaseType::Float && ![32, 64].contains(&s) {
                        return Err(format!(
                            "Syntax Error: Invalid size {} for float. Allowed: 32, 64",
                            s
                        ));
                    }
                    size = Some(s);
                    self.advance();
                } else {
                    return Err(format!(
                        "Syntax Error: Expected integer size for type {}",
                        result.as_str()
                    ));
                }
                self.consume(TokenKind::RParen, "Expected ')' after type size")?;
            } else if result == BaseType::Int || result == BaseType::Float {
                return Err(format!(
                    "Syntax Error: Type '{}' requires a size, e.g., {}(32)",
                    result.as_str(),
                    result.as_str()
                ));
            }

            return Ok(TypeNode::Simple(TypeRef {
                base_type: result,
                size,
            }));
        }
        if self.peek().kind == TokenKind::Less {
            self.advance();
            self.parse_generic_list(&mut generics, &mut size)?;
        }

        if !generics.is_empty() {
            return Ok(TypeNode::Generic(Generic {
                base_type: result,
                generics,
            }));
        }

        Ok(TypeNode::Simple(TypeRef {
            base_type: result,
            size,
        }))
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

            // --- Instantiate: new Target(args) ---
            TokenKind::New => {
                self.advance();
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
                let stmts = self.parse_block()?;
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
