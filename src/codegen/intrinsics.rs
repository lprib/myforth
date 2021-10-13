use std::os::raw::c_char;

use llvm::core::*;
use llvm::prelude::*;
use llvm::*;
use llvm_sys as llvm;

use crate::ast::ConcreteType;
use crate::ast::Type;

use super::CompilationStack;
use super::{Context, ToCStr};

pub(super) unsafe fn try_append_intrinsic(
    context: &mut Context,
    name: &str,
    stack: &mut Vec<(LLVMValueRef, Type)>,
) -> bool {
    match name {
        "+" => binop_intrinsic(LLVMBuildAdd, stack, context),
        "-" => binop_intrinsic(LLVMBuildSub, stack, context),
        "*" => binop_intrinsic(LLVMBuildMul, stack, context),
        "/" => binop_intrinsic(LLVMBuildSDiv, stack, context),
        "%" => binop_intrinsic(LLVMBuildSRem, stack, context),
        "<<" => binop_intrinsic(LLVMBuildShl, stack, context),
        ">>" => binop_intrinsic(LLVMBuildAShr, stack, context),
        "=" => icmp_intrinsic(LLVMIntPredicate::LLVMIntEQ, stack, context),
        "!=" => icmp_intrinsic(LLVMIntPredicate::LLVMIntNE, stack, context),
        ">" => icmp_intrinsic(LLVMIntPredicate::LLVMIntSGT, stack, context),
        "<" => icmp_intrinsic(LLVMIntPredicate::LLVMIntSLT, stack, context),
        ">=" => icmp_intrinsic(LLVMIntPredicate::LLVMIntSGE, stack, context),
        "<=" => icmp_intrinsic(LLVMIntPredicate::LLVMIntSLE, stack, context),
        "swap" => {
            let a = stack.pop().unwrap();
            let b = stack.pop().unwrap();
            stack.push(a);
            stack.push(b);
            true
        }
        "over" => {
            let n = stack.len();
            let a = stack[n - 2].clone();
            stack.push(a);
            true
        }
        "rot" => {
            let a = stack.remove(stack.len() - 3);
            stack.push(a);
            true
        }
        "dup" => {
            stack.push(stack.last().unwrap().clone());
            true
        }
        "dup2" => {
            let n = stack.len();
            let a = stack[n - 2].clone();
            let b = stack[n - 1].clone();
            stack.push(a);
            stack.push(b);
            true
        }
        "drop" => {
            stack.pop().unwrap();
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
    stack: &mut CompilationStack,
    context: &mut Context,
) -> bool {
    let rhs = stack.pop().unwrap();
    let lhs = stack.pop().unwrap();
    let new = f(context.builder, lhs.0, rhs.0, "\0".c_str());
    // ASSUMPTION: binops always have the type 'T 'T -> 'T
    stack.push((new, rhs.1));
    true
}

unsafe fn icmp_intrinsic(
    predicate: LLVMIntPredicate,
    value_stack: &mut CompilationStack,
    context: &mut Context,
) -> bool {
    let rhs = value_stack.pop().unwrap();
    let lhs = value_stack.pop().unwrap();
    let new = LLVMBuildICmp(context.builder, predicate, lhs.0, rhs.0, "\0".c_str());
    value_stack.push((new, Type::Concrete(ConcreteType::Bool)));
    true
}
