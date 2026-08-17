use crate::middle_end::ir::instruction::*;
use cranelift::prelude::*;
use cranelift_jit::{JITBuilder, JITModule};
use cranelift_module::{default_libcall_names, Linkage, Module};
use std::collections::HashMap;
//pub mod aot;
pub struct CraneliftBackend {
    module: JITModule,
    builder_context: FunctionBuilderContext,
    ctx: codegen::Context,
}

impl CraneliftBackend {
    pub fn new() -> Self {
        let mut flag_builder = settings::builder();
        flag_builder.set("use_colocated_libcalls", "false").unwrap();
        flag_builder.set("is_pic", "false").unwrap();

        let isa_builder = cranelift_native::builder().unwrap_or_else(|msg| {
            panic!("host machine is not supported: {}", msg);
        });
        let isa = isa_builder
            .finish(settings::Flags::new(flag_builder))
            .unwrap();

        let builder = JITBuilder::with_isa(isa, cranelift_module::default_libcall_names());
        let mut module = JITModule::new(builder);
        let ctx = module.make_context();

        Self {
            module,
            builder_context: FunctionBuilderContext::new(),
            ctx,
        }
    }

    pub fn compile_module(&mut self, ir_module: &IRModule) {
        for func in &ir_module.functions {
            self.compile_function(func);
        }
    }

    fn compile_function(&mut self, ir_func: &IRFunction) {
        // Clear the context for the new function
        self.ctx.clear();

        println!(
            "CRANELIFT: Compiling function '{}' to CLIF...",
            ir_func.name
        );

        for (_name, ty) in &ir_func.params {
            self.ctx
                .func
                .signature
                .params
                .push(cranelift::prelude::AbiParam::new(Self::map_type(ty)));
        }
        if ir_func.return_type != IRType::Void {
            self.ctx
                .func
                .signature
                .returns
                .push(cranelift::prelude::AbiParam::new(Self::map_type(
                    &ir_func.return_type,
                )));
        }

        let builder =
            cranelift_frontend::FunctionBuilder::new(&mut self.ctx.func, &mut self.builder_context);
        let mut translator = FunctionTranslator {
            builder,
            module: &mut self.module,
            values: HashMap::new(),
            variables: HashMap::new(),
            blocks: HashMap::new(),
        };

        translator.translate(ir_func);

        println!(
            "CRANELIFT CLIF for '{}':\n{}",
            ir_func.name,
            self.ctx.func.display()
        );
    }

    pub fn finalize(mut self) {
        // self.module.finalize_definitions().unwrap();
        println!("CRANELIFT: Module Finalized. Ready for JIT Execution.");
    }

    fn map_type(ty: &IRType) -> Type {
        match ty {
            IRType::Int32 => types::I32,
            IRType::Int64 => types::I64,
            IRType::Float32 => types::F32,
            IRType::Float64 => types::F64,
            IRType::Bool => types::I8,
            IRType::Pointer(_) => types::I64,
            IRType::Array(_) => types::I64,
            IRType::Object(_) | IRType::CustomScope(_) => types::I64, // Pointers to objects
            _ => types::I64,
        }
    }
}
use cranelift_frontend::{FunctionBuilder, Variable};

struct FunctionTranslator<'a> {
    builder: FunctionBuilder<'a>,
    module: &'a mut JITModule,
    values: HashMap<IRValue, Value>,
    variables: HashMap<IRValue, Variable>,
    blocks: HashMap<BlockID, Block>,
}

impl<'a> FunctionTranslator<'a> {
    fn translate(&mut self, ir_func: &IRFunction) {
        // 1. Create Cranelift Blocks
        for &block_id in ir_func.blocks.keys() {
            let cl_block = self.builder.create_block();
            self.blocks.insert(block_id, cl_block);
        }

        // 2. Setup Entry Block & Params
        let entry_block = self.blocks[&ir_func.entry_block];
        self.builder
            .append_block_params_for_function_params(entry_block);
        self.builder.switch_to_block(entry_block);
        self.builder.seal_block(entry_block); // Seal if it has no predecessors (entry block usually has none)

        for (i, (_name, _ty)) in ir_func.params.iter().enumerate() {
            let val = self.builder.block_params(entry_block)[i];
            // If the parameter is treated as a local variable, we need an alloc for it.
            // But right now we'll just handle basic IR operations.
        }

        // 3. Translate Instructions
        let mut sorted_blocks: Vec<&BlockID> = ir_func.blocks.keys().collect();
        sorted_blocks.sort();

        for &b_id in sorted_blocks {
            let cl_block = self.blocks[&b_id];
            if b_id != ir_func.entry_block {
                self.builder.switch_to_block(cl_block);
                self.builder.seal_block(cl_block); // Simplification: assuming no complex CFG loops for now
            }

            for inst in &ir_func.blocks[&b_id].instructions {
                self.translate_inst(inst);
            }
        }

        // We skip builder.finalize() for now to avoid TargetFrontendConfig issues
    }

    fn translate_inst(&mut self, inst: &IRInstruction) {
        let res = match &inst.op {
            IROp::Alloc { ty } => {
                let cl_ty = CraneliftBackend::map_type(ty);
                let var = self.builder.declare_var(cl_ty);
                self.variables.insert(inst.id.unwrap(), var);
                None
            }
            IROp::ConstInt32(v) => Some(self.builder.ins().iconst(types::I32, *v as i64)),
            IROp::ConstInt64(v) => Some(self.builder.ins().iconst(types::I64, *v)),
            IROp::Store { ptr, value } => {
                let var = self.variables[ptr];
                let val = self.values[value];
                self.builder.def_var(var, val);
                None
            }
            IROp::Load { ptr, .. } => {
                let var = self.variables[ptr];
                Some(self.builder.use_var(var))
            }
            IROp::Add(l, r) => Some(self.builder.ins().iadd(self.values[l], self.values[r])),
            IROp::Sub(l, r) => Some(self.builder.ins().isub(self.values[l], self.values[r])),
            IROp::Mul(l, r) => Some(self.builder.ins().imul(self.values[l], self.values[r])),
            IROp::Return(Some(v)) => {
                self.builder.ins().return_(&[self.values[v]]);
                None
            }
            IROp::Return(None) => {
                self.builder.ins().return_(&[]);
                None
            }
            IROp::Call { func, args } => {
                // Ignore Call for a moment since it requires declaring functions in the module first
                println!("CRANELIFT: Skipping call to {} for now", func);
                None
            }
            _ => {
                println!("CRANELIFT: Unimplemented op {:?}", inst.op);
                None
            }
        };

        if let (Some(val), Some(id)) = (res, inst.id) {
            self.values.insert(id, val);
        }
    }
}
