use ast::visitor::ModuleVisitor;
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
    extern fn main -> ;

    intrinsic fn dup 'T -> 'T 'T;
    intrinsic fn drop 'T -> ;
    intrinsic fn swap 'T 'U -> 'U 'T;
    intrinsic fn + i32 i32 -> i32;
    intrinsic fn < i32 i32 -> bool;
    intrinsic fn = i32 i32 -> bool;

    fn test3 -> i32 i32 i32 [ 10 97 99 ]
    
    fn main -> [
        test3 putchar putchar putchar
    ]
    
    fn bar i32 i32 -> bool [
        drop drop f
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

    let functions = FunctionMapBuilder::new().walk(&module);

    ModuleTypeChecker::new(&functions).walk(&module);
    ModuleCodeGen::new(&functions).walk(&module);
}
