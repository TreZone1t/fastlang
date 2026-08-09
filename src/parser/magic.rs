use crate::lexer::token::{Token, TokenKind};
use crate::parser::ast::*;
use crate::parser::parser::Parser;

impl Parser {
    pub fn parse_magic_cast(&mut self) -> Result<Expr, String> {
        Err("Magic Casting not implemented yet".to_string())
    }
}
