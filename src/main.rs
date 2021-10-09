use crate::parser::{function_declaration, function_impl};

mod ast;
mod parser;

fn main() {
    let test = "fn square i32 -> [ 3 4 + drop if [ 1 + ] else [ 2 + ] dup drop 3 4 = while [dup 10 >] do [1 +]]";
    println!("{:#?}", function_impl(test));
    let test = "extern intrinsic fn swap (comment) 'T 'U -> 'U 'T;";
    println!("{:#?}", function_declaration(test));
}
