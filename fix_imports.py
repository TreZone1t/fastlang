import os

for filename in ['src/parser/stmt.rs', 'src/parser/expr.rs', 'src/parser/magic.rs']:
    with open(filename, 'r', encoding='utf-8') as f:
        content = f.read()
    
    content = content.replace('use crate::lexer::scanner::{Token, TokenKind};', 'use crate::lexer::token::{Token, TokenKind};')
    
    with open(filename, 'w', encoding='utf-8') as f:
        f.write(content)
