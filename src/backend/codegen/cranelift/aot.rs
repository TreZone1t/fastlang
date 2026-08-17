use crate::middle_end::ir::instruction::*;
use cranelift::prelude::*;
use cranelift_module::{default_libcall_names, Linkage, Module};
use cranelift_object::{ObjectBuilder, ObjectModule};
use std::collections::HashMap;

pub struct CraneliftAotBackend {
    module: ObjectModule,
    builder_context: FunctionBuilderContext,
    ctx: codegen::Context,
    funcs: HashMap<String, cranelift_module::FuncId>,
}

impl CraneliftAotBackend {
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

        let builder = cranelift_object::ObjectBuilder::new(
            isa,
            "fast_lang_module",
            cranelift_module::default_libcall_names(),
        )
        .unwrap();
        let mut module = ObjectModule::new(builder);
        let ctx = module.make_context();

        Self {
            module,
            builder_context: FunctionBuilderContext::new(),
            ctx,
            funcs: HashMap::new(),
        }
    }

    pub fn compile_module(&mut self, ir_module: &IRModule) {
        // 1. Declare all functions first
        for func in &ir_module.functions {
            let mut sig = self.module.make_signature();
            for (_name, ty) in &func.params {
                sig.params
                    .push(cranelift::prelude::AbiParam::new(Self::map_type(ty)));
            }
            if func.return_type != IRType::Void {
                sig.returns
                    .push(cranelift::prelude::AbiParam::new(Self::map_type(
                        &func.return_type,
                    )));
            }
            let func_id = self
                .module
                .declare_function(&func.name, Linkage::Export, &sig)
                .unwrap();
            self.funcs.insert(func.name.clone(), func_id);
        }

        // 2. Define all functions
        let funcs_clone = self.funcs.clone();
        for func in &ir_module.functions {
            self.compile_function(func, &funcs_clone, ir_module);
            self.module
                .define_function(funcs_clone[&func.name], &mut self.ctx)
                .unwrap();
        }
    }

    fn compile_function(
        &mut self,
        ir_func: &IRFunction,
        cl_funcs: &HashMap<String, cranelift_module::FuncId>,
        ir_structs: &IRModule,
    ) {
        self.ctx.clear();
        self.builder_context = FunctionBuilderContext::new();

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
            funcs: cl_funcs.clone(),
            values: HashMap::new(),
            variables: HashMap::new(),
            blocks: HashMap::new(),
            structs: ir_structs.clone(),
        };

        translator.translate(ir_func);

        println!(
            "CRANELIFT CLIF for '{}':\n{}",
            ir_func.name,
            self.ctx.func.display()
        );
    }

    pub fn finalize(mut self, out_path: &str) {
        let obj = self.module.finish();
        std::fs::write(out_path, obj.emit().unwrap()).unwrap();
        println!(
            "CRANELIFT AOT: Object file '{}' generated successfully.",
            out_path
        );
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
    module: &'a mut ObjectModule,
    funcs: HashMap<String, cranelift_module::FuncId>,
    values: HashMap<IRValue, Value>,
    variables: HashMap<IRValue, Variable>,
    blocks: HashMap<BlockID, Block>,
    structs: IRModule,
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
                let cl_ty = CraneliftAotBackend::map_type(ty);
                let var = self.builder.declare_var(cl_ty);
                self.variables.insert(inst.id.unwrap(), var);

                if let IRType::CustomScope(name) = ty {
                    if let Some(ir_struct) = self.structs.get(name) {
                        let slot = self.builder.create_sized_stack_slot(
                            cranelift::prelude::StackSlotData::new(
                                cranelift::prelude::StackSlotKind::ExplicitSlot,
                                ir_struct.size as u32,
                                0, // align_shift
                            ),
                        );
                        let ptr_val = self.builder.ins().stack_addr(types::I64, slot, 0);
                        self.builder.def_var(var, ptr_val);
                    }
                }
                None
            }
            IROp::StoreParam { param_idx, ptr } => {
                let var = self.variables[ptr];
                let val = self.builder.block_params(self.blocks[&0])[*param_idx];
                self.builder.def_var(var, val);
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
            IROp::GetFieldPtr { ptr, offset } => {
                let base_ptr_val = self.builder.use_var(self.variables[ptr]);
                Some(self.builder.ins().iadd_imm(base_ptr_val, *offset as i64))
            }
            IROp::LoadMemory { ptr, ty } => {
                let cl_ty = CraneliftAotBackend::map_type(ty);
                let ptr_val = self.values[ptr];
                Some(self.builder.ins().load(
                    cl_ty,
                    cranelift_codegen::ir::MemFlagsData::new(),
                    ptr_val,
                    0,
                ))
            }
            IROp::StoreMemory { ptr, value } => {
                let ptr_val = self.values[ptr];
                let val = self.values[value];
                self.builder.ins().store(
                    cranelift_codegen::ir::MemFlagsData::new(),
                    val,
                    ptr_val,
                    0,
                );
                None
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
            IROp::Jump(b_id) => {
                self.builder.ins().jump(self.blocks[b_id], &[]);
                None
            }
            IROp::BranchIf {
                cond,
                true_block,
                false_block,
            } => {
                let cond_val = self.values[cond];
                self.builder.ins().brif(
                    cond_val,
                    self.blocks[true_block],
                    &[],
                    self.blocks[false_block],
                    &[],
                );
                None
            }
            IROp::Call { func, args } => {
                let func_id = self.funcs[func];
                let local_func = self.module.declare_func_in_func(func_id, self.builder.func);
                let arg_vals: Vec<Value> = args.iter().map(|a| self.values[a]).collect();
                let call = self.builder.ins().call(local_func, &arg_vals);
                let results = self.builder.inst_results(call);
                if results.is_empty() {
                    None
                } else {
                    Some(results[0])
                }
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
