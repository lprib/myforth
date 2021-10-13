use std::os::raw::c_char;

use llvm::core::*;
use llvm::prelude::*;
use llvm::*;
use llvm_sys as llvm;

use super::{Context, ToCStr};

pub(super) unsafe fn try_append_intrinsic(
    context: &mut Context,
    name: &str,
    value_stack: &mut Vec<LLVMValueRef>,
) -> bool {
    match name {
        "+" => binop_intrinsic(LLVMBuildAdd, value_stack, context),
        "-" => binop_intrinsic(LLVMBuildSub, value_stack, context),
        "*" => binop_intrinsic(LLVMBuildMul, value_stack, context),
        "/" => binop_intrinsic(LLVMBuildSDiv, value_stack, context),
        "%" => binop_intrinsic(LLVMBuildSRem, value_stack, context),
        "<<" => binop_intrinsic(LLVMBuildShl, value_stack, context),
        ">>" => binop_intrinsic(LLVMBuildAShr, value_stack, context),
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
        "over" => {
            let n = value_stack.len();
            let a = value_stack[n - 2];
            value_stack.push(a);
            true
        }
        "rot" => {
            let a = value_stack.remove(value_stack.len() - 3);
            value_stack.push(a);
            true
        }
        "dup" => {
            value_stack.push(*value_stack.last().unwrap());
            true
        }
        "dup2" => {
            let n = value_stack.len();
            let a = value_stack[n - 2];
            let b = value_stack[n - 1];
            value_stack.push(a);
            value_stack.push(b);
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
