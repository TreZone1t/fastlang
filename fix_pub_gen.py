import os
import re

for filename in ['src/codegen/generator.rs', 'src/codegen/stmt.rs', 'src/codegen/expr.rs', 'src/codegen/magic.rs']:
    with open(filename, 'r', encoding='utf-8') as f:
        content = f.read()
    
    # Replace     fn  with     pub(crate) fn 
    content = re.sub(r'^(\s*)fn ', r'\1pub(crate) fn ', content, flags=re.MULTILINE)
    
    with open(filename, 'w', encoding='utf-8') as f:
        f.write(content)
