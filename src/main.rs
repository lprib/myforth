use std::{
    fs,
    process::{Command, ExitStatus},
};

use ast::visitor::ModuleVisitor;
use codegen::module::ModuleCodeGen;
use typecheck::{FunctionMapBuilder, ModuleTypeChecker};

use crate::parser::module;

mod ast;
mod codegen;
mod parser;
mod typecheck;

fn main() {
    let test = "
    extern fn putchar i32 -> ;
    extern fn getchar -> i32;
    extern fn main -> ;

    intrinsic fn dup 'T -> 'T 'T;
    intrinsic fn drop 'T -> ;
    intrinsic fn swap 'T 'U -> 'U 'T;
    intrinsic fn + i32 i32 -> i32;
    intrinsic fn - i32 i32 -> i32;
    intrinsic fn < i32 i32 -> bool;
    intrinsic fn = i32 i32 -> bool;
    
    fn main [getchar 1 + putchar]

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
    let module_ir = ModuleCodeGen::new(&functions).walk(&module);
    println!("MODULE IR: {}", module_ir);
    run_ir(&module_ir);
}

fn run_ir(module: &str) {
    let ll_filename = "out.ll";
    fs::write(ll_filename, module).unwrap();

    let clang_out = Command::new("clang")
        .arg("-O3")
        .arg(ll_filename)
        .arg("-o")
        .arg("out")
        .output()
        .expect("Failed to invoke clang");
    println!(
        "CLANG STDOUT: {}",
        std::str::from_utf8(&clang_out.stdout).unwrap()
    );
    println!(
        "CLANG STDERR: {}",
        std::str::from_utf8(&clang_out.stderr).unwrap()
    );
    if !clang_out.status.success() {
        println!("Clang returned nonzero exit status");
        return;
    }

    let output = Command::new("./out")
        .output()
        .expect("Failed to invoke ./out");
    println!(
        "EXECUTABLE STDOUT:\n{}",
        std::str::from_utf8(&output.stdout).unwrap()
    );
}
