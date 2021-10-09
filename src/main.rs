use ast::visitor;
use codegen::ModuleCodeGen;
use typecheck::{FunctionMapBuilder, ModuleTypeChecker};

use crate::parser::{module, top_level_item};

mod ast;
mod parser;
mod typecheck;
mod codegen;

fn main() {
    let test = "
    extern fn print i32 -> ;
    intrinsic fn + i32 i32 -> i32;
    
    fn test -> [
        3 4 + print
    ]
    ";
    let module = module(test).unwrap().1;

    let mut map_builder = FunctionMapBuilder::new();
    visitor::walk_module(&mut map_builder, &module);

    let mut type_checker = ModuleTypeChecker::new(&map_builder.map);
    visitor::walk_module(&mut type_checker, &module);
    
    let mut codegen = ModuleCodeGen::new(&map_builder.map);
    visitor::walk_module(&mut codegen, &module);
}
