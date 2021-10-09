use ast::visitor;
use codegen::ModuleCodeGen;
use typecheck::ModuleTypeChecker;

use crate::parser::module;

mod ast;
mod codegen;
mod parser;
mod typecheck;

fn main() {
    let test = "
    extern fn putchar i32 -> ;
    intrinsic fn + i32 i32 -> i32;
    
    fn main -> [
        97 1 + putchar
    ]
    ";
    let module = module(test).unwrap().1;

    let mut type_checker = ModuleTypeChecker::new();
    visitor::walk_module(&mut type_checker, &module);

    let mut codegen = ModuleCodeGen::new(&type_checker.functions);
    visitor::walk_module(&mut codegen, &module);
}
