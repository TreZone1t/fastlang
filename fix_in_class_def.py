import os

filename = 'src/codegen/generator.rs'
with open(filename, 'r', encoding='utf-8') as f:
    content = f.read()

content = content.replace('in_class_def:', 'pub(crate) in_class_def:')

with open(filename, 'w', encoding='utf-8') as f:
    f.write(content)
