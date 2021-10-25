# Strongly Typed Forth using LLVM

## Syntax
```
[comments are enclosed in square brackets]

[main should be external to link to c runtime]
extern main;

[fibonacci example:]
[takes an integer and returns an integer (hence i->i)]
fibonacci i->i :
    [if statement:]
    [<boolean> ? <true block> : <false block> ;]
    dup 1 <= ? :
        [recursion is supported]
        dup 1 - fibonacci swap 2 - fibonacci +
    ;
;

[print all fibonacci numbers from #0 to #20]
fibs:
    [while statement:]
    [@ <boolean condition> : <body> ;]
    0 @ dup 20 <= :
        dup fib iprint nl
        inc
    ;

    [The loop counter is still on the stack]
    [Language is typesafe, so if the signature says no return values,]
    [there must be nothing left on the stack... drop it]
    drop
;

[Multiple arguments]
sumAnd5 i i -> i : 5 + + ;

[Multiple returns]
[Note, (i) casts to integer and has the signature 'T -> i]
['T is generic and accepts any type]
doubleToIntAndIncrement f f -> i i : (i) 1 + swap (i) 5 + swap ;
```

## TODO
- [ ] Make intrinsics generics (ie. `intrinsic fn + num num -> num`) where i32, u8, f32: num
    - IntLike = bool | i32 | i64
    - General notion of type inheritance for intrinsics
    - Maybe `intrinsic type IntLike derives i u q uq c uc b;`
    - Allow the user to derive their own types?
- [ ] capitalize type names? `I U Q UQ C UC B *I...`?
    - More confusion with generics, but less confusion with function names etc.
- [ ] Add spans to parse
- [ ] JIT REPL
- [ ] constants `let a 5`;
    - allow constant folding and simple exprs: `let a : 5; let b : a 1 +;
- [ ]
- [ ] Make parsing and typechecking use spans for better error msg
- [ ] for visitor: make a result_visitor, where each visit function returns a Result<(), TError>
  - the finalize returns Result<Tsuccess, Terror>.
  - If any visit fails, return Err,
  - If all succeed, return result of finalize().
  - Make error enum for typechecking, with display implementation (with spans)
- [ ] In typecheck: ensure implementation type matches definition
- [x] if
- [x] while
- [ ] string literals, char literals
- [ ] pointer intrinsics
- [ ] array instantiations and indexing
- [ ] macros and #include (for stdlib/intrinsics include)
    - Can be done with a separate nom parser
- [ ] Compile to ASM or just invoke clang each time?
- [ ] rot, 3grab, 4grab, 5grab
- [x] parser failing if there is a function at end of module with no whitespace after
- [x] walk_n should be a default method on the visitor trait
- [x] more terse function syntax, better if/while syntax. maybe
  - bool ? true : false ;
  - inc i -> i: 1 + ;
  - rename i32 -> i, f32 -> f. Also ub (uint8) sb (int8), d (double), q (quadword, int64)
- [x] typecasts
- [ ] Compile time inlining (copy tokens)
    - `inline fn inc: 1 + ;`
    - `macro inc: 1 + ;` <-- prefer
    - Distinguish between macros (no type signature) and inline fn (with type signature?)
    - Or allow macros to have optional type signature?
- [ ] proper command line args
- [ ] stop all the .clone()s!
    - Everyone keeps references to the types created in typechecking/parsing
- [ ] CodeGen panics if functions reference functions that haven't been generated yet
    - Are stubs being generated properly?
