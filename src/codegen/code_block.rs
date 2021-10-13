use crate::ast::visitor::CodeBlockVisitor;
use crate::ast::{ConcreteType, FunctionCall, IfStatement, Type, WhileStatement};

use llvm::core::*;
use llvm::prelude::*;
use llvm_sys as llvm;

use super::intrinsics::try_append_intrinsic;
use super::{CompilationStack, Context, ToCStr};

pub(super) struct CodeBlockCodeGen<'a, 'b> {
    context: &'a mut Context<'b>,
    containing_function: LLVMValueRef,
    stack: CompilationStack,
    final_bb: LLVMBasicBlockRef,
}

impl<'a, 'b> CodeBlockCodeGen<'a, 'b> {
    pub(super) fn new(
        context: &'a mut Context<'b>,
        containing_function: LLVMValueRef,
        stack: CompilationStack,
        current_final_bb: LLVMBasicBlockRef,
    ) -> Self {
        Self {
            context,
            containing_function,
            stack,
            final_bb: current_final_bb,
        }
    }
}

// finalize returns (stack, final BasicBlock)
impl CodeBlockVisitor<(CompilationStack, LLVMBasicBlockRef)> for CodeBlockCodeGen<'_, '_> {
    fn visit_i32_literal(&mut self, n: i32) {
        unsafe {
            let typ = Type::Concrete(ConcreteType::I32);
            self.stack.push((
                LLVMConstInt(
                    self.context.get_llvm_type(&typ),
                    // TODO negatives will be broken here:
                    n as u64,
                    false as LLVMBool,
                ),
                typ,
            ))
        }
    }

    fn visit_f32_literal(&mut self, n: f32) {
        unsafe {
            let typ = Type::Concrete(ConcreteType::F32);
            self.stack.push((
                LLVMConstReal(self.context.get_llvm_type(&typ), n as f64),
                typ,
            ));
        }
    }

    fn visit_bool_literal(&mut self, n: bool) {
        unsafe {
            let typ = Type::Concrete(ConcreteType::Bool);
            self.stack.push((
                LLVMConstInt(
                    self.context.get_llvm_type(&typ),
                    if n { 1 } else { 0 },
                    false as LLVMBool,
                ),
                typ,
            ));
        }
    }

    fn visit_function(&mut self, function: &mut FunctionCall) {
        unsafe {
            if !try_append_intrinsic(self.context, &function.name, &mut self.stack) {
                let call_type = &self.context.functions[&function.name];
                let mut args = Vec::new();
                for _ in 0..call_type.inputs.len() {
                    args.push(self.stack.pop().unwrap().0);
                }
                // let call_args = self.value_stack.pop
                let result = LLVMBuildCall(
                    self.context.builder,
                    self.context.generated_functions[&function.name].function_value,
                    args.as_mut_ptr(),
                    args.len() as u32,
                    "\0".c_str(),
                );

                match call_type.outputs.len() {
                    0 => {}
                    1 => self
                        .stack
                        .push((result, function.reified_type.as_ref().unwrap().outputs[0].clone())),
                    _ => {
                        for i in 0..call_type.outputs.len() {
                            let ret_extracted = LLVMBuildExtractValue(
                                self.context.builder,
                                result,
                                i as u32,
                                "\0".c_str(),
                            );
                            self.stack
                                .push((ret_extracted, function.reified_type.as_ref().unwrap().outputs[i].clone()));
                        }
                    }
                }
            }
        }
    }

    fn visit_if_statement(&mut self, statement: &mut IfStatement) {
        unsafe {
            let predicate = self.stack.pop().unwrap().0;

            let true_bb = LLVMAppendBasicBlockInContext(
                self.context.llvm_context,
                self.containing_function,
                "if-true-branch\0".c_str(),
            );
            let false_bb = LLVMAppendBasicBlockInContext(
                self.context.llvm_context,
                self.containing_function,
                "if-false-branch\0".c_str(),
            );
            let end_bb = LLVMAppendBasicBlockInContext(
                self.context.llvm_context,
                self.containing_function,
                "if-finish\0".c_str(),
            );

            // LLVMPositionBuilderAtEnd(self.context.builder, if_entry_bb);
            LLVMBuildCondBr(self.context.builder, predicate, true_bb, false_bb);

            LLVMPositionBuilderAtEnd(self.context.builder, true_bb);
            // Generate true block code
            let (true_output_stack, mut true_final_bb) = CodeBlockCodeGen::new(
                self.context,
                self.containing_function,
                self.stack.to_vec(),
                true_bb,
            )
            .walk(&mut statement.true_branch);
            LLVMBuildBr(self.context.builder, end_bb);

            LLVMPositionBuilderAtEnd(self.context.builder, false_bb);
            // Generate false block code
            let (false_output_stack, mut false_final_bb) = CodeBlockCodeGen::new(
                self.context,
                self.containing_function,
                self.stack.to_vec(),
                false_bb,
            )
            .walk(&mut statement.false_branch);
            LLVMBuildBr(self.context.builder, end_bb);

            LLVMPositionBuilderAtEnd(self.context.builder, end_bb);
            let mut output_stack = Vec::new();
            for (true_stackval, false_stackval) in
                true_output_stack.into_iter().zip(false_output_stack.into_iter())
            {
                let mut true_stackval = true_stackval;
                let mut false_stackval = false_stackval;
                if true_stackval == false_stackval {
                    // Both branches didn't touch this value
                    output_stack.push(true_stackval);
                } else {
                    // Branches differ in how they computed the stackval

                    // typechecking ensures that true_stackval and false_stackval will have the
                    // same type, so we only need to get the type of the true branch's value
                    // Get the LLVM type from the types added by typechecking.
                    let output_type = self.context.get_llvm_type(&true_stackval.1);
                    let phi = LLVMBuildPhi(self.context.builder, output_type, "\0".c_str());
                    LLVMAddIncoming(phi, &mut true_stackval.0, &mut true_final_bb, 1);
                    LLVMAddIncoming(phi, &mut false_stackval.0, &mut false_final_bb, 1);
                    output_stack.push((phi, true_stackval.1));
                }
            }
            self.final_bb = end_bb;
            self.stack = output_stack;
        }
    }

    fn visit_while_statement(&mut self, statement: &mut WhileStatement) {
        unsafe {
            let condition_bb = LLVMAppendBasicBlockInContext(
                self.context.llvm_context,
                self.containing_function,
                "while-condition\0".c_str(),
            );
            let body_bb = LLVMAppendBasicBlockInContext(
                self.context.llvm_context,
                self.containing_function,
                "while-body\0".c_str(),
            );
            let end_bb = LLVMAppendBasicBlockInContext(
                self.context.llvm_context,
                self.containing_function,
                "while-finish\0".c_str(),
            );

            LLVMBuildBr(self.context.builder, condition_bb);

            LLVMPositionBuilderAtEnd(self.context.builder, condition_bb);
            let mut condition_phis = Vec::new();
            for mut entry_stackval in self.stack.drain(..) {
                let phi = LLVMBuildPhi(
                    self.context.builder,
                    self.context.get_llvm_type(&entry_stackval.1),
                    "while_phi\0".c_str(),
                );
                LLVMAddIncoming(phi, &mut entry_stackval.0, &mut self.final_bb, 1);
                condition_phis.push((phi, entry_stackval.1));
            }

            let (mut condition_output_stack, condition_final_bb) = CodeBlockCodeGen::new(
                self.context,
                self.containing_function,
                condition_phis.to_vec(),
                condition_bb,
            )
            .walk(&mut statement.condition);

            LLVMPositionBuilderAtEnd(self.context.builder, condition_final_bb);
            LLVMBuildCondBr(
                self.context.builder,
                condition_output_stack.pop().unwrap().0,
                body_bb,
                end_bb,
            );

            LLVMPositionBuilderAtEnd(self.context.builder, body_bb);
            let (mut body_output_stack, mut body_final_bb) = CodeBlockCodeGen::new(
                self.context,
                self.containing_function,
                condition_output_stack.to_vec(),
                body_bb,
            )
            .walk(&mut statement.body);
            LLVMBuildBr(self.context.builder, condition_bb);

            // Complete the phi nodes to merge branches from entry and loop body
            assert!(condition_phis.len() == body_output_stack.len());
            for (phi, body_stackval) in condition_phis.iter_mut().zip(body_output_stack.iter_mut())
            {
                LLVMAddIncoming((*phi).0, &mut body_stackval.0, &mut body_final_bb, 1);
            }

            LLVMPositionBuilderAtEnd(self.context.builder, end_bb);
            self.stack = condition_output_stack;
            self.final_bb = end_bb;
        }
    }

    fn finalize(self) -> (CompilationStack, LLVMBasicBlockRef) {
        (self.stack, self.final_bb)
    }
}
