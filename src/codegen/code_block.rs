use std::os::raw::c_char;

use crate::ast::visitor::CodeBlockVisitor;
use crate::ast::{ConcreteType, IfStatement, Type, WhileStatement};

use llvm::core::*;
use llvm::prelude::*;
use llvm::*;
use llvm_sys as llvm;

use super::{Context, ToCStr};

pub(super) struct CodeBlockCodeGen<'a, 'b> {
    context: &'a mut Context<'b>,
    containing_function: LLVMValueRef,
    value_stack: Vec<LLVMValueRef>,
    final_bb: LLVMBasicBlockRef,
}

impl<'a, 'b> CodeBlockCodeGen<'a, 'b> {
    pub(super) fn new(
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

            let true_bb = LLVMAppendBasicBlockInContext(
                self.context.llvm_context,
                self.containing_function,
                "if_true_branch\0".c_str(),
            );
            let false_bb = LLVMAppendBasicBlockInContext(
                self.context.llvm_context,
                self.containing_function,
                "if_false_branch\0".c_str(),
            );
            let end_bb = LLVMAppendBasicBlockInContext(
                self.context.llvm_context,
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
