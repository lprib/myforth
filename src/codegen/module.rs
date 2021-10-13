use std::collections::HashMap;
use std::ffi::CStr;
use std::ptr;

use crate::ast::visitor::CodeBlockVisitor;
use crate::ast::{visitor::ModuleVisitor, FunctionType};
use crate::ast::{FunctionDecl, FunctionImpl};

use llvm::analysis::LLVMVerifyFunction;
use llvm::core::*;
use llvm::prelude::*;
use llvm::*;
use llvm::target_machine::LLVMGetDefaultTargetTriple;
use llvm_sys as llvm;

use super::{code_block::CodeBlockCodeGen, Context, ToCStr};

pub struct ModuleCodeGen<'a> {
    context: Context<'a>,
}

impl<'a> ModuleCodeGen<'a> {
    pub fn new(functions: &'a HashMap<String, FunctionType>) -> Self {
        unsafe {
            let context = LLVMContextCreate();
            let module = LLVMModuleCreateWithNameInContext("main_module\0".c_str(), context);
            let target_triple = LLVMGetDefaultTargetTriple();
            LLVMSetTarget(module, target_triple);
            Self {
                context: Context {
                    llvm_context: context,
                    module,
                    builder: LLVMCreateBuilderInContext(context),
                    generated_functions: HashMap::new(),
                    functions,
                },
            }
        }
    }
}

impl<'a> ModuleVisitor<String> for ModuleCodeGen<'a> {
    fn visit_decl(&mut self, function: &mut FunctionDecl) {
        if !function.is_intrinsic {
            unsafe {
                self.context
                    .create_function_decl(&function.head, function.is_extern);
            }
        }
    }

    fn visit_impl(&mut self, function: &mut FunctionImpl) {
        unsafe {
            // If the function already exists in the generated functions, append to it.
            // This is the case if there was a forward declaration.
            let generated_function = if self
                .context
                .generated_functions
                .contains_key(&function.head.name)
            {
                self.context.generated_functions[&function.head.name]
            } else {
                self.context.create_function_decl(&function.head, false)
            };

            let entry_bb = LLVMAppendBasicBlockInContext(
                self.context.llvm_context,
                generated_function.function_value,
                "entry\0".c_str(),
            );
            LLVMPositionBuilderAtEnd(self.context.builder, entry_bb);

            // initial value_stack is the parameters passed to the function
            let mut params: Vec<LLVMValueRef> =
                vec![ptr::null_mut(); function.head.typ.inputs.len()];
            let params_ptr = params.as_mut_ptr();
            LLVMGetParams(generated_function.function_value, params_ptr);
            let params = params
                .into_iter()
                .zip(function.head.typ.inputs.iter().cloned())
                .collect();

            // Generate the body of the function as a CodeBlock:
            // The code block generation takes the function's parameters as it's initial stack
            let (mut output_stack, _) = CodeBlockCodeGen::new(
                &mut self.context,
                generated_function.function_value,
                params,
                entry_bb,
            )
            .walk(&mut function.body);

            match function.head.typ.outputs.len() {
                0 => LLVMBuildRetVoid(self.context.builder),

                // If only a single return, we can just return the value directly
                1 => LLVMBuildRet(self.context.builder, output_stack.pop().unwrap().0),

                // If multiple returns, must pack all returned stack items into a struct
                _ => {
                    // Allocate space for the return struct on stack
                    let return_alloca = LLVMBuildAlloca(
                        self.context.builder,
                        self.context.generated_functions[&function.head.name].return_type,
                        "return_struct_ptr\0".c_str(),
                    );

                    // Create a GEP and store instruction for each return value
                    for (i, output_val) in output_stack.into_iter().enumerate() {
                        let output_ptr = LLVMBuildStructGEP(
                            self.context.builder,
                            return_alloca,
                            i as u32,
                            "return_value_ptr\0".c_str(),
                        );
                        LLVMBuildStore(self.context.builder, output_val.0, output_ptr);
                    }
                    // Load the populated return structure into a value
                    let return_struct = LLVMBuildLoad(
                        self.context.builder,
                        return_alloca,
                        "return_value\0".c_str(),
                    );

                    // Return the loaded structure
                    LLVMBuildRet(self.context.builder, return_struct)
                }
            };

            LLVMVerifyFunction(
                self.context.generated_functions[&function.head.name].function_value,
                analysis::LLVMVerifierFailureAction::LLVMPrintMessageAction,
            );
        }
    }

    fn finalize(self) -> String {
        unsafe {
            CStr::from_ptr(LLVMPrintModuleToString(self.context.module))
                .to_string_lossy()
                .into_owned()
        }
    }
}
