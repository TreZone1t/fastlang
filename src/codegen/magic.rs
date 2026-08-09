use crate::parser::ast::*;
use crate::codegen::generator::CodeGenerator;

impl CodeGenerator {
    pub(crate) fn generate_magic_cast(&mut self) -> String {
        "/* magic casting */".to_string()
    }
}
