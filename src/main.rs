use ast::visitor;
use codegen::ModuleCodeGen;
use typecheck::{FunctionMapBuilder, ModuleTypeChecker};

use crate::parser::module;

mod ast;
mod codegen;
mod parser;
mod typecheck;

fn main() {
    let test = "
    extern fn putchar i32 -> ;
    intrinsic fn + i32 i32 -> i32;
    intrinsic fn - i32 i32 -> i32;
    intrinsic fn * i32 i32 -> i32;
    
    fn main -> [
        49 2 * 1 + putchar
    ]
    ";
    let module = module(test).unwrap().1;

    let mut map_builder = FunctionMapBuilder::new();
    visitor::walk_module(&mut map_builder, &module);

    let functions = map_builder.get_final_map();

    let mut type_checker = ModuleTypeChecker::new(&functions);
    visitor::walk_module(&mut type_checker, &module);

    let mut codegen = ModuleCodeGen::new(&functions);
    visitor::walk_module(&mut codegen, &module);
}
