use std::ptr;
use std::{collections::HashMap, os::raw::c_char};

use crate::ast::visitor::CodeBlockVisitor;
use crate::ast::{visitor::ModuleVisitor, FunctionType};
use crate::ast::{
    ConcreteType, FunctionDecl, FunctionHeader, FunctionImpl, IfStatement, Type, WhileStatement,
};

use llvm::analysis::LLVMVerifyFunction;
use llvm::core::*;
use llvm::prelude::*;
use llvm::*;
use llvm_sys as llvm;

#[derive(Clone, Copy)]
struct GeneratedFunction {
    function_value: LLVMValueRef,
    return_type: LLVMTypeRef,
}

struct Context<'a> {
    context: LLVMContextRef,
    module: LLVMModuleRef,
    builder: LLVMBuilderRef,
    generated_functions: HashMap<String, GeneratedFunction>,
    functions: &'a HashMap<String, FunctionType>,
}

impl<'a> Context<'a> {
    pub(super) unsafe fn get_llvm_type(&mut self, typ: &Type) -> LLVMTypeRef {
        match typ {
            Type::Concrete(concrete_type) => match concrete_type {
                ConcreteType::I32 => LLVMInt32TypeInContext(self.context),
                ConcreteType::F32 => LLVMBFloatTypeInContext(self.context),
                ConcreteType::Bool => LLVMInt1TypeInContext(self.context),
            },
            Type::Generic(_) => todo!(),
            Type::Pointer(_) => todo!("Keep track of reified generic values from typechecking"),
        }
    }

    pub(super) unsafe fn get_function_type(
        &mut self,
        typ: &FunctionType,
        return_type: LLVMTypeRef,
    ) -> LLVMTypeRef {
        let mut param_types = typ
            .inputs
            .iter()
            .map(|t| self.get_llvm_type(t))
            .collect::<Vec<_>>();

        LLVMFunctionType(
            return_type,
            param_types.as_mut_ptr(),
            param_types.len() as u32,
            0,
        )
    }

    pub(super) unsafe fn create_return_type(&mut self, head: &FunctionHeader) -> LLVMTypeRef {
        match &head.typ.outputs.len() {
            0 => LLVMVoidTypeInContext(self.context),
            1 => self.get_llvm_type(&head.typ.outputs[0]),
            _ => {
                let mut ret_type_name = String::from(&head.name);
                ret_type_name.push_str("_output");
                let ret_type_name = ret_type_name.c_str();
                // Create return structure
                let ret_struct = LLVMStructCreateNamed(self.context, ret_type_name);

                // Fill return structure
                let mut ret_types = Vec::new();
                for ret_type in &head.typ.outputs {
                    let ret_struct_member = self.get_llvm_type(ret_type);
                    ret_types.push(ret_struct_member);
                }

                LLVMStructSetBody(
                    ret_struct,
                    ret_types.as_mut_ptr(),
                    head.typ.outputs.len() as u32,
                    false as LLVMBool,
                );

                ret_struct
            }
        }
    }

    pub(super) unsafe fn create_function_decl(
        &mut self,
        head: &FunctionHeader,
        is_extern: bool,
    ) -> GeneratedFunction {
        let return_type = self.create_return_type(head);
        let function_type = self.get_function_type(&head.typ, return_type);

        let mut function_name = head.name.clone();
        let function_value = LLVMAddFunction(self.module, function_name.c_str(), function_type);
        if !is_extern {
            LLVMSetLinkage(function_value, LLVMLinkage::LLVMPrivateLinkage);
        }

        self.generated_functions.insert(
            String::from(&head.name),
            GeneratedFunction {
                function_value,
                return_type,
            },
        );

        self.generated_functions[&head.name]
    }
}

pub struct ModuleCodeGen<'a> {
    context: Context<'a>,
}

impl<'a> ModuleCodeGen<'a> {
    pub fn new(functions: &'a HashMap<String, FunctionType>) -> Self {
        unsafe {
            let context = LLVMContextCreate();
            Self {
                context: Context {
                    context,
                    module: LLVMModuleCreateWithNameInContext("main_module\0".c_str(), context),
                    builder: LLVMCreateBuilderInContext(context),
                    generated_functions: HashMap::new(),
                    functions,
                },
            }
        }
    }
}

impl<'a> ModuleVisitor<()> for ModuleCodeGen<'a> {
    fn visit_decl(&mut self, f_decl: &FunctionDecl) {
        if !f_decl.is_intrinsic {
            unsafe {
                self.context
                    .create_function_decl(&f_decl.head, f_decl.is_extern);
            }
        }
    }

    fn visit_impl(&mut self, f_impl: &FunctionImpl) {
        FunctionCodeGen::new(&mut self.context, &f_impl.head).walk(&f_impl.body);
    }

    fn finalize(self) {
        unsafe {
            LLVMDumpModule(self.context.module);
        }
    }
}

struct FunctionCodeGen<'a, 'b> {
    context: &'a mut Context<'b>,
    head: &'a FunctionHeader,
    value_stack: Vec<LLVMValueRef>,
}

impl<'a, 'b> FunctionCodeGen<'a, 'b> {
    fn new(context: &'a mut Context<'b>, head: &'a FunctionHeader) -> Self {
        unsafe {
            let function = if context.generated_functions.contains_key(&head.name) {
                context.generated_functions[&head.name]
            } else {
                context.create_function_decl(head, false)
            };

            let entry_block = LLVMAppendBasicBlockInContext(
                context.context,
                function.function_value,
                "entry\0".c_str(),
            );
            LLVMPositionBuilderAtEnd(context.builder, entry_block);

            // initial value_stack is the parameters passed to the function
            let mut params: Vec<LLVMValueRef> = vec![ptr::null_mut(); head.typ.inputs.len()];
            let params_ptr = params.as_mut_ptr();
            LLVMGetParams(function.function_value, params_ptr);
            let value_stack = params;

            Self {
                context,
                head,
                value_stack,
            }
        }
    }
}

impl CodeBlockVisitor<()> for FunctionCodeGen<'_, '_> {
    fn visit_i32_literal(&mut self, n: i32) {
        unsafe {
            self.value_stack.push(LLVMConstInt(
                self.context
                    .get_llvm_type(&Type::Concrete(ConcreteType::I32)),
                // TODO negatives will be broken here:
                n as u64,
                false as LLVMBool,
            ))
        }
    }

    fn visit_f32_literal(&mut self, n: f32) {
        unsafe {
            self.value_stack.push(LLVMConstReal(
                self.context
                    .get_llvm_type(&Type::Concrete(ConcreteType::F32)),
                n as f64,
            ))
        }
    }

    fn visit_bool_literal(&mut self, n: bool) {
        unsafe {
            self.value_stack.push(LLVMConstInt(
                self.context
                    .get_llvm_type(&Type::Concrete(ConcreteType::Bool)),
                if n { 1 } else { 0 },
                false as LLVMBool,
            ))
        }
    }

    // send the top n value_stack items to the function.
    // The result will either be void (no effect), single return (push return to value stack),
    // or multiple return (create return struct value, unpack all inner values to value_stack)
    fn visit_function(&mut self, name: &str) {
        unsafe {
            if !try_append_intrinsic(self.context, name, &mut self.value_stack) {
                let call_type = &self.context.functions[name];
                let mut args = Vec::new();
                for _ in 0..call_type.inputs.len() {
                    args.push(self.value_stack.pop().unwrap());
                }
                // let call_args = self.value_stack.pop
                let result = LLVMBuildCall(
                    self.context.builder,
                    self.context.generated_functions[name].function_value,
                    args.as_mut_ptr(),
                    args.len() as u32,
                    "\0".c_str(),
                );

                match call_type.outputs.len() {
                    0 => {}
                    1 => self.value_stack.push(result),
                    _ => {
                        for i in 0..call_type.outputs.len() {
                            let ret_extracted = LLVMBuildExtractValue(
                                self.context.builder,
                                result,
                                i as u32,
                                "\0".c_str(),
                            );
                            self.value_stack.push(ret_extracted);
                        }
                    }
                }
            }
        }
    }

    fn visit_if_statement(&mut self, _statment: &IfStatement) {
        todo!()
    }

    fn visit_while_statement(&mut self, _statment: &WhileStatement) {
        todo!()
    }

    fn finalize(mut self) {
        unsafe {
            match self.head.typ.outputs.len() {
                0 => LLVMBuildRetVoid(self.context.builder),
                1 => LLVMBuildRet(self.context.builder, self.value_stack.pop().unwrap()),
                _ => {
                    // Pack return into return struct
                    let ret_struct = LLVMConstNamedStruct(
                        self.context.generated_functions[&self.head.name].return_type,
                        self.value_stack.as_mut_ptr(),
                        self.head.typ.outputs.len() as u32,
                    );
                    LLVMBuildRet(self.context.builder, ret_struct)
                }
            };
            LLVMVerifyFunction(
                self.context.generated_functions[&self.head.name].function_value,
                analysis::LLVMVerifierFailureAction::LLVMPrintMessageAction,
            );
        }
    }
}

unsafe fn try_append_intrinsic(
    context: &mut Context,
    name: &str,
    value_stack: &mut Vec<LLVMValueRef>,
) -> bool {
    match name {
        "+" => binop_intrinsic(LLVMBuildAdd, value_stack, context),
        "-" => binop_intrinsic(LLVMBuildSub, value_stack, context),
        "*" => binop_intrinsic(LLVMBuildMul, value_stack, context),
        "=" => icmp_intrinsic(LLVMIntPredicate::LLVMIntEQ, value_stack, context),
        "!=" => icmp_intrinsic(LLVMIntPredicate::LLVMIntNE, value_stack, context),
        ">" => icmp_intrinsic(LLVMIntPredicate::LLVMIntSGT, value_stack, context),
        "<" => icmp_intrinsic(LLVMIntPredicate::LLVMIntSLT, value_stack, context),
        ">=" => icmp_intrinsic(LLVMIntPredicate::LLVMIntSGE, value_stack, context),
        "<=" => icmp_intrinsic(LLVMIntPredicate::LLVMIntSLE, value_stack, context),
        "swap" => {
            let a = value_stack.pop().unwrap();
            let b = value_stack.pop().unwrap();
            value_stack.push(a);
            value_stack.push(b);
            true
        }
        "dup" => {
            value_stack.push(*value_stack.last().unwrap());
            true
        }
        "drop" => {
            value_stack.pop().unwrap();
            true
        }
        // TODO implementh `nth` function for dereferenceing into an array. Need to keep reified
        // generic types around from the typechecking stage so we know what types are being indexed
        // (to generate correct getelementptr).
        _ => false,
    }
}

type LLVMBuildBinopFn =
    unsafe extern "C" fn(LLVMBuilderRef, LLVMValueRef, LLVMValueRef, *const c_char) -> LLVMValueRef;

unsafe fn binop_intrinsic(
    f: LLVMBuildBinopFn,
    value_stack: &mut Vec<LLVMValueRef>,
    context: &mut Context,
) -> bool {
    let rhs = value_stack.pop().unwrap();
    let lhs = value_stack.pop().unwrap();
    let new = f(context.builder, lhs, rhs, "\0".c_str());
    value_stack.push(new);
    true
}

unsafe fn icmp_intrinsic(
    predicate: LLVMIntPredicate,
    value_stack: &mut Vec<LLVMValueRef>,
    context: &mut Context,
) -> bool {
    let rhs = value_stack.pop().unwrap();
    let lhs = value_stack.pop().unwrap();
    let new = LLVMBuildICmp(context.builder, predicate, lhs, rhs, "\0".c_str());
    value_stack.push(new);
    true
}

trait ToCStr {
    fn c_str(self) -> *const c_char;
}

impl ToCStr for &'static str {
    /// NOTE: this implementation needs a '\0' manually added to the end
    fn c_str(self) -> *const c_char {
        self.as_bytes().as_ptr() as *const c_char
    }
}

impl ToCStr for &mut String {
    /// NOTE: this implementation automatically adds zero terminator
    fn c_str(self) -> *const c_char {
        self.push('\0');
        self.as_bytes().as_ptr() as *const c_char
    }
}
