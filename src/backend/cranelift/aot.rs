use crate::{ frontend::parser::ast::Size, middle_end::ir::instruction::* };
use cranelift::prelude::*;
use cranelift_module::{ Linkage, Module };
use cranelift_object::ObjectModule;
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
        let isa = isa_builder.finish(settings::Flags::new(flag_builder)).unwrap();

        let builder = cranelift_object::ObjectBuilder
            ::new(isa, "fast_lang_module", cranelift_module::default_libcall_names())
            .unwrap();
        let module = ObjectModule::new(builder);
        let ctx = module.make_context();

        Self {
            module,
            builder_context: FunctionBuilderContext::new(),
            ctx,
            funcs: HashMap::new(),
        }
    }

    pub fn compile_module(&mut self, ir_module: &IRModule) {
        let default_conv = self.module.target_config().default_call_conv;
        // 1. Declare all functions first
        for func in &ir_module.functions {
            if self.funcs.contains_key(&func.name) {
                continue;
            }
            let mut sig = self.module.make_signature();
            sig.call_conv = default_conv;
            for (_name, ty) in &func.params {
                sig.params.push(cranelift::prelude::AbiParam::new(Self::map_type(ty)));
            }
            if func.return_type != IRType::Void {
                sig.returns.push(
                    cranelift::prelude::AbiParam::new(Self::map_type(&func.return_type))
                );
            }
            let linkage = if func.is_extern { Linkage::Import } else { Linkage::Export };
            let func_id = self.module.declare_function(&func.name, linkage, &sig).unwrap();
            self.funcs.insert(func.name.clone(), func_id);
        }

        // Declare C runtime helpers: printf
        if !self.funcs.contains_key("printf") {
            let mut printf_sig = self.module.make_signature();
            printf_sig.call_conv = default_conv;
            printf_sig.params.push(cranelift::prelude::AbiParam::new(types::I64)); // format string
            printf_sig.returns.push(cranelift::prelude::AbiParam::new(types::I32));
            let printf_id = self.module
                .declare_function("printf", Linkage::Import, &printf_sig)
                .unwrap();
            self.funcs.insert("printf".to_string(), printf_id);
        }

        // 2. Define all non-extern functions
        let funcs_clone = self.funcs.clone();
        let mut defined = std::collections::HashSet::new();
        for func in &ir_module.functions {
            if !func.is_extern && defined.insert(&func.name) {
                self.compile_function(func, &funcs_clone, ir_module);
                self.module.define_function(funcs_clone[&func.name], &mut self.ctx).unwrap();
            }
        }
    }

    fn compile_function(
        &mut self,
        ir_func: &IRFunction,
        cl_funcs: &HashMap<String, cranelift_module::FuncId>,
        ir_structs: &IRModule
    ) {
        self.ctx.clear();
        self.builder_context = FunctionBuilderContext::new();

        let default_conv = self.module.target_config().default_call_conv;
        self.ctx.func.signature.call_conv = default_conv;

        for (_name, ty) in &ir_func.params {
            self.ctx.func.signature.params.push(
                cranelift::prelude::AbiParam::new(Self::map_type(ty))
            );
        }
        if ir_func.return_type != IRType::Void {
            self.ctx.func.signature.returns.push(
                cranelift::prelude::AbiParam::new(Self::map_type(&ir_func.return_type))
            );
        }

        let builder = cranelift_frontend::FunctionBuilder::new(
            &mut self.ctx.func,
            &mut self.builder_context
        );
        let mut translator = FunctionTranslator {
            builder,
            module: &mut self.module,
            funcs: cl_funcs.clone(),
            values: HashMap::new(),
            variables: HashMap::new(),
            var_types: HashMap::new(),
            blocks: HashMap::new(),
            structs: ir_structs.clone(),
            return_type: ir_func.return_type.clone(),
        };

        translator.translate(ir_func);
    }

    pub fn finalize(self, out_path: &str) {
        let obj = self.module.finish();
        std::fs::write(out_path, obj.emit().unwrap()).unwrap();
    }

    fn map_type(ty: &IRType) -> Type {
        match ty {
            IRType::Int(Size::S8) | IRType::UInt(Size::S8) | IRType::Bool | IRType::Byte =>
                types::I8,
            IRType::Int(Size::S16) | IRType::UInt(Size::S16) => types::I16,
            IRType::Int(Size::S32) | IRType::UInt(Size::S32) | IRType::Char | IRType::Type =>
                types::I32,
            | IRType::Int(Size::S64 | Size::S128)
            | IRType::UInt(Size::S64 | Size::S128)
            | IRType::USize
            | IRType::ISize => types::I64,
            IRType::Float(Size::S32 | Size::S16 | Size::S8) => types::F32,
            IRType::Float(Size::S64 | Size::S128) => types::F64,
            | IRType::Pointer(_)
            | IRType::Array(_)
            | IRType::Object(_)
            | IRType::CustomScope(_)
            | IRType::Generic(_) => types::I64,
            IRType::Void => types::I32,
        }
    }
}
use cranelift_frontend::{ FunctionBuilder, Variable };

struct FunctionTranslator<'a> {
    builder: FunctionBuilder<'a>,
    module: &'a mut ObjectModule,
    funcs: HashMap<String, cranelift_module::FuncId>,
    values: HashMap<IRValue, Value>,
    variables: HashMap<IRValue, Variable>,
    var_types: HashMap<IRValue, Type>,
    blocks: HashMap<BlockID, Block>,
    structs: IRModule,
    return_type: IRType,
}

impl<'a> FunctionTranslator<'a> {
    fn get_block(&mut self, b_id: BlockID) -> Block {
        if let Some(&b) = self.blocks.get(&b_id) {
            b
        } else {
            let b = self.builder.create_block();
            self.blocks.insert(b_id, b);
            b
        }
    }

    fn resolve_val(&mut self, id: IRValue) -> Value {
        if let Some(&val) = self.values.get(&id) {
            val
        } else if let Some(&var) = self.variables.get(&id) {
            self.builder.use_var(var)
        } else {
            self.builder.ins().iconst(types::I64, 0)
        }
    }

    fn coerce_to_type(&mut self, mut val: Value, from: Type, to: Type) -> Value {
        if from == to {
            return val;
        }
        if from.is_int() && to.is_int() {
            let from_bits = from.bits();
            let to_bits = to.bits();
            if from_bits < to_bits {
                self.builder.ins().sextend(to, val)
            } else if from_bits > to_bits {
                self.builder.ins().ireduce(to, val)
            } else {
                val
            }
        } else if from.is_float() && to.is_float() {
            if from == types::F32 && to == types::F64 {
                self.builder.ins().fpromote(to, val)
            } else if from == types::F64 && to == types::F32 {
                self.builder.ins().fdemote(to, val)
            } else {
                val
            }
        } else if from.is_int() && to.is_float() {
            if from == types::I8 || from == types::I16 {
                val = self.builder.ins().sextend(types::I32, val);
            }
            self.builder.ins().fcvt_from_sint(to, val)
        } else if from.is_float() && to.is_int() {
            let int_target = if to.bits() <= 32 { types::I32 } else { types::I64 };
            let converted = self.builder.ins().fcvt_to_sint_sat(int_target, val);
            if int_target != to {
                self.builder.ins().ireduce(to, converted)
            } else {
                converted
            }
        } else {
            val
        }
    }

    fn resolve_and_coerce_pair(&mut self, l: IRValue, r: IRValue) -> (Value, Value) {
        let mut lv = self.resolve_val(l);
        let mut rv = self.resolve_val(r);
        let l_ty = self.builder.func.dfg.value_type(lv);
        let r_ty = self.builder.func.dfg.value_type(rv);
        if l_ty == r_ty {
            return (lv, rv);
        }
        if l_ty == types::F64 && r_ty == types::F32 {
            rv = self.builder.ins().fpromote(types::F64, rv);
            return (lv, rv);
        }
        if r_ty == types::F64 && l_ty == types::F32 {
            lv = self.builder.ins().fpromote(types::F64, lv);
            return (lv, rv);
        }
        if l_ty.is_int() && r_ty.is_float() {
            lv = self.coerce_to_type(lv, l_ty, r_ty);
            return (lv, rv);
        }
        if l_ty.is_float() && r_ty.is_int() {
            rv = self.coerce_to_type(rv, r_ty, l_ty);
            return (lv, rv);
        }
        if l_ty.is_int() && r_ty.is_int() {
            if l_ty.bits() < r_ty.bits() {
                lv = self.builder.ins().sextend(r_ty, lv);
            } else if r_ty.bits() < l_ty.bits() {
                rv = self.builder.ins().sextend(l_ty, rv);
            }
        }
        (lv, rv)
    }

    fn translate(&mut self, ir_func: &IRFunction) {
        // 1. Create Cranelift Blocks
        for &block_id in ir_func.blocks.keys() {
            let cl_block = self.builder.create_block();
            self.blocks.insert(block_id, cl_block);
        }

        // 2. Setup Entry Block & Params
        let entry_block = self.get_block(ir_func.entry_block);
        self.builder.append_block_params_for_function_params(entry_block);

        // 3. Translate Instructions
        let mut sorted_blocks: Vec<&BlockID> = ir_func.blocks.keys().collect();
        sorted_blocks.sort();

        for &b_id in sorted_blocks {
            let cl_block = self.get_block(b_id);
            self.builder.switch_to_block(cl_block);

            let mut terminated = false;
            for inst in &ir_func.blocks[&b_id].instructions {
                if terminated {
                    break;
                }
                if matches!(inst.op, IROp::Return(_) | IROp::Jump(_) | IROp::BranchIf { .. }) {
                    terminated = true;
                }
                self.translate_inst(inst);
            }

            if !terminated {
                if self.return_type == IRType::Void {
                    self.builder.ins().return_(&[]);
                } else {
                    let ret_cl_ty = CraneliftAotBackend::map_type(&self.return_type);
                    let zero = if ret_cl_ty.is_float() {
                        self.builder.ins().f64const(0.0)
                    } else {
                        self.builder.ins().iconst(ret_cl_ty, 0)
                    };
                    self.builder.ins().return_(&[zero]);
                }
            }
        }

        self.builder.seal_all_blocks();
    }

    fn translate_inst(&mut self, inst: &IRInstruction) {
        let res = match &inst.op {
            IROp::Alloc { ty } => {
                let cl_ty = CraneliftAotBackend::map_type(ty);
                let var = self.builder.declare_var(cl_ty);
                self.variables.insert(inst.id.unwrap(), var);
                self.var_types.insert(inst.id.unwrap(), cl_ty);

                if let IRType::CustomScope(name) = ty {
                    if let Some(ir_struct) = self.structs.get(name) {
                        let slot = self.builder.create_sized_stack_slot(
                            cranelift::prelude::StackSlotData::new(
                                cranelift::prelude::StackSlotKind::ExplicitSlot,
                                ir_struct.size as u32,
                                0 // align_shift
                            )
                        );
                        let ptr_val = self.builder.ins().stack_addr(types::I64, slot, 0);
                        self.builder.def_var(var, ptr_val);
                    }
                }
                None
            }
            IROp::AllocArray { elem_ty: _, size } => {
                let var = self.builder.declare_var(types::I64);
                self.variables.insert(inst.id.unwrap(), var);
                self.var_types.insert(inst.id.unwrap(), types::I64);

                let slot = self.builder.create_sized_stack_slot(
                    cranelift::prelude::StackSlotData::new(
                        cranelift::prelude::StackSlotKind::ExplicitSlot,
                        *size as u32,
                        0
                    )
                );
                let ptr_val = self.builder.ins().stack_addr(types::I64, slot, 0);
                self.builder.def_var(var, ptr_val);
                Some(ptr_val)
            }
            IROp::StoreParam { param_idx, ptr } => {
                if let Some(&var) = self.variables.get(ptr) {
                    let entry_b = self.get_block(0);
                    let params = self.builder.block_params(entry_b);
                    if let Some(&val) = params.get(*param_idx) {
                        let actual_ty = self.builder.func.dfg.value_type(val);
                        if let Some(&expected_ty) = self.var_types.get(ptr) {
                            let coerced = self.coerce_to_type(val, actual_ty, expected_ty);
                            self.builder.def_var(var, coerced);
                        } else {
                            self.builder.def_var(var, val);
                        }
                    }
                }
                None
            }
            IROp::ConstInt32(v) => Some(self.builder.ins().iconst(types::I32, *v as i64)),
            IROp::ConstInt64(v) => Some(self.builder.ins().iconst(types::I64, *v)),
            IROp::ConstFloat32(f) => Some(self.builder.ins().f32const(*f)),
            IROp::ConstFloat64(f) => Some(self.builder.ins().f64const(*f)),
            IROp::ConstBool(b) =>
                Some(self.builder.ins().iconst(types::I8, if *b { 1 } else { 0 })),
            IROp::ConstTypeID(id) => Some(self.builder.ins().iconst(types::I32, *id as i64)),
            IROp::ConstString(s) => {
                use cranelift_module::DataDescription;
                let mut data_desc = DataDescription::new();
                let unescaped = s.replace("\\n", "\n").replace("\\t", "\t").replace("\\r", "\r");
                let mut bytes = unescaped.into_bytes();
                bytes.push(0); // Null terminator for C strings
                data_desc.define(bytes.into_boxed_slice());
                let data_id = self.module.declare_anonymous_data(false, false).unwrap();
                self.module.define_data(data_id, &data_desc).unwrap();
                let local_data = self.module.declare_data_in_func(data_id, self.builder.func);
                Some(self.builder.ins().symbol_value(types::I64, local_data))
            }
            IROp::Store { ptr, value } => {
                if let Some(&var) = self.variables.get(ptr) {
                    let val = self.resolve_val(*value);
                    let actual_ty = self.builder.func.dfg.value_type(val);
                    if let Some(&expected_ty) = self.var_types.get(ptr) {
                        let coerced = self.coerce_to_type(val, actual_ty, expected_ty);
                        self.builder.def_var(var, coerced);
                    } else {
                        self.builder.def_var(var, val);
                    }
                }
                None
            }
            IROp::Load { ptr, .. } => {
                Some(self.resolve_val(*ptr))
            }
            IROp::GetFieldPtr { ptr, offset } => {
                let ptr_raw = self.resolve_val(*ptr);
                let ptr_ty = self.builder.func.dfg.value_type(ptr_raw);
                let base_ptr_val = self.coerce_to_type(ptr_raw, ptr_ty, types::I64);
                Some(self.builder.ins().iadd_imm_s(base_ptr_val, *offset as i64))
            }
            IROp::GetElementPtr { base_ptr, index, elem_size } => {
                let base_ptr_raw = self.resolve_val(*base_ptr);
                let base_ty = self.builder.func.dfg.value_type(base_ptr_raw);
                let base_ptr_val = self.coerce_to_type(base_ptr_raw, base_ty, types::I64);
                let idx_val = self.resolve_val(*index);
                let idx_ty = self.builder.func.dfg.value_type(idx_val);
                let idx_i64 = self.coerce_to_type(idx_val, idx_ty, types::I64);
                let offset = self.builder.ins().imul_imm_s(idx_i64, *elem_size as i64);
                Some(self.builder.ins().iadd(base_ptr_val, offset))
            }
            IROp::LoadMemory { ptr, ty } => {
                let cl_ty = CraneliftAotBackend::map_type(ty);
                let ptr_raw = self.resolve_val(*ptr);
                let ptr_ty = self.builder.func.dfg.value_type(ptr_raw);
                let ptr_val = self.coerce_to_type(ptr_raw, ptr_ty, types::I64);
                Some(
                    self.builder
                        .ins()
                        .load(cl_ty, cranelift_codegen::ir::MemFlagsData::new(), ptr_val, 0)
                )
            }
            IROp::StoreMemory { ptr, value } => {
                let ptr_raw = self.resolve_val(*ptr);
                let ptr_ty = self.builder.func.dfg.value_type(ptr_raw);
                let ptr_val = self.coerce_to_type(ptr_raw, ptr_ty, types::I64);
                let val = self.resolve_val(*value);
                self.builder
                    .ins()
                    .store(cranelift_codegen::ir::MemFlagsData::new(), val, ptr_val, 0);
                None
            }
            IROp::Add(l, r) => {
                let (lv, rv) = self.resolve_and_coerce_pair(*l, *r);
                let ty = self.builder.func.dfg.value_type(lv);
                if ty.is_float() {
                    Some(self.builder.ins().fadd(lv, rv))
                } else {
                    Some(self.builder.ins().iadd(lv, rv))
                }
            }
            IROp::Sub(l, r) => {
                let (lv, rv) = self.resolve_and_coerce_pair(*l, *r);
                let ty = self.builder.func.dfg.value_type(lv);
                if ty.is_float() {
                    Some(self.builder.ins().fsub(lv, rv))
                } else {
                    Some(self.builder.ins().isub(lv, rv))
                }
            }
            IROp::Mul(l, r) => {
                let (lv, rv) = self.resolve_and_coerce_pair(*l, *r);
                let ty = self.builder.func.dfg.value_type(lv);
                if ty.is_float() {
                    Some(self.builder.ins().fmul(lv, rv))
                } else {
                    Some(self.builder.ins().imul(lv, rv))
                }
            }
            IROp::Div(l, r) => {
                let (lv, rv) = self.resolve_and_coerce_pair(*l, *r);
                let ty = self.builder.func.dfg.value_type(lv);
                if ty.is_float() {
                    Some(self.builder.ins().fdiv(lv, rv))
                } else {
                    Some(self.builder.ins().sdiv(lv, rv))
                }
            }
            IROp::Mod(l, r) => {
                let (lv, rv) = self.resolve_and_coerce_pair(*l, *r);
                Some(self.builder.ins().srem(lv, rv))
            }
            IROp::Neg(v) => {
                let val = self.resolve_val(*v);
                let ty = self.builder.func.dfg.value_type(val);
                if ty.is_float() {
                    Some(self.builder.ins().fneg(val))
                } else {
                    Some(self.builder.ins().ineg(val))
                }
            }
            IROp::Eq(l, r) => {
                let (lv, rv) = self.resolve_and_coerce_pair(*l, *r);
                let ty = self.builder.func.dfg.value_type(lv);
                if ty.is_float() {
                    Some(self.builder.ins().fcmp(FloatCC::Equal, lv, rv))
                } else {
                    Some(self.builder.ins().icmp(IntCC::Equal, lv, rv))
                }
            }
            IROp::Neq(l, r) => {
                let (lv, rv) = self.resolve_and_coerce_pair(*l, *r);
                let ty = self.builder.func.dfg.value_type(lv);
                if ty.is_float() {
                    Some(self.builder.ins().fcmp(FloatCC::NotEqual, lv, rv))
                } else {
                    Some(self.builder.ins().icmp(IntCC::NotEqual, lv, rv))
                }
            }
            IROp::Lt(l, r) => {
                let (lv, rv) = self.resolve_and_coerce_pair(*l, *r);
                let ty = self.builder.func.dfg.value_type(lv);
                if ty.is_float() {
                    Some(self.builder.ins().fcmp(FloatCC::LessThan, lv, rv))
                } else {
                    Some(self.builder.ins().icmp(IntCC::SignedLessThan, lv, rv))
                }
            }
            IROp::Le(l, r) => {
                let (lv, rv) = self.resolve_and_coerce_pair(*l, *r);
                let ty = self.builder.func.dfg.value_type(lv);
                if ty.is_float() {
                    Some(self.builder.ins().fcmp(FloatCC::LessThanOrEqual, lv, rv))
                } else {
                    Some(self.builder.ins().icmp(IntCC::SignedLessThanOrEqual, lv, rv))
                }
            }
            IROp::Gt(l, r) => {
                let (lv, rv) = self.resolve_and_coerce_pair(*l, *r);
                let ty = self.builder.func.dfg.value_type(lv);
                if ty.is_float() {
                    Some(self.builder.ins().fcmp(FloatCC::GreaterThan, lv, rv))
                } else {
                    Some(self.builder.ins().icmp(IntCC::SignedGreaterThan, lv, rv))
                }
            }
            IROp::Ge(l, r) => {
                let (lv, rv) = self.resolve_and_coerce_pair(*l, *r);
                let ty = self.builder.func.dfg.value_type(lv);
                if ty.is_float() {
                    Some(self.builder.ins().fcmp(FloatCC::GreaterThanOrEqual, lv, rv))
                } else {
                    Some(self.builder.ins().icmp(IntCC::SignedGreaterThanOrEqual, lv, rv))
                }
            }
            IROp::And(l, r) => {
                let (lv, rv) = self.resolve_and_coerce_pair(*l, *r);
                Some(self.builder.ins().band(lv, rv))
            }
            IROp::Or(l, r) => {
                let (lv, rv) = self.resolve_and_coerce_pair(*l, *r);
                Some(self.builder.ins().bor(lv, rv))
            }
            IROp::Not(v) => {
                let val = self.resolve_val(*v);
                Some(self.builder.ins().bnot(val))
            }
            IROp::Return(Some(v)) => {
                if self.return_type == IRType::Void {
                    self.builder.ins().return_(&[]);
                } else {
                    let val = self.resolve_val(*v);
                    let actual_ty = self.builder.func.dfg.value_type(val);
                    let expected_cl_ty = CraneliftAotBackend::map_type(&self.return_type);
                    let coerced = self.coerce_to_type(val, actual_ty, expected_cl_ty);
                    self.builder.ins().return_(&[coerced]);
                }
                None
            }
            IROp::Return(None) => {
                self.builder.ins().return_(&[]);
                None
            }
            IROp::Jump(b_id) => {
                let tb = self.get_block(*b_id);
                self.builder.ins().jump(tb, &[]);
                None
            }
            IROp::BranchIf { cond, true_block, false_block } => {
                let cond_val = self.resolve_val(*cond);
                let cond_ty = self.builder.func.dfg.value_type(cond_val);
                let bool_cond = if cond_ty != types::I8 && cond_ty.is_int() {
                    self.builder.ins().icmp_imm_s(IntCC::NotEqual, cond_val, 0)
                } else {
                    cond_val
                };
                let tb = self.get_block(*true_block);
                let fb = self.get_block(*false_block);
                self.builder.ins().brif(bool_cond, tb, &[], fb, &[]);
                None
            }
            IROp::Call { func, args } => {
                if let Some(&func_id) = self.funcs.get(func) {
                    let local_func = self.module.declare_func_in_func(func_id, self.builder.func);
                    let sig_ref = self.builder.func.dfg.ext_funcs[local_func].signature;
                    let expected_params = self.builder.func.dfg.signatures[sig_ref].params.clone();
                    let mut arg_vals = Vec::new();
                    for (idx, a) in args.iter().enumerate() {
                        let val = self.resolve_val(*a);
                        let val_ty = self.builder.func.dfg.value_type(val);
                        let coerced_val = if let Some(param) = expected_params.get(idx) {
                            if func.contains("uint")
                                && val_ty.is_int()
                                && param.value_type.is_int()
                                && val_ty.bits() < param.value_type.bits()
                            {
                                self.builder.ins().uextend(param.value_type, val)
                            } else {
                                self.coerce_to_type(val, val_ty, param.value_type)
                            }
                        } else {
                            val
                        };
                        arg_vals.push(coerced_val);
                    }
                    while arg_vals.len() < expected_params.len() {
                        let expected_ty = expected_params[arg_vals.len()].value_type;
                        let default_val = if expected_ty.is_float() {
                            self.builder.ins().f64const(0.0)
                        } else {
                            self.builder.ins().iconst(expected_ty, 0)
                        };
                        arg_vals.push(default_val);
                    }
                    if arg_vals.len() > expected_params.len() {
                        arg_vals.truncate(expected_params.len());
                    }
                    let call = self.builder.ins().call(local_func, &arg_vals);
                    let results = self.builder.inst_results(call);
                    if results.is_empty() {
                        None
                    } else {
                        Some(results[0])
                    }
                } else {
                    None
                }
            }
        };

        if let (Some(val), Some(id)) = (res, inst.id) {
            self.values.insert(id, val);
        }
    }
}

