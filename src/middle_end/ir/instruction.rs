use crate::frontend::parser::ast::*;
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq)]
pub enum IRType {
    Void,
    Int(Size),
    UInt(Size),
    Float(Size),
    Bool,
    Char,
    Byte,
    USize,
    ISize,
    Type,
    Pointer(Box<IRType>),
    Array(Box<IRType>),
    Object(String),
    CustomScope(String),
    Generic(String),
}

impl IRType {
    pub fn from_ast(tn: &BaseType) -> Self {
        match tn {
            BaseType::Int(s) => IRType::Int(s.clone()),
            BaseType::Char => IRType::Char,
            BaseType::UInt(s) => IRType::UInt(s.clone()),
            BaseType::USize => IRType::USize,
            BaseType::ISize => IRType::ISize,
            BaseType::Float(s) => IRType::Float(s.clone()),
            BaseType::Bool => IRType::Bool,
            BaseType::Type(_) => IRType::Type,
            BaseType::Void => IRType::Void,
            BaseType::Array { base_type, .. } =>
                IRType::Array(Box::new(IRType::from_ast(base_type.as_ref()))),
            BaseType::Pointer(inner) => IRType::Pointer(Box::new(IRType::from_ast(inner.as_ref()))),
            BaseType::Block { name, .. } | BaseType::Machine { name, .. } =>
                IRType::CustomScope(name.clone()),
            BaseType::Struct { name, .. } => IRType::CustomScope(name.clone()),
            BaseType::Class { name, .. } => IRType::CustomScope(name.clone()),
            BaseType::Enum { name, .. } => IRType::CustomScope(name.clone()),
            BaseType::Blueprint { name, .. } => IRType::CustomScope(name.clone()),
            BaseType::Generic(_) => IRType::Generic("Generic".to_string()),
            _ => IRType::Pointer(Box::new(IRType::Void)),
        }
    }

    pub fn size_in_bytes(&self) -> usize {
        match self {
            IRType::Void => 0,
            IRType::Bool | IRType::Int(Size::S8) | IRType::UInt(Size::S8) | IRType::Byte => 1,
            IRType::Int(Size::S16) | IRType::UInt(Size::S16) => 2,
            | IRType::Int(Size::S32)
            | IRType::UInt(Size::S32)
            | IRType::Float(Size::S32)
            | IRType::Char
            | IRType::Type => 4,
            | IRType::Int(Size::S64)
            | IRType::UInt(Size::S64)
            | IRType::Float(Size::S64)
            | IRType::USize
            | IRType::ISize => 8,
            IRType::Int(Size::S128) | IRType::UInt(Size::S128) | IRType::Float(Size::S128) => 8,
            IRType::Pointer(_) | IRType::Array(_) | IRType::Object(_) | IRType::CustomScope(_) => 8,
            IRType::Generic(_) => 8,
            _ => 8, // todo : that is bad
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
    AllocArray {
        elem_ty: IRType,
        size: usize,
    },
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
    ConstTypeID(u32),

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

    // Param and Heap / Field Access
    StoreParam {
        param_idx: usize,
        ptr: IRValue,
    },
    GetFieldPtr {
        ptr: IRValue,
        offset: i32,
    },
    GetElementPtr {
        base_ptr: IRValue,
        index: IRValue,
        elem_size: usize,
    },
    LoadMemory {
        ptr: IRValue,
        ty: IRType,
    },
    StoreMemory {
        ptr: IRValue,
        value: IRValue,
    },
    Not(IRValue),
    Neg(IRValue),
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
    pub is_extern: bool,
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
            is_extern: false,
        };
        func.blocks.insert(0, BasicBlock {
            id: 0,
            instructions: Vec::new(),
        });
        func
    }

    pub fn new_extern(name: String, params: Vec<(String, IRType)>, return_type: IRType) -> Self {
        IRFunction {
            name,
            params,
            return_type,
            blocks: HashMap::new(),
            entry_block: 0,
            next_vreg: 1,
            next_block_id: 1,
            is_extern: true,
        }
    }

    pub fn new_vreg(&mut self) -> IRValue {
        let v = self.next_vreg;
        self.next_vreg += 1;
        v
    }

    pub fn new_block(&mut self) -> BlockID {
        let b = self.next_block_id;
        self.next_block_id += 1;
        self.blocks.insert(b, BasicBlock {
            id: b,
            instructions: Vec::new(),
        });
        b
    }

    pub fn add_inst(&mut self, block: BlockID, inst: IRInstruction) {
        if let Some(b) = self.blocks.get_mut(&block) {
            b.instructions.push(inst);
        }
    }
}

#[derive(Debug, Clone)]
pub struct IRStruct {
    pub name: String,
    pub size: usize,
    pub fields: Vec<(String, IRType, usize)>, // (field_name, field_type, byte_offset)
}

#[derive(Debug, Clone)]
pub struct IRModule {
    pub name: String,
    pub functions: Vec<IRFunction>,
    pub structs: HashMap<String, IRStruct>,
}

impl IRModule {
    pub fn new(name: String) -> Self {
        Self {
            name,
            functions: Vec::new(),
            structs: HashMap::new(),
        }
    }

    pub fn get(&self, name: &str) -> Option<&IRStruct> {
        self.structs.get(name)
    }
}
use std::fmt;

impl fmt::Display for IRType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            IRType::Void => write!(f, "void"),
            IRType::Int(s) => {
                match s {
                    Size::S8 => write!(f, "i8"),
                    Size::S16 => write!(f, "i16"),
                    Size::S32 => write!(f, "i32"),
                    Size::S64 => write!(f, "i64"),
                    Size::S128 => write!(f, "i128"),
                }
            }
            IRType::UInt(s) => {
                match s {
                    Size::S8 => write!(f, "u8"),
                    Size::S16 => write!(f, "u16"),
                    Size::S32 => write!(f, "u32"),
                    Size::S64 => write!(f, "u64"),
                    Size::S128 => write!(f, "u128"),
                }
            }
            IRType::Float(s) => {
                match s {
                    Size::S8 => write!(f, "f8"), // unhappening
                    Size::S16 => write!(f, "f16"), // unhappening
                    Size::S32 => write!(f, "f32"),
                    Size::S64 => write!(f, "f64"),
                    Size::S128 => write!(f, "f128"),
                }
            }
            IRType::Bool => write!(f, "bool"),
            IRType::Char => write!(f, "char"),
            IRType::Byte => write!(f, "byte"),
            IRType::USize => write!(f, "usize"),
            IRType::ISize => write!(f, "isize"),
            IRType::Type => write!(f, "type"),
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
            IROp::AllocArray { elem_ty, size } =>
                write!(f, "alloc_array {} ({} bytes)", elem_ty, size),
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
            IROp::ConstTypeID(v) => write!(f, "const.type_id {}", v),
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
            IROp::BranchIf { cond, true_block, false_block } =>
                write!(f, "br_if v{}, block_{}, block_{}", cond, true_block, false_block),
            IROp::StoreParam { param_idx, ptr } =>
                write!(f, "store_param #{} -> [v{}]", param_idx, ptr),
            IROp::GetFieldPtr { ptr, offset } => write!(f, "get_field_ptr [v{}] + {}", ptr, offset),
            IROp::GetElementPtr { base_ptr, index, elem_size } =>
                write!(f, "get_elem_ptr [v{}] + (v{} * {})", base_ptr, index, elem_size),
            IROp::LoadMemory { ptr, ty } => write!(f, "load_mem [v{}] as {}", ptr, ty),
            IROp::StoreMemory { ptr, value } => write!(f, "store_mem v{} -> [v{}]", value, ptr),
            IROp::Not(v) => write!(f, "not v{}", v),
            IROp::Neg(v) => write!(f, "neg v{}", v),
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
