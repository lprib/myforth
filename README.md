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
- [ ] Add spans to parse
- [ ] Make parsing and typechecking use spans for better error msg
- [ ] 