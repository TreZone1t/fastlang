use crate::lexer::token::{Token, TokenKind};
use crate::parser::ast::*;
use crate::parser::parser::Parser;

impl Parser {
    pub(crate) fn is_type_token(kind: &TokenKind) -> bool {
        match kind {
            TokenKind::TypeInt
            | TokenKind::TypeFloat
            | TokenKind::TypeStr
            | TokenKind::TypeArray
            | TokenKind::TypeBool
            | TokenKind::TypeChar
            | TokenKind::TypeName
            | TokenKind::TypeLength
            | TokenKind::TypeSize
            | TokenKind::TypeScope
            | TokenKind::TypeFlag
            | TokenKind::TypeParam
            | TokenKind::TypeType
            | TokenKind::TypeBluePrint
            | TokenKind::TypeInit
            | TokenKind::TypeStatic
            | TokenKind::TypePublic
            | TokenKind::TypePrivate
            | TokenKind::TypeEvent
            | TokenKind::TypeHandle
            | TokenKind::TypeCustom
            | TokenKind::TypeStruct
            | TokenKind::TypeVoid
            | TokenKind::TypeString
            | TokenKind::TypeBlock
            | TokenKind::TypeObject => true,
            _ => false,
        }
    }

    /// Helper موحد — يقرأ token يمثل type ويرجع اسمه كـ String.
    /// يقبل:
    ///   Primitives: int, float, str, bool, char
    ///   Context/Magic types: name, length, size, scope, flag, param,
    ///                        type, blueprint, init, static, public, private,
    ///                        event, handle, custom
    /// يرجع None لو الـ token الحالي مش type أصلاً.
    pub(crate) fn parse_type(&mut self) -> Result<(String, Option<i64>), String> {
        let result = match &self.peek().kind {
            TokenKind::TypeInt => "int".to_string(),
            TokenKind::TypeFloat => "float".to_string(),
            TokenKind::TypeStr => "str".to_string(),
            TokenKind::TypeArray => "array".to_string(),
            TokenKind::TypeBool => "bool".to_string(),
            TokenKind::TypeChar => "char".to_string(),
            TokenKind::TypeName => "name".to_string(),
            TokenKind::TypeLength => "length".to_string(),
            TokenKind::TypeSize => "size".to_string(),
            TokenKind::TypeScope => "scope".to_string(),
            TokenKind::TypeFlag => "flag".to_string(),
            TokenKind::TypeParam => "param".to_string(),
            TokenKind::TypeType => "type".to_string(),
            TokenKind::TypeBluePrint => "blueprint".to_string(),
            TokenKind::TypeInit => "init".to_string(),
            TokenKind::TypeStatic => "static".to_string(),
            TokenKind::TypePublic => "public".to_string(),
            TokenKind::TypePrivate => "private".to_string(),
            TokenKind::TypeEvent => "event".to_string(),
            TokenKind::TypeHandle => "handle".to_string(),
            TokenKind::TypeCustom => "custom".to_string(),
            TokenKind::TypeStruct => "struct".to_string(),
            TokenKind::TypeVoid => "void".to_string(),
            TokenKind::TypeString => "string".to_string(),
            TokenKind::TypeBlock => "block".to_string(),
            TokenKind::TypeObject => "object".to_string(),
            TokenKind::Identifier(n) => n.clone(),
            _ => return Err("Expected a type".to_string()),
        };
        self.advance();

        // Enforce sizes for int, float, str, string, and array
        let mut size = None;
        let mut final_result = result.clone();

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
            } else {
                if result == "int" || result == "float" {
                    return Err(format!(
                        "Syntax Error: Type '{}' requires a size, e.g., {}(32)",
                        result, result
                    ));
                }
            }
        } else if result == "array" {
            if self.peek().kind == TokenKind::LParen {
                self.advance();
                let (inner_type, inner_size) = self.parse_type()?;
                let formatted_inner = if let Some(s) = inner_size {
                    format!("{}({})", inner_type, s)
                } else {
                    inner_type
                };
                final_result = format!("array<{}>", formatted_inner);

                if self.peek().kind == TokenKind::Comma {
                    self.advance();
                    if let TokenKind::Int(s) = self.peek().kind {
                        size = Some(s);
                        self.advance();
                    } else if self.peek().kind == TokenKind::TypeSize || matches!(self.peek().kind, TokenKind::Identifier(_)) {
                        size = Some(-1); // Dynamic size
                        self.advance();
                    } else {
                        return Err("Syntax Error: Expected integer size for array".to_string());
                    }
                }
                self.consume(
                    TokenKind::RParen,
                    "Expected ')' after array inner type or size",
                )?;
            } else {
                return Err("Syntax Error: Type 'array' requires an inner type, e.g., array(int(32)) or array(int(32), 10)".to_string());
            }
        } else if self.peek().kind == TokenKind::LParen {
            // Optional size or generic type argument
            self.advance();
            if let TokenKind::Int(s) = self.peek().kind {
                size = Some(s);
                self.advance();
                self.consume(TokenKind::RParen, "Expected ')' after type size")?;
            } else {
                // Try parsing it as an inner type
                let old_pos = self.current;
                if let Ok((inner_type, inner_size)) = self.parse_type() {
                    if self.peek().kind == TokenKind::RParen {
                        self.advance(); // consume ')'
                        let formatted_inner = if let Some(s) = inner_size {
                            format!("{}({})", inner_type, s)
                        } else {
                            inner_type
                        };
                        final_result = format!("{}<{}>", final_result, formatted_inner);
                    } else if self.peek().kind == TokenKind::Comma {
                        self.advance(); // consume ','
                        if let TokenKind::Int(s) = self.peek().kind {
                            size = Some(s);
                            self.advance();
                        } else if self.peek().kind == TokenKind::TypeSize || matches!(self.peek().kind, TokenKind::Identifier(_)) {
                            size = Some(-1);
                            self.advance();
                        } else {
                            return Err("Syntax Error: Expected integer size for array".to_string());
                        }
                        self.consume(TokenKind::RParen, "Expected ')' after generic type or size")?;
                        let formatted_inner = if let Some(s) = inner_size {
                            format!("{}({})", inner_type, s)
                        } else {
                            inner_type
                        };
                        final_result = format!("{}<{}>", final_result, formatted_inner);
                    } else {
                        // Not a valid generic syntax, fallback to dynamic size marker
                        self.current = old_pos;
                        if self.peek().kind == TokenKind::TypeSize || matches!(self.peek().kind, TokenKind::Identifier(_)) {
                            size = Some(-1);
                            self.advance();
                        }
                        self.consume(TokenKind::RParen, "Expected ')' after type size")?;
                    }
                } else {
                    self.current = old_pos;
                    if self.peek().kind == TokenKind::TypeSize || matches!(self.peek().kind, TokenKind::Identifier(_)) {
                        size = Some(-1);
                        self.advance();
                    }
                    self.consume(TokenKind::RParen, "Expected ')' after type size")?;
                }
            }
        } else if self.peek().kind == TokenKind::Less {
            // Generics: Option<name>
            self.advance(); // consume '<'
            let (inner_type, _) = self.parse_type()?;
            self.consume(TokenKind::Greater, "Expected '>' after generic type")?;
            final_result = format!("{}<{}>", final_result, inner_type);
        }

        Ok((final_result, size))
    }

    // ====================================================
    // Expression Parser — Pratt / Top-Down Operator Precedence
    // ====================================================
    //
    // فكرة Pratt: كل operator ليه "binding power" (قوة ربط).
    // parse_expr(min_bp) بتاكل operators طالما قوتها أكبر من min_bp.
    // ده بيحل مشكلة الـ precedence بشكل طبيعي.
    //
    // المستويات:
    //   or/||       -> 1
    //   and/&&      -> 2
    //   == / !=     -> 3
    //   < / >       -> 4
    //   + / -       -> 5
    //   * / / / %   -> 6
    //   Unary ! -   -> prefix, right-associative (bp = 7)
    //   . () []     -> postfix, left-associative (bp = 8)

    pub(crate) fn parse_expression(&mut self) -> Result<Expr, String> {
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
                Ok(Expr::LiteralString(val))
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
                Ok(Expr::Identifier(val))
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
                    while self.peek().kind == TokenKind::Comma {
                        self.advance();
                        elements.push(self.parse_expr(0)?);
                    }
                }
                self.consume(TokenKind::RBracket, "Expected ']' to close array literal")?;
                Ok(Expr::ArrayLiteral(elements))
            }

            // --- Instantiate: new/copy/modify Target(args) ---
            TokenKind::New | TokenKind::Copy | TokenKind::Modify => {
                let op = match self.peek().kind {
                    TokenKind::New => "new",
                    TokenKind::Copy => "copy",
                    TokenKind::Modify => "modify",
                    _ => unreachable!(),
                }
                .to_string();
                self.advance();

                // Target can be an identifier (like Counter), or another expression
                let target = self.parse_expr(9)?; // Bind tightly to the target (higher than postfix 8)

                // Optional arguments
                let mut args = Vec::new();
                if self.peek().kind == TokenKind::LParen {
                    self.advance();
                    if self.peek().kind != TokenKind::RParen {
                        args.push(self.parse_expr(0)?);
                        while self.peek().kind == TokenKind::Comma {
                            self.advance();
                            args.push(self.parse_expr(0)?);
                        }
                    }
                    self.consume(TokenKind::RParen, "Expected ')' to close arguments")?;
                }

                Ok(Expr::Instantiate {
                    op,
                    target: Box::new(target),
                    args,
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
                if let Some(kw_name) = Self::keyword_as_identifier(other) {
                    self.advance();
                    Ok(Expr::Identifier(kw_name))
                } else {
                    Err(format!(
                        "Syntax Error: Unexpected token '{:?}' in expression at line {}, column {}",
                        other, line, col
                    ))
                }
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
                let prop = if let TokenKind::Identifier(name) = &self.peek().kind.clone() {
                    let n = name.clone();
                    self.advance();
                    n
                } else if let Some(kw_name) = Self::keyword_as_identifier(&self.peek().kind.clone())
                {
                    self.advance();
                    kw_name
                } else {
                    return Err(format!(
                        "Syntax Error: Expected field name after '.' at line {}, column {}",
                        self.peek().line,
                        self.peek().column
                    ));
                };
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
                    property: Box::new(Expr::Identifier(prop)),
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
