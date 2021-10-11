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
