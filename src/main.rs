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
    intrinsic fn - i32 i32 -> i32;
    intrinsic fn < i32 i32 -> bool;
    intrinsic fn = i32 i32 -> bool;

    fn main -> [
        (98 1 test putchar 98 0 test putchar)
    ]

    (fn test i32 i32 -> i32 i32 [
        1 = if [
            3 4 + drop 97
        ] else [
            9 8 + drop 99
        ]
        swap 1 + swap
    ])
    
    (fn test2 bool -> i32 i32 [
        1 swap 2 swap if [
            10 + swap 10 + swap
        ] else [
            100 + swap 100 + swap
        ]
    ])

    (fn test2 i32 bool -> i32 i32 [
        99 swap if [
            10 +
        ] else [
            100 +
        ]
    ])
    
    extern fn test_nested bool bool -> i32;
    fn test_nested bool bool -> i32 [
        if [
            if [
                111
            ] else [
                222
            ]
        ] else [
            drop
            666
        ]
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
