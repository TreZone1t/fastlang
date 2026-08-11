use crate::lexer::token::{Token, TokenKind};

pub struct Scanner {
    source: Vec<char>,
    position: usize,
    line: usize,
    column: usize,
}

impl Scanner {
    pub fn new(source: String) -> Self {
        Scanner {
            source: source.chars().collect(),
            position: 0,
            line: 1,
            column: 1,
        }
    }

    /// Look at the character `offset` positions ahead without consuming anything.
    /// Bounds-checked: returns None past the end, for any offset.
    fn peek_at(&self, offset: usize) -> Option<char> {
        self.source.get(self.position + offset).copied()
    }

    fn peek(&self) -> Option<char> {
        self.peek_at(0)
    }

    fn advance(&mut self) -> Option<char> {
        if self.is_at_end() {
            None
        } else {
            let c = self.source[self.position];
            self.position += 1;
            self.column += 1;
            Some(c)
        }
    }

    fn is_at_end(&self) -> bool {
        self.position >= self.source.len()
    }

    fn skip_whitespace(&mut self) {
        while let Some(c) = self.peek() {
            match c {
                ' ' | '\r' | '\t' => {
                    self.advance();
                }
                '\n' => {
                    self.line += 1;
                    self.column = 1;
                    self.advance();
                }
                _ => break,
            }
        }
    }

    fn check_keyword(word: &str) -> Option<TokenKind> {
        match word {
            // Keywords
            "let" => Some(TokenKind::Let),
            "const" => Some(TokenKind::Const),
            "set" => Some(TokenKind::Set),
            "del" => Some(TokenKind::Del),

            // out and in
            "log" => Some(TokenKind::Log),
            // func
            "fn" => Some(TokenKind::Fn),
            "return" => Some(TokenKind::Return),
            // if
            "if" => Some(TokenKind::If),
            "else" => Some(TokenKind::Else),
            "for" => Some(TokenKind::For),
            "in" => Some(TokenKind::In),
            "switch" => Some(TokenKind::Switch),
            "case" => Some(TokenKind::Case),

            // loops
            "loop" => Some(TokenKind::Loop),
            "while" => Some(TokenKind::While),
            "continue" => Some(TokenKind::Continue),
            "break" => Some(TokenKind::Break),

            "copy" => Some(TokenKind::Copy),
            "new" => Some(TokenKind::New),

            "class" => Some(TokenKind::TypeClass),
            "enum" => Some(TokenKind::TypeEnum),
            "extends" => Some(TokenKind::Extends),
            "super" => Some(TokenKind::Super),

            // Primitives
            "char" => Some(TokenKind::TypeChar),
            "int" => Some(TokenKind::TypeInt),
            "str" => Some(TokenKind::TypeStr),
            "array" => Some(TokenKind::TypeArray),
            "float" => Some(TokenKind::TypeFloat),
            "bool" => Some(TokenKind::TypeBool),

            // Context Types
            "struct" => Some(TokenKind::TypeStruct),
            "scope" => Some(TokenKind::TypeScope),
            "param" => Some(TokenKind::TypeParam),
            "init" => Some(TokenKind::TypeInit),
            "blueprint" => Some(TokenKind::TypeBluePrint),
            "flag" => Some(TokenKind::TypeFlag),
            "generic" => Some(TokenKind::TypeGeneric),
            "type" => Some(TokenKind::TypeType),
            "event" => Some(TokenKind::TypeEvent),
            "handle" => Some(TokenKind::TypeHandle),
            "keywords" => Some(TokenKind::TypeKeyword),
            "variants" => Some(TokenKind::TypeVariants),
            "public" => Some(TokenKind::TypePublic),
            "private" => Some(TokenKind::TypePrivate),

            "static" => Some(TokenKind::TypeStatic),

            // for list and string types
            "size" => Some(TokenKind::TypeSize),
            "length" => Some(TokenKind::TypeLength),
            "data" => Some(TokenKind::TypeData),
            // memory / instances
            "modify" => Some(TokenKind::Modify),
            "this" => Some(TokenKind::This),
            "global" => Some(TokenKind::Global),

            // logical
            "and" => Some(TokenKind::And),
            "or" => Some(TokenKind::Or),

            // boolean literals
            "true" => Some(TokenKind::Bool(true)),
            "false" => Some(TokenKind::Bool(false)),

            // scope impl / unrestricted type
            "statement" => Some(TokenKind::TypeStatement),
            "custom" => Some(TokenKind::TypeCustom),
            // for custom
            "index_access" => Some(TokenKind::CustomIndexAccess),
            "display" => Some(TokenKind::CustomDisplay),
            "iterator" => Some(TokenKind::CustomIterator),
            "operators" => Some(TokenKind::CustomOperators),
            "custom_keyword" => Some(TokenKind::CustomKeyword),
            "custom_generic" => Some(TokenKind::CustomGeneric),
            "constructor" => Some(TokenKind::CustomConstructor),
            // context / magic types
            "name" => Some(TokenKind::TypeName),
            "void" => Some(TokenKind::TypeVoid),
            "object" => Some(TokenKind::TypeObject),
            "block" => Some(TokenKind::TypeBlock),

            "try" => Some(TokenKind::Try),
            "catch" => Some(TokenKind::Catch),
            "throw" => Some(TokenKind::Throw),
            "error" => Some(TokenKind::TypeError),
            "enable" => Some(TokenKind::Enable),
            "disable" => Some(TokenKind::Disable),
            "all" => Some(TokenKind::All),
            "use" => Some(TokenKind::Use),
            "export" => Some(TokenKind::Export),

            // constructor
            "_" => Some(TokenKind::Underscore),

            _ => None,
        }
    }

    pub fn next_token(&mut self) -> Token {
        self.skip_whitespace();

        // Capture the start position AFTER whitespace/comments are skipped by the
        // caller's previous call, but BEFORE this token's own characters are consumed.
        let start_line = self.line;
        let start_column = self.column;

        if self.is_at_end() {
            return Token::new(TokenKind::EOF, start_line, start_column);
        }

        let c = self.advance().unwrap();

        let kind = match c {
            'a'..='z' | 'A'..='Z' | '_' => {
                let mut word = String::new();
                word.push(c);
                while let Some(next_c) = self.peek() {
                    if next_c.is_alphanumeric() || next_c == '_' {
                        word.push(self.advance().unwrap());
                    } else {
                        break;
                    }
                }
                Self::check_keyword(&word).unwrap_or(TokenKind::Identifier(word.clone()))
            }

            '0'..='9' => {
                let mut num_str = String::new();
                num_str.push(c);

                while let Some(next_c) = self.peek() {
                    if next_c.is_ascii_digit() {
                        num_str.push(self.advance().unwrap());
                    } else {
                        break;
                    }
                }

                // Float: a '.' followed by at least one digit. A trailing bare '.'
                // (e.g. `5.` or `5.foo`) is left alone so `.` can still be a Dot token
                // (property access, etc.) on the next scan.
                let is_float = self.peek() == Some('.')
                    && matches!(self.peek_at(1), Some(d) if d.is_ascii_digit());

                if is_float {
                    num_str.push(self.advance().unwrap()); // consume '.'
                    while let Some(next_c) = self.peek() {
                        if next_c.is_ascii_digit() {
                            num_str.push(self.advance().unwrap());
                        } else {
                            break;
                        }
                    }
                    match num_str.parse::<f64>() {
                        Ok(f) => TokenKind::Float(f),
                        Err(_) => TokenKind::Error(format!(
                            "Invalid float literal '{}' at line {}",
                            num_str, start_line
                        )),
                    }
                } else {
                    match num_str.parse::<i64>() {
                        Ok(i) => TokenKind::Int(i),
                        Err(_) => TokenKind::Error(format!(
                            "Integer literal '{}' out of range at line {}",
                            num_str, start_line
                        )),
                    }
                }
            }

            '"' => {
                let mut s = String::new();
                let mut terminated = false;
                while let Some(next_c) = self.peek() {
                    if next_c == '"' {
                        self.advance();
                        terminated = true;
                        break;
                    } else if next_c == '\n' {
                        // Strings don't span lines in v1; bail out and let the caller
                        // decide this was unterminated rather than swallowing the newline.
                        break;
                    } else {
                        s.push(self.advance().unwrap());
                    }
                }
                if terminated {
                    TokenKind::String(s.clone())
                } else {
                    TokenKind::Error(format!(
                        "Unterminated string literal starting at line {}",
                        start_line
                    ))
                }
            }

            '\'' => {
                // Opening quote already consumed (it was `c`). Consume exactly one
                // character, then require the closing quote immediately after it.
                match self.advance() {
                    Some(inner) => {
                        if self.peek() == Some('\'') {
                            self.advance(); // consume closing quote
                            TokenKind::Char(inner)
                        } else {
                            TokenKind::Error(format!(
                                "Invalid char literal starting at line {}: expected closing '\''",
                                start_line
                            ))
                        }
                    }
                    None => TokenKind::Error(format!(
                        "Unterminated char literal at line {}: unexpected EOF",
                        start_line
                    )),
                }
            }

            // comments
            '/' => {
                if let Some('/') = self.peek() {
                    self.advance(); // consume the second '/'
                    while let Some(next_c) = self.peek() {
                        if next_c == '\n' {
                            // Leave the newline for skip_whitespace to consume on the
                            // next call, so line/column bookkeeping only happens in
                            // one place.
                            break;
                        }
                        self.advance();
                    }
                    TokenKind::InlineComment
                } else if let Some('*') = self.peek() {
                    self.advance(); // consume the '*' that opens the block comment
                    let mut closed = false;
                    while let Some(next_c) = self.peek() {
                        if next_c == '*' && self.peek_at(1) == Some('/') {
                            self.advance(); // consume '*'
                            self.advance(); // consume '/'
                            closed = true;
                            break;
                        }
                        if next_c == '\n' {
                            self.line += 1;
                            self.column = 1;
                        }
                        self.advance();
                    }
                    if closed {
                        TokenKind::MultiLineComment
                    } else {
                        TokenKind::Error(format!(
                            "Unterminated block comment starting at line {}",
                            start_line
                        ))
                    }
                } else {
                    TokenKind::Divide
                }
            }
            ':' => {
                if let Some(':') = self.peek() {
                    self.advance();
                    TokenKind::DoubleColon
                } else {
                    TokenKind::Colon
                }
            }
            ';' => TokenKind::SemiColon,
            ',' => TokenKind::Comma,
            '.' => TokenKind::Dot,
            '+' => {
                if let Some('+') = self.peek() {
                    self.advance();
                    TokenKind::PlusPlus
                } else {
                    TokenKind::Plus
                }
            }
            '*' => TokenKind::Multiply,
            '%' => TokenKind::Mod,
            '{' => TokenKind::LBrace,
            '}' => TokenKind::RBrace,
            '[' => TokenKind::LBracket,
            ']' => TokenKind::RBracket,
            '(' => TokenKind::LParen,
            ')' => TokenKind::RParen,
            '=' => {
                if let Some('=') = self.peek() {
                    self.advance();
                    TokenKind::Eq
                } else if let Some('>') = self.peek() {
                    self.advance();
                    TokenKind::FatArrow
                } else {
                    TokenKind::Assign
                }
            }
            '!' => {
                if let Some('=') = self.peek() {
                    self.advance();
                    TokenKind::NotEq
                } else {
                    TokenKind::Not
                }
            }
            '-' => {
                if let Some('>') = self.peek() {
                    self.advance();
                    TokenKind::Arrow
                } else if let Some('-') = self.peek() {
                    self.advance();
                    TokenKind::MinusMinus
                } else {
                    TokenKind::Minus
                }
            }
            '&' => {
                if let Some('&') = self.peek() {
                    self.advance();
                    TokenKind::And
                } else {
                    TokenKind::Ampersand
                }
            }
            '|' => {
                if let Some('|') = self.peek() {
                    self.advance();
                    TokenKind::Or
                } else {
                    TokenKind::Error(
                        "Unsupported operator '|' (bitwise OR is not supported; did you mean '||'?)"
                            .to_string(),
                    )
                }
            }
            '>' => {
                if let Some('=') = self.peek() {
                    self.advance();
                    TokenKind::GreaterEq
                } else {
                    TokenKind::Greater
                }
            }
            '<' => {
                if let Some('=') = self.peek() {
                    self.advance();
                    TokenKind::LessEq
                } else {
                    TokenKind::Less
                }
            }

            other => TokenKind::Error(format!(
                "Unexpected character '{}' at line {}",
                other, start_line
            )),
        };

        Token::new(kind, start_line, start_column)
    }
}
