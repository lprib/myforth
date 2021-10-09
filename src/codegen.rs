use std::{collections::HashMap, os::raw::c_char};

use crate::ast::visitor::{self, CodeBlockVisitor};
use crate::ast::{visitor::ModuleVisitor, FunctionType};
use crate::ast::{
    ConcreteType, FunctionDecl, FunctionHeader, FunctionImpl, IfStatement, Type, WhileStatement,
};

use llvm::core::*;
use llvm::prelude::*;
use llvm::*;
use llvm_sys as llvm;

struct Context<'a> {
    context: LLVMContextRef,
    module: LLVMModuleRef,
    builder: LLVMBuilderRef,
    generated_functions: HashMap<String, LLVMValueRef>,
    functions: &'a HashMap<String, FunctionType>,
}

impl<'a> Context<'a> {
    pub(super) unsafe fn get_llvm_type(&mut self, typ: &Type) -> LLVMTypeRef {
        match typ {
            Type::Concrete(concrete_type) => match concrete_type {
                ConcreteType::I32 => LLVMInt32TypeInContext(self.context),
                ConcreteType::F32 => LLVMBFloatTypeInContext(self.context),
            },
            Type::Generic(_) => todo!(),
            Type::Pointer(_) => todo!(),
        }
    }

    pub(super) unsafe fn get_function_type(&mut self, typ: &FunctionType) -> LLVMTypeRef {
        assert!(typ.outputs.len() < 2, "Multiple returns not yet supported");
        let mut param_types = typ
            .inputs
            .iter()
            .map(|t| self.get_llvm_type(t))
            .collect::<Vec<_>>();

        let ret_type = if typ.outputs.len() == 0 {
            LLVMVoidTypeInContext(self.context)
        } else {
            self.get_llvm_type(&typ.outputs[0])
        };

        LLVMFunctionType(
            ret_type,
            param_types.as_mut_ptr(),
            param_types.len() as u32,
            0,
        )
    }

    pub(super) unsafe fn create_function_decl(&mut self, head: &FunctionHeader, is_extern: bool) -> LLVMValueRef {
        let function_type = self.get_function_type(&head.typ);
        let mut function_name = head.name.clone();
        let new_function = LLVMAddFunction(self.module, function_name.c_str(), function_type);
        if !is_extern {
            LLVMSetLinkage(new_function, LLVMLinkage::LLVMInternalLinkage);
        }
        self.generated_functions
            .insert(String::from(&head.name), new_function);
        new_function
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
                    module: LLVMModuleCreateWithName("main_module\0".c_str()),
                    builder: LLVMCreateBuilderInContext(context),
                    generated_functions: HashMap::new(),
                    functions,
                },
            }
        }
    }
}

impl<'a> ModuleVisitor for ModuleCodeGen<'a> {
    fn visit_decl(&mut self, f_decl: &FunctionDecl) {
        if !f_decl.is_intrinsic {
            unsafe {
                self.context.create_function_decl(&f_decl.head, f_decl.is_extern);
            }
        }
    }

    fn visit_impl(&mut self, f_impl: &FunctionImpl) {
        let mut codegen = FunctionCodeGen::new(&mut self.context, &f_impl.head);
        visitor::walk_code_block(&mut codegen, &f_impl.body);
    }

    fn finalize(&mut self) {
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

            let entry_block =
                LLVMAppendBasicBlockInContext(context.context, function, "entry\0".c_str());
            LLVMPositionBuilderAtEnd(context.builder, entry_block);
        }
        Self {
            context,
            head,
            value_stack: Vec::new(),
        }
    }
}

impl CodeBlockVisitor for FunctionCodeGen<'_, '_> {
    fn visit_i32_literal(&mut self, n: i32) {
        unsafe {
            self.value_stack.push(LLVMConstInt(
                self.context
                    .get_llvm_type(&Type::Concrete(ConcreteType::I32)),
                // TODO negatives will be broken here:
                n as u64,
                false as i32,
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

    fn visit_function(&mut self, name: &str) {
        unsafe {
            if !try_append_intrinsic(self.context, name, &mut self.value_stack) {
                let call_type = &self.context.functions[name];
                let mut args = Vec::new();
                for call_arg in call_type.inputs.iter().rev() {
                    args.push(self.value_stack.pop().unwrap());
                }
                // let call_args = self.value_stack.pop
                LLVMBuildCall(
                    self.context.builder,
                    self.context.generated_functions[name],
                    args.as_mut_ptr(),
                    args.len() as u32,
                    "\0".c_str(),
                );
            }
        }
    }

    fn visit_if_statement(&mut self, statment: &IfStatement) {
        todo!()
    }

    fn visit_while_statement(&mut self, statment: &WhileStatement) {
        todo!()
    }

    fn finalize(&mut self) {
        assert!(
            self.head.typ.outputs.len() < 2,
            "multiple returns not yet supported"
        );
        unsafe {
            if self.head.typ.outputs.is_empty() {
                LLVMBuildRetVoid(self.context.builder);
            } else {
                LLVMBuildRet(self.context.builder, self.value_stack.pop().unwrap());
            }
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
