use crate::frontend::parser::ast::BaseType;
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq)]
pub enum IRType {
    Void,
    Int32,
    Int64,
    Float32,
    Float64,
    Bool,
    Pointer(Box<IRType>),
    Array(Box<IRType>),
    Object(String),
    CustomScope(String),
    Generic(String),
}

impl IRType {
    pub fn from_ast(tn: &BaseType) -> Self {
        match tn {
            BaseType::Int8 | BaseType::Int16 | BaseType::Int32 => IRType::Int32,
            BaseType::Int64 | BaseType::Int128 => IRType::Int64,
            BaseType::Float32 => IRType::Float32,
            BaseType::Float64 => IRType::Float64,
            BaseType::Bool => IRType::Bool,
            BaseType::Char => IRType::Int32,
            BaseType::Void => IRType::Void,
            BaseType::Array { base_type, .. } => IRType::from_ast(base_type.as_ref()),

            BaseType::Custom { name, .. } => IRType::CustomScope(name.clone()),
            BaseType::Struct { name, .. } => IRType::CustomScope(name.clone()),
            BaseType::Class { name, .. } => IRType::CustomScope(name.clone()),
            BaseType::Enum { name, .. } => IRType::CustomScope(name.clone()),
            BaseType::Blueprint { name, .. } => IRType::CustomScope(name.clone()),
            BaseType::Generic(_) => IRType::Generic("Generic".to_string()),
            _ => IRType::Pointer(Box::new(IRType::Void)),
        }
    }
}

pub type IRValue = usize; // Virtual Register ID
pub type BlockID = usize;

#[derive(Debug, Clone)]
pub enum IROp {
    // Memory
    Alloc {
        ty: IRType,
    }, // Returns Pointer
    Load {
        ptr: IRValue,
        ty: IRType,
    },
    Store {
        ptr: IRValue,
        value: IRValue,
    },

    // Arithmetic
    Add(IRValue, IRValue),
    Sub(IRValue, IRValue),
    Mul(IRValue, IRValue),
    Div(IRValue, IRValue),
    Mod(IRValue, IRValue),

    // Constants
    ConstInt32(i32),
    ConstInt64(i64),
    ConstFloat32(f32),
    ConstFloat64(f64),
    ConstBool(bool),
    ConstString(String),

    // Logic / Comparisons
    Eq(IRValue, IRValue),
    Neq(IRValue, IRValue),
    Lt(IRValue, IRValue),
    Le(IRValue, IRValue),
    Gt(IRValue, IRValue),
    Ge(IRValue, IRValue),
    And(IRValue, IRValue),
    Or(IRValue, IRValue),

    // Control Flow
    Call {
        func: String,
        args: Vec<IRValue>,
    },
    Return(Option<IRValue>),
    Jump(BlockID),
    BranchIf {
        cond: IRValue,
        true_block: BlockID,
        false_block: BlockID,
    },
}

#[derive(Debug, Clone)]
pub struct IRInstruction {
    pub id: Option<IRValue>, // The register that stores the result of this operation (if any)
    pub op: IROp,
}

#[derive(Debug, Clone)]
pub struct BasicBlock {
    pub id: BlockID,
    pub instructions: Vec<IRInstruction>,
}

#[derive(Debug, Clone)]
pub struct IRFunction {
    pub name: String,
    pub params: Vec<(String, IRType)>,
    pub return_type: IRType,
    pub blocks: HashMap<BlockID, BasicBlock>,
    pub entry_block: BlockID,
    pub next_vreg: usize,
    pub next_block_id: usize,
}

impl IRFunction {
    pub fn new(name: String, params: Vec<(String, IRType)>, return_type: IRType) -> Self {
        let mut func = IRFunction {
            name,
            params,
            return_type,
            blocks: HashMap::new(),
            entry_block: 0,
            next_vreg: 1,
            next_block_id: 1,
        };
        func.blocks.insert(
            0,
            BasicBlock {
                id: 0,
                instructions: Vec::new(),
            },
        );
        func
    }

    pub fn new_vreg(&mut self) -> IRValue {
        let v = self.next_vreg;
        self.next_vreg += 1;
        v
    }

    pub fn new_block(&mut self) -> BlockID {
        let b = self.next_block_id;
        self.next_block_id += 1;
        self.blocks.insert(
            b,
            BasicBlock {
                id: b,
                instructions: Vec::new(),
            },
        );
        b
    }

    pub fn add_inst(&mut self, block: BlockID, inst: IRInstruction) {
        if let Some(b) = self.blocks.get_mut(&block) {
            b.instructions.push(inst);
        }
    }
}

#[derive(Debug, Clone)]
pub struct IRModule {
    pub name: String,
    pub functions: Vec<IRFunction>,
}
use std::fmt;

impl fmt::Display for IRType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            IRType::Void => write!(f, "void"),
            IRType::Int32 => write!(f, "i32"),
            IRType::Int64 => write!(f, "i64"),
            IRType::Float32 => write!(f, "f32"),
            IRType::Float64 => write!(f, "f64"),
            IRType::Bool => write!(f, "bool"),
            IRType::Pointer(inner) => write!(f, "ptr<{}>", inner),
            IRType::Array(inner) => write!(f, "array<{}>", inner),
            IRType::Object(name) => write!(f, "obj<{}>", name),
            IRType::CustomScope(name) => write!(f, "custom<{}>", name),
            IRType::Generic(name) => write!(f, "generic<{}>", name),
        }
    }
}

impl fmt::Display for IROp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            IROp::Alloc { ty } => write!(f, "alloc {}", ty),
            IROp::Load { ptr, ty } => write!(f, "load v{} as {}", ptr, ty),
            IROp::Store { ptr, value } => write!(f, "store v{} -> [v{}]", value, ptr),
            IROp::Add(l, r) => write!(f, "add v{}, v{}", l, r),
            IROp::Sub(l, r) => write!(f, "sub v{}, v{}", l, r),
            IROp::Mul(l, r) => write!(f, "mul v{}, v{}", l, r),
            IROp::Div(l, r) => write!(f, "div v{}, v{}", l, r),
            IROp::Mod(l, r) => write!(f, "mod v{}, v{}", l, r),
            IROp::ConstInt32(v) => write!(f, "const.i32 {}", v),
            IROp::ConstInt64(v) => write!(f, "const.i64 {}", v),
            IROp::ConstFloat32(v) => write!(f, "const.f32 {}", v),
            IROp::ConstFloat64(v) => write!(f, "const.f64 {}", v),
            IROp::ConstBool(v) => write!(f, "const.bool {}", v),
            IROp::ConstString(v) => write!(f, "const.str \"{}\"", v),
            IROp::Eq(l, r) => write!(f, "eq v{}, v{}", l, r),
            IROp::Neq(l, r) => write!(f, "neq v{}, v{}", l, r),
            IROp::Lt(l, r) => write!(f, "lt v{}, v{}", l, r),
            IROp::Le(l, r) => write!(f, "le v{}, v{}", l, r),
            IROp::Gt(l, r) => write!(f, "gt v{}, v{}", l, r),
            IROp::Ge(l, r) => write!(f, "ge v{}, v{}", l, r),
            IROp::And(l, r) => write!(f, "and v{}, v{}", l, r),
            IROp::Or(l, r) => write!(f, "or v{}, v{}", l, r),
            IROp::Call { func, args } => {
                write!(f, "call {}(", func)?;
                for (i, arg) in args.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "v{}", arg)?;
                }
                write!(f, ")")
            }
            IROp::Return(Some(v)) => write!(f, "ret v{}", v),
            IROp::Return(None) => write!(f, "ret"),
            IROp::Jump(b) => write!(f, "jmp block_{}", b),
            IROp::BranchIf {
                cond,
                true_block,
                false_block,
            } => write!(
                f,
                "br_if v{}, block_{}, block_{}",
                cond, true_block, false_block
            ),
        }
    }
}

impl fmt::Display for IRInstruction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(id) = self.id {
            write!(f, "v{} = {}", id, self.op)
        } else {
            write!(f, "{}", self.op)
        }
    }
}

impl fmt::Display for BasicBlock {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "block_{}:", self.id)?;
        for inst in &self.instructions {
            writeln!(f, "  {}", inst)?;
        }
        Ok(())
    }
}

impl fmt::Display for IRFunction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "fn {}(", self.name)?;
        for (i, (name, ty)) in self.params.iter().enumerate() {
            if i > 0 {
                write!(f, ", ")?;
            }
            write!(f, "{}: {}", name, ty)?;
        }
        writeln!(f, ") -> {} {{", self.return_type)?;

        let mut block_ids: Vec<&usize> = self.blocks.keys().collect();
        block_ids.sort();
        for bid in block_ids {
            write!(f, "{}", self.blocks[bid])?;
        }
        writeln!(f, "}}")
    }
}

impl fmt::Display for IRModule {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "module {} {{", self.name)?;
        for func in &self.functions {
            writeln!(f, "{}", func)?;
        }
        writeln!(f, "}}")
    }
}
