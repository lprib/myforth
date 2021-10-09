use std::collections::HashMap;

use ast::visitor;
use codegen::ModuleCodeGen;
use typecheck::{FunctionMapBuilder, ModuleTypeChecker};

use crate::{ast::{ConcreteType, Type}, parser::module};

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
    intrinsic fn drop 'T -> ;
    intrinsic fn swap 'T 'U -> 'U 'T;
    intrinsic fn dup 'T -> 'T 'T;

    extern fn main  ;
    
    fn main -> [
        50 dup + putchar
        5 2 * putchar
    ]
    ";
    
    // let mut gens = HashMap::new();
    // gens.insert("A".to_string(), Type::Concrete(ConcreteType::F32));

    // let a = Type::Generic("A".to_string());
    // let b = Type::Concrete(ConcreteType::I32);
    // let m= a.matches(&b, &mut gens);
    // println!("{}", m);
    // println!("{:?}", gens);
    // std::process::exit(0);

    let module = module(test).unwrap().1;

    let mut map_builder = FunctionMapBuilder::new();
    visitor::walk_module(&mut map_builder, &module);

    let functions = map_builder.get_final_map();

    let mut type_checker = ModuleTypeChecker::new(&functions);
    visitor::walk_module(&mut type_checker, &module);

    let mut codegen = ModuleCodeGen::new(&functions);
    visitor::walk_module(&mut codegen, &module);
}
