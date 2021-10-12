use std::{fs, process::Command};

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
    intrinsic fn dup2 'T 'U -> 'T 'U 'T 'U;
    intrinsic fn drop 'T -> ;
    intrinsic fn over 'T 'U -> 'T 'U 'T;
    intrinsic fn swap 'T 'U -> 'U 'T;
    intrinsic fn + i32 i32 -> i32;
    intrinsic fn - i32 i32 -> i32;
    intrinsic fn * i32 i32 -> i32;
    intrinsic fn / i32 i32 -> i32;
    intrinsic fn % i32 i32 -> i32;
    intrinsic fn >> i32 i32 -> i32;
    intrinsic fn << i32 i32 -> i32;
    intrinsic fn < i32 i32 -> bool;
    intrinsic fn > i32 i32 -> bool;
    intrinsic fn = i32 i32 -> bool;
    
    (fn main [
        97 dup 26 + swap while [ dup2 swap < ] do [
            dup 2 % 0 = if [
                dup putchar
            ] else []
            1 +
        ] drop drop
        10 putchar
    ])
    
    (fn main [
        1 while [dup 30 <] do [
            dup 1 swap << 
            1 +
        ] drop
    ])
    
    fn print i32 ->;
    
    fn main [ 123 print ]
    
    fn print i32 -> [
        dup 9 > if [
            dup 10 / 10 * dup
            - dup print
        ] else []
        48 + putchar
    ]

    ";
    let module = module(test).unwrap().1;

    let functions = FunctionMapBuilder::new().walk(&module);

    ModuleTypeChecker::new(&functions).walk(&module);
    let module_ir = ModuleCodeGen::new(&functions).walk(&module);
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
