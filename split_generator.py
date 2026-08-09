import os
import re

with open('src/codegen/generator.rs', 'r', encoding='utf-8') as f:
    lines = f.readlines()

stmt_start = -1
expr_start = -1
type_start = -1
for i, line in enumerate(lines):
    if 'fn visit_statement(' in line:
        stmt_start = i
    if 'fn visit_expression(' in line:
        expr_start = i
    if 'fn map_type(' in line:
        type_start = i

stmt_lines = lines[stmt_start:expr_start]
expr_lines = lines[expr_start:type_start]
generator_lines = lines[:stmt_start] + lines[type_start:]

# Fix pub(crate) in generator_lines
for i in range(len(generator_lines)):
    if 'output: String,' in generator_lines[i]:
        generator_lines[i] = generator_lines[i].replace('output:', 'pub(crate) output:')
    if 'indent_level: usize,' in generator_lines[i]:
        generator_lines[i] = generator_lines[i].replace('indent_level:', 'pub(crate) indent_level:')
    if 'in_switch:' in generator_lines[i]:
        generator_lines[i] = generator_lines[i].replace('in_switch:', 'pub(crate) in_switch:')
    if 'current_switch_type:' in generator_lines[i]:
        generator_lines[i] = generator_lines[i].replace('current_switch_type:', 'pub(crate) current_switch_type:')

with open('src/codegen/stmt.rs', 'w', encoding='utf-8') as f:
    f.write('use crate::parser::ast::*;\n')
    f.write('use crate::codegen::generator::CodeGenerator;\n\n')
    f.write('impl CodeGenerator {\n')
    f.writelines(stmt_lines)
    f.write('}\n')

with open('src/codegen/expr.rs', 'w', encoding='utf-8') as f:
    f.write('use crate::parser::ast::*;\n')
    f.write('use crate::codegen::generator::CodeGenerator;\n\n')
    f.write('impl CodeGenerator {\n')
    f.writelines(expr_lines)
    f.write('}\n')

with open('src/codegen/magic.rs', 'w', encoding='utf-8') as f:
    f.write('use crate::parser::ast::*;\n')
    f.write('use crate::codegen::generator::CodeGenerator;\n\n')
    f.write('impl CodeGenerator {\n')
    f.write('    pub(crate) fn generate_magic_cast(&mut self) -> String {\n')
    f.write('        \"/* magic casting */\".to_string()\n')
    f.write('    }\n')
    f.write('}\n')

with open('src/codegen/generator.rs', 'w', encoding='utf-8') as f:
    f.writelines(generator_lines)

with open('src/codegen/mod.rs', 'w', encoding='utf-8') as f:
    f.write('pub mod expr;\n')
    f.write('pub mod generator;\n')
    f.write('pub mod magic;\n')
    f.write('pub mod stmt;\n')

