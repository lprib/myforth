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
    let test = concat!(
        include_str!("../std.f"),
        "
    extern getchar -> i;
    extern main;

    pow2 i->i: 1 swap << ;

    powersof2 :
        1 @ dup 30 < :
            dup pow2 iprintln
            inc
        ;
        @ dup 1 > :
            dup pow2 iprintln
            dec
        ; drop
    ;

    fib i->i :
        dup 1 <= ? :
            dup 1 - fib swap 2 - fib +
        ;
    ;
    
    fibs:
        0 @ dup 20 <= :
            dup iprint
            32 putchar
            dup fib iprintln
            inc
        ; drop
    ;

    main : fibs ;

    "
    );
    let mut module = module(test).unwrap().1;

    let functions = FunctionMapBuilder::new().walk(&mut module);

    ModuleTypeChecker::new(&functions).walk(&mut module);
    let module_ir = ModuleCodeGen::new(&functions).walk(&mut module);
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
