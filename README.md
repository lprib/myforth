# Strongly typed compiled forth

Syntax:
```
// [ words ] is always a block of instructions

fn square i32 -> i32 [
    dup *
]

// generics have '
fn triple 'T -> 'T 'T 'T [
    dup dup
]

fn foo i32 'T -> 'T i32 [
    swap 1 +
]

intrinsic fn + i32 i32 -> i32;
intrinsic fn swap 'T 'U -> 'U 'T;
intrinsic fn drop 'T -> ;

// maybe?
intrinsic fn to_stack *'T -> 'T;

```

File example:

```
extern fn henlo i32 -> i32;
```

## TODO
- [ ] Make intrinsics generics (ie. `intrinsic fn + num num -> num`) where i32, u8, f32: num
    - IntLike = bool | i32 | i64
    - General notion of type inheritance for intrinsics
- [ ] Add spans to parse
- [ ] Make parsing and typechecking use spans for better error msg
- [ ] for visitor: make a result_visitor, where each visit function returns a Result<(), TError>
  - the finalize returns Result<Tsuccess, Terror>.
  - If any visit fails, return Err,
  - If all succeed, return result of finalize().
  - Make error enum for typechecking, with display implementation (with spans)
- [ ] In typecheck: ensure implementation type matches definition
- [x] if
- [x] while
- [ ] string literals and pointer intrinsics
- [ ] array instantiations and indexing
- [ ] macros and #include (for stdlib/intrinsics include)
    - Can be done with a separate nom parser
- [ ] Compile to ASM or just invoke clang each time?
- [ ] rot, 3grab, 4grab, 5grab
- [x] parser failing if there is a function at end of module with no whitespace after
- [x] walk_n should be a default method on the visitor trait
- [ ] more terse function syntax, better if/while syntax. maybe
  - bool ? true : false ;
  - inc i -> i: 1 + ;
  - rename i32 -> i, f32 -> f. Also ub (uint8) sb (int8), d (double), q (quadword, int64)
- [ ] typecasts
- [ ] Compile time inlining (copy tokens)
