use crate::parser::function_definition;

mod ast;
mod parser;

fn main() {
    let test = "fn square i32 -> [ 3 4 + drop if [ 1 + ] else [ 2 + ] dup drop 3 4 = while [dup 10 >] do [1 +]]";
    println!("{:#?}", function_definition(test));
}
