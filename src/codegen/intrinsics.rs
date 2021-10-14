use std::os::raw::c_char;

use llvm::core::*;
use llvm::prelude::*;
use llvm::*;
use llvm_sys as llvm;

use crate::ast::ConcreteType;
use crate::ast::Type;

use super::CompilationStack;
use super::CompilationStackValue;
use super::{Context, ToCStr};

pub(super) unsafe fn try_append_intrinsic(
    context: &mut Context,
    name: &str,
    stack: &mut Vec<CompilationStackValue>,
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
        "(i)" => cast_intrinsic(context, stack, Type::Concrete(ConcreteType::I32)),
        "(ui)" => cast_intrinsic(context, stack, Type::Concrete(ConcreteType::U32)),
        "(q)" => cast_intrinsic(context, stack, Type::Concrete(ConcreteType::I64)),
        "(uq)" => cast_intrinsic(context, stack, Type::Concrete(ConcreteType::U64)),
        "(c)" => cast_intrinsic(context, stack, Type::Concrete(ConcreteType::I8)),
        "(uc)" => cast_intrinsic(context, stack, Type::Concrete(ConcreteType::U8)),
        "(f)" => cast_intrinsic(context, stack, Type::Concrete(ConcreteType::F32)),
        "(d)" => cast_intrinsic(context, stack, Type::Concrete(ConcreteType::F64)),
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
    let new = f(context.builder, lhs.llvm_value, rhs.llvm_value, "\0".c_str());
    // ASSUMPTION: binops always have the type 'T 'T -> 'T
    stack.push(CompilationStackValue {
        llvm_value: new,
        typ: rhs.typ,
    });
    true
}

unsafe fn icmp_intrinsic(
    predicate: LLVMIntPredicate,
    stack: &mut CompilationStack,
    context: &mut Context,
) -> bool {
    let rhs = stack.pop().unwrap();
    let lhs = stack.pop().unwrap();
    let new = LLVMBuildICmp(context.builder, predicate, lhs.llvm_value, rhs.llvm_value, "\0".c_str());
    stack.push(CompilationStackValue {
        llvm_value: new,
        typ: Type::Concrete(ConcreteType::Bool),
    });
    true
}

unsafe fn cast_intrinsic(context: &mut Context, stack: &mut CompilationStack, to: Type) -> bool {
    let from = stack.pop().unwrap();
    let opcode = get_cast_opcode(&from.typ, &to);
    if let Some(opcode) = opcode {
        let casted = LLVMBuildCast(
            context.builder,
            opcode,
            from.llvm_value,
            context.get_llvm_type(&to),
            "\0".c_str(),
        );
        stack.push(CompilationStackValue {
            llvm_value: casted,
            typ: to,
        });
    } else {
        stack.push(CompilationStackValue {
            llvm_value: from.llvm_value,
            typ: to,
        });
    }
    true
}

fn get_cast_opcode(from: &Type, to: &Type) -> Option<LLVMOpcode> {
    match (from, to) {
        (Type::Concrete(from), Type::Concrete(to)) => get_cast_opcode_concrete(from, to.clone()),
        (Type::Concrete(_), Type::Pointer(_)) => todo!(),
        (Type::Pointer(_), Type::Concrete(_)) => todo!(),
        (Type::Pointer(_), Type::Pointer(_)) => todo!(),
        (_, _) => panic!("Should not be any generics in codegen stage"),
    }
}

// Casting opcodes for all concrete types
// TODO check if these match the C semantics and if they make sense
fn get_cast_opcode_concrete(from: &ConcreteType, to: ConcreteType) -> Option<LLVMOpcode> {
    match (from.is_integral(), to.is_integral()) {
        (true, true) => {
            if from.width() == to.width() {
                // same width int->int is always no-op
                None
            } else if from.width() < to.width() {
                // extending int->int
                Some(if from.is_signed() {
                    LLVMOpcode::LLVMSExt
                } else {
                    LLVMOpcode::LLVMZExt
                })
            } else {
                // truncating int->int
                Some(LLVMOpcode::LLVMTrunc)
            }
        }
        (true, false) => Some(if from.is_signed() {
            LLVMOpcode::LLVMSIToFP
        } else {
            LLVMOpcode::LLVMUIToFP
        }),
        (false, true) => Some(if to.is_signed() {
            LLVMOpcode::LLVMFPToSI
        } else {
            LLVMOpcode::LLVMFPToUI
        }),
        (false, false) => {
            if from.width() == to.width() {
                None
            } else if from.width() < to.width() {
                Some(LLVMOpcode::LLVMFPExt)
            } else {
                Some(LLVMOpcode::LLVMFPTrunc)
            }
        }
    }
}
