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
    intrinsic fn drop 'T -> ;
    intrinsic fn swap 'T 'U -> 'U 'T;
    intrinsic fn dup 'T -> 'T 'T;
    intrinsic fn th_int *i32 i32 -> i32;

    extern fn main  ;
    
    fn a -> i32 [ 5 ]
    
    fn p i32 -> [
        putchar
    ]
    
    fn main -> i32 [
        a 
    ]
    ";

    let test = "
    extern fn putchar i32 -> ;
    extern fn main -> ;

    intrinsic fn dup 'T -> 'T 'T;
    intrinsic fn drop 'T -> ;
    intrinsic fn swap 'T 'U -> 'U 'T;
    intrinsic fn + i32 i32 -> i32;

    fn test3 -> i32 i32 i32 [ 10 97 99 ]
    
    fn main -> [
        test3 putchar putchar putchar
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
