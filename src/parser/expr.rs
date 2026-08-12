use crate::lexer::token::TokenKind;
use crate::parser::ast::*;
use crate::parser::parser::Parser;
use crate::parser::scope::{builtins, class, custom, enum_decl, struct_decl};
impl Parser {
    pub(crate) fn is_type_token(&self, kind: &TokenKind) -> bool {
        match kind {
            // Primitives — always valid as types
            TokenKind::TypeInt
            | TokenKind::TypeFloat
            | TokenKind::TypeBool
            | TokenKind::TypeChar
            | TokenKind::TypeName
            | TokenKind::TypeVoid => true,

            // Built-in scope-backed types — always valid (array/str are first-class type keywords)
            TokenKind::TypeArray | TokenKind::TypeStr => true,

            // `scope` is the general type for any scope value
            // (only custom and block scopes can be passed this way)
            TokenKind::TypeScope => true,

            // Custom keywords registered dynamically via `keyword -> "...";` in scope bodies
            // OR any registered built-in type token (for future use when std is fully external)
            t => {
                self.registered_builtin_type_tokens
                    .iter()
                    .any(|r| core::mem::discriminant(r) == core::mem::discriminant(t))
                    || matches!(t, TokenKind::Identifier(n) if self.custom_keywords.contains(n))
            }
        }
    }

    /// Helper موحد — يقرأ token يمثل type ويرجع اسمه كـ String.
    /// يقبل:
    ///   Primitives: int, float, str, bool, char
    ///   Context/Magic types: name, length, size, scope, flag, param,
    ///                        type, blueprint, init, static, public, private,
    ///                        event, handle, custom
    /// يرجع None لو الـ token الحالي مش type أصلاً.
    fn parse_generic_list(
        &mut self,
        generics: &mut Vec<crate::parser::ast::TypeNode>,
        size: &mut Option<i64>,
        is_array: bool,
    ) -> Result<(), String> {
        if self.peek().kind != TokenKind::Greater {
            generics.push(self.parse_type()?);

            while self.peek().kind == TokenKind::Comma {
                self.advance();
                if is_array && generics.len() == 1 {
                    if let TokenKind::Int(s) = self.peek().kind {
                        *size = Some(s);
                        self.advance();
                        break;
                    }
                }
                generics.push(self.parse_type()?);
            }
        }
        self.consume(TokenKind::Greater, "Expected '>' after type parameters")?;
        Ok(())
    }

    pub(crate) fn parse_type(&mut self) -> Result<crate::parser::ast::TypeNode, String> {
        let result = match &self.peek().kind {
            // Primitives
            TokenKind::TypeInt => "int".to_string(),
            TokenKind::TypeFloat => "float".to_string(),
            TokenKind::TypeBool => "bool".to_string(),
            TokenKind::TypeChar => "char".to_string(),
            // Magic value types
            TokenKind::TypeName => "name".to_string(),
            TokenKind::TypeVoid => "void".to_string(),
            // `scope` as a type — represents any scope value
            TokenKind::TypeScope => "scope".to_string(),
            // Built-in scope-backed types: always valid as first-class type keywords
            TokenKind::TypeArray => "array".to_string(),
            TokenKind::TypeStr => "str".to_string(),
            // Custom scope keywords — registered dynamically via `keyword -> "...";`
            TokenKind::Identifier(n) if self.custom_keywords.contains(n) => n.to_string(),
            // User-defined type names (struct/class instances) via bare Identifier
            TokenKind::Identifier(n) => n.to_string(),
            _ => {
                return Err(format!(
                    "Syntax Error: Expected a type, found '{}'.",
                    self.peek().kind.as_str()
                ))
            }
        };
        self.advance();

        let mut size = None;
        let mut generics = Vec::new();

        if result == "int" || result == "float" || result == "str" || result == "string" {
            if self.peek().kind == TokenKind::LParen {
                self.advance();
                if let TokenKind::Int(s) = self.peek().kind {
                    if result == "int" && ![8, 16, 32, 64, 128].contains(&s) {
                        return Err(format!(
                            "Syntax Error: Invalid size {} for int. Allowed: 8, 16, 32, 64, 128",
                            s
                        ));
                    }
                    if result == "float" && ![32, 64].contains(&s) {
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
                        result
                    ));
                }
                self.consume(TokenKind::RParen, "Expected ')' after type size")?;
            } else if result == "int" || result == "float" {
                return Err(format!(
                    "Syntax Error: Type '{}' requires a size, e.g., {}(32)",
                    result, result
                ));
            }

            return Ok(crate::parser::ast::TypeNode::Simple(
                crate::parser::ast::TypeRef {
                    base_type: result,
                    size,
                },
            ));
        }
        if self.peek().kind == TokenKind::Less {
            self.advance();
            self.parse_generic_list(&mut generics, &mut size, result == "array")?;
        }

        if !generics.is_empty() {
            return Ok(crate::parser::ast::TypeNode::Generic(
                crate::parser::ast::TypeGeneric {
                    base_type: result,
                    generics,
                },
            ));
        }

        Ok(crate::parser::ast::TypeNode::Simple(
            crate::parser::ast::TypeRef {
                base_type: result,
                size,
            },
        ))
    }
    pub(crate) fn parse_expression(&mut self) -> Result<Expr, String> {
        println!(
            "DEBUG: Successfully parsed array. Next token is: {:?}",
            self.peek().kind
        );
        self.parse_expr(0)
    }

    /// الدالة الأساسية لـ Pratt Parser.
    /// `min_bp`: أدنى binding power مقبول في الجانب الأيمن.
    pub(crate) fn parse_expr(&mut self, min_bp: u8) -> Result<Expr, String> {
        // --- Prefix: اقرأ الـ left-hand side أولاً ---
        let mut lhs = self.parse_prefix()?;

        // --- Infix / Postfix: استمر طالما في operators بقوة كافية ---
        loop {
            // نقطة الوصول (postfix): . و ()
            if let Some(postfix_bp) = self.postfix_binding_power() {
                if postfix_bp < min_bp {
                    break;
                }
                lhs = self.parse_postfix(lhs)?;
                println!(
                    "DEBUG: Successfully parsed array. Next token is: {:?}",
                    self.peek().kind
                );
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
                println!(
                    "DEBUG: Successfully parsed array. Next token is: {:?}",
                    self.peek().kind
                );
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

    /// يقرأ prefix expressions: literals، identifiers، unary ops، grouped.
    pub(crate) fn parse_prefix(&mut self) -> Result<Expr, String> {
        let line = self.peek().line;
        let col = self.peek().column;

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
                println!(
                    "DEBUG: Successfully parsed array. Next token is: {:?}",
                    self.peek().kind
                );
                Ok(Expr::LiteralInt(val))
            }
            TokenKind::Float(v) => {
                let val = *v;
                self.advance();
                println!(
                    "DEBUG: Successfully parsed array. Next token is: {:?}",
                    self.peek().kind
                );
                Ok(Expr::LiteralFloat(val))
            }
            TokenKind::String(s) => {
                let val = s.clone();
                self.advance();
                println!(
                    "DEBUG: Successfully parsed array. Next token is: {:?}",
                    self.peek().kind
                );
                Ok(Expr::LiteralString(val.to_string()))
            }
            TokenKind::Char(c) => {
                let val = *c;
                self.advance();
                println!(
                    "DEBUG: Successfully parsed array. Next token is: {:?}",
                    self.peek().kind
                );
                Ok(Expr::LiteralChar(val))
            }
            TokenKind::Bool(b) => {
                let val = *b;
                self.advance();
                println!(
                    "DEBUG: Successfully parsed array. Next token is: {:?}",
                    self.peek().kind
                );
                Ok(Expr::LiteralBool(val))
            }

            // --- Identifier ---
            TokenKind::Identifier(name) => {
                let val = name.clone();
                self.advance();
                println!(
                    "DEBUG: Successfully parsed array. Next token is: {:?}",
                    self.peek().kind
                );
                Ok(Expr::Identifier(val.to_string()))
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
                    println!(
                        "DEBUG: Successfully parsed array. Next token is: {:?}",
                        self.peek().kind
                    );
                    while self.peek().kind == TokenKind::Comma {
                        self.advance();
                        elements.push(self.parse_expr(0)?);
                    }
                }
                self.consume(TokenKind::RBracket, "Expected ']' to close array literal")?;
                println!(
                    "DEBUG: Successfully parsed array. Next token is: {:?}",
                    self.peek().kind
                );
                Ok(Expr::ArrayLiteral(elements))
            }

            // --- Instantiate: new Target(args) ---
            TokenKind::New => {
                self.advance();
                let target = self.parse_expr(9)?;
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
                Ok(Expr::ObjectLiteral(stmts))
            }

            // --- Keywords used as identifier expressions (e.g. `flag && check`) ---
            other => {
                let kw_name = other.as_str().to_string();
                self.advance();
                Ok(Expr::Identifier(kw_name))
            }
        }
    }

    /// يرجع الـ left binding power للـ postfix operators (. و ())
    /// None لو اللي قدامنا مش postfix operator.
    pub(crate) fn postfix_binding_power(&self) -> Option<u8> {
        match &self.peek().kind {
            TokenKind::Dot => Some(8),        // property access: obj.field
            TokenKind::LParen => Some(8),     // function call:   foo(...)
            TokenKind::LBracket => Some(8),   // array indexing: arr[0]
            TokenKind::PlusPlus => Some(9),   // postfix ++
            TokenKind::MinusMinus => Some(9), // postfix --
            _ => None,
        }
    }

    /// يقرأ postfix operation على الـ lhs اللي اتبنى قبل كده.
    pub(crate) fn parse_postfix(&mut self, lhs: Expr) -> Result<Expr, String> {
        match &self.peek().kind.clone() {
            // --- Property Access: lhs.identifier ---
            TokenKind::Dot => {
                self.advance(); // نتخطى '.'
                                // نقبل identifiers وكمان keywords كـ field names (زي .length, .size)
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
                self.advance(); // نتخطى '::'
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

    /// يرجع string representation للـ operator اللي قدامنا حالياً.
    /// يُستدعى قبل advance() في parse_expr.
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
