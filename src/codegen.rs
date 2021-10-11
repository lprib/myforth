use std::iter::repeat;
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
        unsafe {
            // If the function already exists in the generated functions, append to it.
            // This is the case if there was a forward declaration.
            let function = if self
                .context
                .generated_functions
                .contains_key(&f_impl.head.name)
            {
                self.context.generated_functions[&f_impl.head.name]
            } else {
                self.context.create_function_decl(&f_impl.head, false)
            };

            let entry_bb = LLVMAppendBasicBlockInContext(
                self.context.context,
                function.function_value,
                "entry\0".c_str(),
            );
            LLVMPositionBuilderAtEnd(self.context.builder, entry_bb);

            // initial value_stack is the parameters passed to the function
            let mut params: Vec<LLVMValueRef> = vec![ptr::null_mut(); f_impl.head.typ.inputs.len()];
            let params_ptr = params.as_mut_ptr();
            LLVMGetParams(function.function_value, params_ptr);

            // Generate the body of the function as a CodeBlock:
            // The code block generation takes the function's parameters as it's initial stack
            let (mut output_stack, _) = CodeBlockCodeGen::new(
                &mut self.context,
                function.function_value,
                params,
                entry_bb,
            )
            .walk(&f_impl.body);

            match f_impl.head.typ.outputs.len() {
                0 => LLVMBuildRetVoid(self.context.builder),
                1 => LLVMBuildRet(self.context.builder, output_stack.pop().unwrap()),
                _ => {
                    // Allocate space for the return struct on stack
                    let return_alloca = LLVMBuildAlloca(
                        self.context.builder,
                        self.context.generated_functions[&f_impl.head.name].return_type,
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
                        LLVMBuildStore(self.context.builder, output_val, output_ptr);
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
                self.context.generated_functions[&f_impl.head.name].function_value,
                analysis::LLVMVerifierFailureAction::LLVMPrintMessageAction,
            );
        }
    }

    fn finalize(self) {
        unsafe {
            LLVMDumpModule(self.context.module);
        }
    }
}

struct CodeBlockCodeGen<'a, 'b> {
    context: &'a mut Context<'b>,
    containing_function: LLVMValueRef,
    value_stack: Vec<LLVMValueRef>,
    final_bb: LLVMBasicBlockRef,
}

impl<'a, 'b> CodeBlockCodeGen<'a, 'b> {
    fn new(
        context: &'a mut Context<'b>,
        containing_function: LLVMValueRef,
        value_stack: Vec<LLVMValueRef>,
        current_final_bb: LLVMBasicBlockRef,
    ) -> Self {
        Self {
            context,
            containing_function,
            value_stack,
            final_bb: current_final_bb,
        }
    }
}

// finalize returns (stack, final BasicBlock)
impl CodeBlockVisitor<(Vec<LLVMValueRef>, LLVMBasicBlockRef)> for CodeBlockCodeGen<'_, '_> {
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

    fn visit_if_statement(&mut self, statement: &IfStatement) {
        unsafe {
            let predicate = self.value_stack.pop().unwrap();

            // let if_entry_bb = LLVMAppendBasicBlockInContext(
            //     self.context.context,
            //     self.containing_function,
            //     "if_entry\0".c_str(),
            // );

            // LLVMBuildBr(self.context.builder, if_entry_bb);

            let true_bb = LLVMAppendBasicBlockInContext(
                self.context.context,
                self.containing_function,
                "if_true_branch\0".c_str(),
            );
            let false_bb = LLVMAppendBasicBlockInContext(
                self.context.context,
                self.containing_function,
                "if_false_branch\0".c_str(),
            );
            let end_bb = LLVMAppendBasicBlockInContext(
                self.context.context,
                self.containing_function,
                "if_finish\0".c_str(),
            );

            // LLVMPositionBuilderAtEnd(self.context.builder, if_entry_bb);
            LLVMBuildCondBr(self.context.builder, predicate, true_bb, false_bb);

            LLVMPositionBuilderAtEnd(self.context.builder, true_bb);
            // Generate true block code
            let (true_output_stack, mut true_final_bb) = CodeBlockCodeGen::new(
                self.context,
                self.containing_function,
                self.value_stack.to_vec(),
                true_bb,
            )
            .walk(&statement.true_branch);
            LLVMBuildBr(self.context.builder, end_bb);

            LLVMPositionBuilderAtEnd(self.context.builder, false_bb);
            // Generate false block code
            let (false_output_stack, mut false_final_bb) = CodeBlockCodeGen::new(
                self.context,
                self.containing_function,
                self.value_stack.to_vec(),
                false_bb,
            )
            .walk(&statement.false_branch);
            LLVMBuildBr(self.context.builder, end_bb);

            LLVMPositionBuilderAtEnd(self.context.builder, end_bb);
            let mut output_stack = Vec::new();
            for (true_stackval, false_stackval) in
                true_output_stack.iter().zip(false_output_stack.iter())
            {
                let mut true_stackval = *true_stackval;
                let mut false_stackval = *false_stackval;
                if true_stackval == false_stackval {
                    // Both branches didn't touch this value
                    output_stack.push(true_stackval);
                } else {
                    // Branches differ in how they computed the stackval

                    // typechecking ensures that true_stackval and false_stackval will have the
                    // same type, so we only need to get the type of the true branch's value
                    let output_type = LLVMTypeOf(true_stackval);
                    let phi = LLVMBuildPhi(self.context.builder, output_type, "\0".c_str());
                    LLVMAddIncoming(phi, &mut true_stackval, &mut true_final_bb, 1);
                    LLVMAddIncoming(phi, &mut false_stackval, &mut false_final_bb, 1);
                    output_stack.push(phi);
                }
            }
            self.final_bb = end_bb;
            self.value_stack = output_stack;
        }
    }

    fn visit_while_statement(&mut self, _statement: &WhileStatement) {
        todo!()
    }

    fn finalize(self) -> (Vec<LLVMValueRef>, LLVMBasicBlockRef) {
        (self.value_stack, self.final_bb)
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
