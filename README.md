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
- [ ] In typecheck: ensure implementation type matches definition
- [ ] if/while
- [ ] parser failing if there is a function at end of module with no whitespace after
- [ ] walk_n should be a default method on the visitor trait