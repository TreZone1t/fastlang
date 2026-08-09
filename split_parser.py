import os

with open('src/parser/parser.rs', 'r', encoding='utf-8') as f:
    lines = f.readlines()

# stmt.rs lines: from n parse_statement (line 141) up to n parse_var_decl_bare end (line 1793 approx)
# expr.rs lines: from n is_type_token (line 1794) to end of impl Parser.

# Find indices
stmt_start = -1
expr_start = -1
for i, line in enumerate(lines):
    if 'fn parse_statement(' in line:
        stmt_start = i
    if 'fn is_type_token(' in line:
        expr_start = i

end_impl = len(lines) - 1
while end_impl > 0 and '}' not in lines[end_impl]:
    end_impl -= 1

stmt_lines = lines[stmt_start:expr_start]
expr_lines = lines[expr_start:end_impl]
parser_lines = lines[:stmt_start] + lines[end_impl:]

# Modify parser_lines to change private fields to pub(crate)
for i in range(len(parser_lines)):
    if 'tokens: Vec<Token>,' in parser_lines[i]:
        parser_lines[i] = parser_lines[i].replace('tokens:', 'pub(crate) tokens:')
    if 'current: usize,' in parser_lines[i]:
        parser_lines[i] = parser_lines[i].replace('current:', 'pub(crate) current:')

with open('src/parser/stmt.rs', 'w', encoding='utf-8') as f:
    f.write('use crate::lexer::scanner::{Token, TokenKind};\n')
    f.write('use crate::parser::ast::*;\n')
    f.write('use crate::parser::parser::Parser;\n\n')
    f.write('impl Parser {\n')
    f.writelines(stmt_lines)
    f.write('}\n')

with open('src/parser/expr.rs', 'w', encoding='utf-8') as f:
    f.write('use crate::lexer::scanner::{Token, TokenKind};\n')
    f.write('use crate::parser::ast::*;\n')
    f.write('use crate::parser::parser::Parser;\n\n')
    f.write('impl Parser {\n')
    f.writelines(expr_lines)
    f.write('}\n')

with open('src/parser/magic.rs', 'w', encoding='utf-8') as f:
    f.write('use crate::lexer::scanner::{Token, TokenKind};\n')
    f.write('use crate::parser::ast::*;\n')
    f.write('use crate::parser::parser::Parser;\n\n')
    f.write('impl Parser {\n')
    f.write('    pub fn parse_magic_cast(&mut self) -> Result<Expr, String> {\n')
    f.write('        Err(\"Magic Casting not implemented yet\".to_string())\n')
    f.write('    }\n')
    f.write('}\n')

with open('src/parser/parser.rs', 'w', encoding='utf-8') as f:
    f.writelines(parser_lines)

with open('src/parser/mod.rs', 'w', encoding='utf-8') as f:
    f.write('pub mod ast;\n')
    f.write('pub mod expr;\n')
    f.write('pub mod magic;\n')
    f.write('pub mod parser;\n')
    f.write('pub mod stmt;\n')

