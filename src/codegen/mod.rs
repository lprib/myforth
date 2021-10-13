mod code_block;
pub mod module;

use std::{collections::HashMap, os::raw::c_char};

use crate::ast::{ConcreteType, Type};
use crate::ast::{FunctionHeader, FunctionType};

use llvm::core::*;
use llvm::prelude::*;
use llvm::*;
use llvm_sys as llvm;

#[derive(Clone, Copy)]
pub(super) struct GeneratedFunction {
    function_value: LLVMValueRef,
    return_type: LLVMTypeRef,
}

pub(super) struct Context<'a> {
    llvm_context: LLVMContextRef,
    module: LLVMModuleRef,
    builder: LLVMBuilderRef,
    generated_functions: HashMap<String, GeneratedFunction>,
    functions: &'a HashMap<String, FunctionType>,
}

impl<'a> Context<'a> {
    pub(super) unsafe fn get_llvm_type(&mut self, typ: &Type) -> LLVMTypeRef {
        match typ {
            Type::Concrete(concrete_type) => match concrete_type {
                ConcreteType::I8 | ConcreteType::U8 => LLVMInt8TypeInContext(self.llvm_context),
                ConcreteType::I32 | ConcreteType::U32 => LLVMInt32TypeInContext(self.llvm_context),
                ConcreteType::I64 | ConcreteType::U64=> todo!(),
                ConcreteType::F32 => LLVMFloatTypeInContext(self.llvm_context),
                ConcreteType::F64 => LLVMDoubleTypeInContext(self.llvm_context),
                ConcreteType::Bool => LLVMInt1TypeInContext(self.llvm_context),
            },
            Type::Generic(_) => todo!("Should get the reified type here!"),
            Type::Pointer(inner) => LLVMPointerType(self.get_llvm_type(inner), 0)
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
            0 => LLVMVoidTypeInContext(self.llvm_context),
            1 => self.get_llvm_type(&head.typ.outputs[0]),
            _ => {
                let mut ret_type_name = String::from(&head.name);
                ret_type_name.push_str("_output");
                let ret_type_name = ret_type_name.c_str();
                // Create return structure
                let ret_struct = LLVMStructCreateNamed(self.llvm_context, ret_type_name);

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
