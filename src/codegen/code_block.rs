use crate::ast::visitor::CodeBlockVisitor;
use crate::ast::{ConcreteType, FunctionCall, IfStatement, Type, WhileStatement};

use llvm::core::*;
use llvm::prelude::*;
use llvm_sys as llvm;

use super::intrinsics::try_append_intrinsic;
use super::{CompilationStack, Context, ToCStr};

pub(super) struct CodeBlockCodeGen<'a, 'b> {
    context: &'a mut Context<'b>,

    // Used to generate new BasicBlocks
    containing_function: LLVMValueRef,

    // Stack of LLVM values and reified types, this is mutated by this function and returned after
    // AST visiting
    stack: CompilationStack,

    // Keep track of the final BasicBlock if this codeblock contains jumps. This is so the
    // potential containing function or CodeBlock can create PHI nodes which have this CodeBlock as
    // an incoming control flow block.
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
                // Pop the required number of arguments off the compilation stack
                for _ in 0..call_type.inputs.len() {
                    args.push(self.stack.pop().unwrap().0);
                }

                // generate function call
                let result = LLVMBuildCall(
                    self.context.builder,
                    self.context.generated_functions[&function.name].function_value,
                    args.as_mut_ptr(),
                    args.len() as u32,
                    "\0".c_str(),
                );

                match call_type.outputs.len() {
                    0 => {}
                    // If single return, we just need to push the return value to stack
                    1 => self.stack.push((
                        result,
                        function.reified_type.as_ref().unwrap().outputs[0].clone(),
                    )),
                    // if multiple return, we need to unpack the return values from the return
                    // struct and push them to the stack individually
                    _ => {
                        for i in 0..call_type.outputs.len() {
                            let ret_extracted = LLVMBuildExtractValue(
                                self.context.builder,
                                result,
                                i as u32,
                                "\0".c_str(),
                            );
                            self.stack.push((
                                ret_extracted,
                                function.reified_type.as_ref().unwrap().outputs[i].clone(),
                            ));
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

            // Branch to true or false block depending on predicate
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
            // Need to branch to exit block after true branch executes
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
            // Need to branch to exit block after false branch executes
            LLVMBuildBr(self.context.builder, end_bb);

            LLVMPositionBuilderAtEnd(self.context.builder, end_bb);
            let mut output_stack = Vec::new();
            for (true_stackval, false_stackval) in true_output_stack
                .into_iter()
                .zip(false_output_stack.into_iter())
            {
                let mut true_stackval = true_stackval;
                let mut false_stackval = false_stackval;
                if true_stackval == false_stackval {
                    // Both branches didn't touch this value, so no phi node required
                    output_stack.push(true_stackval);
                } else {
                    // Branches differ in how they computed the stack value, so create phi node to merge

                    // typechecking ensures that true_stackval and false_stackval will have the
                    // same type, so we only need to get the type of the true branch's value
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
            // Condition branch can be entered from start of loop (eg the current final BasicBlock
            // before appending while), or from the body of the loop if this is not the first time
            // around the loop. Create PHIs to merge
            let mut condition_phis = Vec::new();
            for mut entry_stackval in self.stack.drain(..) {
                let phi = LLVMBuildPhi(
                    self.context.builder,
                    self.context.get_llvm_type(&entry_stackval.1),
                    "while_phi\0".c_str(),
                );

                // Since the body has not been compiled yet, we dont know what the incoming values
                // from it will be yet. Only add the incoming values from the entry of the loop
                // here.
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
            // if condition is false, goto end, else goto body
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
            // body always jumps back up to check the condition again
            LLVMBuildBr(self.context.builder, condition_bb);

            assert!(condition_phis.len() == body_output_stack.len());
            // Complete the PHI nodes created above, since we now know the stack output of the body.
            for (phi, body_stackval) in condition_phis.iter_mut().zip(body_output_stack.iter_mut())
            {
                LLVMAddIncoming((*phi).0, &mut body_stackval.0, &mut body_final_bb, 1);
            }

            LLVMPositionBuilderAtEnd(self.context.builder, end_bb);
            // The exit can only be jumped to from the condition, so the compilation stack should
            // be whatever the condition block left on the stack.
            self.stack = condition_output_stack;
            self.final_bb = end_bb;
        }
    }

    fn finalize(self) -> (CompilationStack, LLVMBasicBlockRef) {
        (self.stack, self.final_bb)
    }
}
