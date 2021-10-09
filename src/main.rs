use ast::visitor;
use typecheck::{FunctionMapBuilder, ModuleTypeChecker};

use crate::parser::{module, top_level_item};

mod ast;
mod parser;
mod typecheck;
mod codegen;

fn main() {
    let test = "
    intrinsic fn convert i32 -> f32;
    intrinsic fn swap0 i32 f32 -> f32 i32;
    intrinsic fn drop i32 -> ;
    intrinsic fn dropf f32 -> ;
    intrinsic fn + i32 i32 -> i32;
    
    fn test -> f32 i32 [
        2.0 3.0 swap0
    ]
    ";
    let module = module(test).unwrap().1;

    let mut map_builder = FunctionMapBuilder::new();
    visitor::walk_module(&mut map_builder, &module);

    let mut type_checker = ModuleTypeChecker::new(&map_builder.map);
    visitor::walk_module(&mut type_checker, &module);
}
