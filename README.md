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
- [ ] Make intrinsics generics (ie. `intrinsic + num num -> num`) where i32, u8, f32: num
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
    - `inline inc: 1 + ;`
    - `macro inc: 1 + ;` <-- prefer
    - Distinguish between macros (no type signature) and inline fn (with type signature?)
    - Or allow macros to have optional type signature?
- [ ] proper command line args
- [ ] stop all the .clone()s!
    - Everyone keeps references to the types created in typechecking/parsing
- [ ] CodeGen panics if functions reference functions that haven't been generated yet
    - Are stubs being generated properly?
- [ ] Change bool literals to `true`, `false` to fix ambiguity with float type `f`

## Sytax wishlist
### records
```
record my-struct :
    foo i ,
    bar i ,
    other b ,
;

get-foo-element-plus-one my-struct -> i :
    .my-struct.foo nip 1 +
;

[element getters have the effective types:]
intrinsic .my-struct.foo my-struct -> my-struct i;

[constructing my-struct]
new-my-struct -> my-struct :
    [How does the typechecker find the type of construct<'T>? It takes arbitrary arguments so needs a special case]
    2 3 false construct<my-struct>
;

[option 2 for construction, doesnt require variadic construct function]
new-my-struct -> my-struct :
    [construct fills with uninitialized values]
    construct<my-struct>
    2 >my-struct.foo
    3 >my-struct.bar
    false >my-struct.other
;

[element setters have the effective types:]
[Note that they return the item back on the stack to avoid destroying it or requiring a bunch of dups]
intrinsic >my-struct.foo my-struct i -> my-struct;
intrinsic >my-struct.bar my-struct i -> my-struct;
intrinsic >my-struct.ther my-struct b -> my-struct;
```

Note that this requires reserving `.ident` for struct member geting. Can
restrict all identifiers to not include `.`, or just require that their first
character is not a period.

```
get_ptr my-struct -> i :
    my-struct (*my-struct) (i)
```
This requires refactoring the cast operator to not be a function but '(' .. type .. ')'

Should casts always succeed? how does casting my-struct to other-struct work? Only cast integer and pointer types.

*my-struct -> *other-struct should succeed but can access out-of-bounds memory.

#### Generalizing getters and setters
```
record my-struct :
    foo i ,
    bar i ,
    other b ,
;
[returns pointer into struct with getelementptr]
intrinsic my-struct.foo-ptr my-struct -> my-struct *i;

[getter]
.my-struct.foo my-struct -> my-struct i :
    my-struct.foo-ptr deref
;

[setter]
>my-struct.foo my-struct i -> my-struct :
    swap my-struct.foo-ptr
    [stack: i my-struct *i]
    rot
    [stack: my-struct *i i]
    store
```

can we generalize GEP operations to work on both structs and arays as in LLVM
IR? Ie. any aggregate type provides a way to access the nth item. Where N is
either an integer (arrays) or an identifier (structs).

#### Records without named items
Can more easily lowered to GEP calls in LLVM. More in line with the "no named
variables" symmetry with functions.

`nth` is a special function which takes a struct or array and returns a pointer
to the nth element. Easily converted to a GEP.

```
record baz i b ;
intrinsic nth 'TStruct i -> *'TItem;

.baz.0: baz -> baz i : dup 0 nth deref ;
.baz.1: baz -> baz b : dup 0 nth deref ;

```

Alternatively (even closer to GEP):
```
record baz i b ;
intrinsic nth *'TStruct i -> *'TItem;

.baz.0: *baz -> i : 0 nth deref ;
.baz.1: *baz -> b : 0 nth deref ;
```

### More literals
- Hex: `0xFF -> i`
- Bin: `0b1010 -> i`
- CStrings `"foo" -> *c`
    - Allocate in writable memory section
- characters `'a' -> c`
- Untyped integers? eg passing 123 to a `i -> 'T` will coerce 123 to i, but passing to `uq -> 'T` will coerce to uq

### If-elseif
Currently:
```
dup input test-1 ?
    branch1
    :
dup input test-2 ?
    branch2
    :
dup input test-1 ?
    branch3
  :
    else-branch
;
;
; [note many end blocks]
```

### First class functions
####  Types
`(i->i)` a function type with int input and int output.

#### Invoking
invoking functions using `invoke`?

`invoke` should run the function pointer on the current stack
```
add-and-apply i (i->i) -> i :
    [add 1 to first arg]
    swap 1 +
    [invoke second arg (a function) on the result]
    swap invoke
;

apply-to-range (i -> i) -> i :
    [iterate from 0 to 10]
    0 @ dup 10 < :
        [stack: fn iter]

        2dup
        [stack: fn iter fn iter] 

        swap
        [stack: fn iter iter fn]

        invoke [invokes fn on top of stack, which consumes the iter variable below and pushes it's result (an int)]
        [stack: fn iter fn-result]
        iprintln
    ;
;

[condensed version]
apply-to-range-condensed (i -> i) -> i :
    0 @ dup 10 < : 2dup swap invoke iprintln ;
;
```

Obtaining a function value `&fun`

```
return-plus -> (i i -> i): &+ ;
```

### Closures
- Capture environment/containing stack
```
[closure that takes integer and returns integer]
&fn(i -> i) : 1 + ;

```
Closures by default should execute imediately as if they were called. This
provides symmetry with function pointers `&` defined above.

This should create an anonymous function which is executed immediately:

`add-one-immediate-closure i -> i : fn(i -> i): 1 + ; ;`

instead of scope capturing, can instead support partial application / currying.
Closure is a normal function, which is curried with the captured variables. The
value in brackets should denote how many values should be taken off the stack
to be curried.

a "closure" which captures it's environment through currying:

`2.3 dup &{1}fn(f i -> i): [closure body] ;` The top value on the stack (the
closure) has the type `(i -> i)` because the f was curried.

```
add i i -> i : + ;
add-two i -> i: 2 {1}add ;

[with first class functions]
add-2 -> (i -> i) : 1 &{2}add ;
```

```
map array<'T> ('T -> 'U) -> array<'U> :
    swap .array.len construct<array<'U>>
    [stack: array<'T> closure array<'U>]

;
```

#### Lowering closures to functions:
use-closure i ()


### Implicit duping (low priority)
It is very common to duplicate a value and then perform an operation to avoid
destroying the value in the process. Provide a prefix or alias dup to
something very short like "

`1 dup inc` becomes `1 " inc` or `1 "inc`

### Arrays
Must have parity with C array operations.
- creation: `int arr[] = {1, 2, 3}`
- indexing: `int x = arr[N]`
- set at index: `arr[N] = 5`
- length: `int length = sizeof(arr) / sizeof(arr[0])`

Allow arrays to be a first-class type that can be pushed to the stack? Or only
use pointers?

```
[first-class array option]
[array type: {}T (WIP syntax)]

intrinsic gen-int-array [size] i -> {}i;
[if the generic support was to be extended, this could be changed to]
intrinsic gen-array i -> {}'T;
[and called like this]
10 gen-array<uq>
[to generate array of uq with len 10]

intrinsic nth {}'T i -> 'T;
intrinsic len {}'T -> i;
```

How to access array multiple times without pointer? dup currently doesn't
actually duplicate anything, just duplicates the LLVMValue reference.

```
[stack: array]
dup
[stack: array array] [both point to the same array]
len
[stack: array array-len]
```

Other options
- arrays are always simple pointers (must manually keep track of len)
- arrays are always fat pointers aka (T{}, i)
- arrays are fat pointers, but use struct syntax (cleaner than the fat pointer option)
```
[requires generic records]
record array 'T :
    ptr *T ,
    len i ,
;

len array<'T> -> i : .len ;
```
