    use crate::parser::module;

    use super::*;

    const INTRINSICS_DECLS: &'static str = "

        intrinsic fn dup 'T -> 'T 'T;
        intrinsic fn drop 'T -> ;
        intrinsic fn swap 'T 'U -> 'U 'T;
        intrinsic fn + i32 i32 -> i32;
        intrinsic fn = i32 i32 -> bool;
        intrinsic fn < i32 i32 -> bool;

    ";

    // TODO typechecker should return result with error types, so we can make sure #[should_panic]
    // panics with the right error message

    fn typecheck(input: &str) {
        let mut program = String::from(INTRINSICS_DECLS);
        program.push_str(input);

        let module = module(&program).unwrap().1;
        let functions = FunctionMapBuilder::new().walk(&module);
        ModuleTypeChecker::new(&functions).walk(&module);
    }

    #[test]
    fn test_no_args() {
        typecheck("fn a [ 3 4 + drop ]");
    }

    #[test]
    fn test_return() {
        typecheck("fn a -> i32 [ 1 ]");
        typecheck("fn b -> i32 i32 i32 [ 1 2 3 ]");
        typecheck("fn c -> i32 f32 [ 1 1.0 ]");
    }

    #[test]
    #[should_panic]
    fn test_bad_return() {
        typecheck("fn a -> i32 [ 1.0 ]");
    }

    #[test]
    #[should_panic]
    fn test_extra_return() {
        typecheck("fn a -> i32 [ 1 1 ]");
    }

    #[test]
    #[should_panic]
    fn test_not_enough_return() {
        typecheck("fn a -> f32 i32 [ i32 ]");
    }

    #[test]
    fn test_args() {
        typecheck("fn a i32 -> [ drop ]");
    }

    #[test]
    fn test_input_output() {
        typecheck("fn a i32 -> i32 [ 1 + ]");
        typecheck("fn b f32 -> f32 [ ]");
        typecheck("fn c i32 f32 bool -> i32 f32 bool [ ]");
    }

    #[test]
    fn test_drop() {
        typecheck("fn a [ 3 drop ]");
        typecheck("fn c [ 1.0 drop ]");
        typecheck("fn b [ t drop ]");
        typecheck("fn d [ 3 1.0 drop drop ]");
        typecheck("fn e i32 i32 -> i32 [ drop ]");
    }

    #[test]
    fn test_swap() {
        typecheck("fn a i32 f32 -> f32 i32 [ swap ]");
    }

    #[test]
    fn test_dup() {
        typecheck("fn a f32 -> f32 f32 [ dup ]");
    }

    #[test]
    fn test_generics() {
        typecheck(
            "
        fn nop 'T -> 'T;
        fn foo 'T 'U 'V -> 'V 'T 'U;
        fn deref *'T -> 'T;
        fn ref 'T -> *'T;
        
        fn test1 -> i32 *f32 bool [ 1 nop 1.0 ref t ref deref ]
        fn test2 f32 *i32 -> *f32 i32 [ deref swap ref swap ]
        fn test3 i32 -> **i32 [ ref deref ref ref ]
        fn test4 ***bool -> bool [ deref deref deref ]
        
        fn deref3 ***'T -> 'T;
        fn test5 **i32 -> i32 [ ref deref3 ]
        ",
        )
    }

    #[test]
    #[should_panic]
    fn test_undef_generic() {
        // NOTE: undefined generics ('Q) only get caught when the generic is reified/monomorphized
        typecheck("fn a i32 'T 'U -> 'U 'Q 'T bool; fn test [ 1 t f32 drop drop drop drop ]");
    }

    #[test]
    #[should_panic]
    fn test_bad_deref() {
        typecheck("fn deref *'T -> 'T; fn test [ 1 deref ]");
    }

    #[test]
    fn test_literals() {
        typecheck("fn a -> i32 [ 1 ]");
        typecheck("fn b -> f32 [ 1.0 ]");
        typecheck("fn c -> bool [ t ]");
    }

    #[test]
    fn test_call() {
        typecheck("fn a i32 -> f32; fn testa i32 -> f32 [ a ]");
        typecheck("fn a i32 i32 -> f32; fn testb -> f32 [ 1 2 a ]");
        typecheck("fn a f32 i32 -> f32; fn testc -> f32 [ 1.0 2 a ]");
    }

    #[test]
    #[should_panic]
    fn test_bad_call_args() {
        typecheck("fn a f32 -> ; fn b [ 1 a ]");
    }

    #[test]
    #[should_panic]
    fn test_no_call_args() {
        typecheck("fn a f32 -> ; fn b [ a ]");
    }

    #[test]
    fn test_decl_then_impl() {
        typecheck("fn a; fn a [ ]");
    }

    #[test]
    #[should_panic]
    fn test_redecl() {
        typecheck("fn a; fn a;");
    }

    #[test]
    #[should_panic]
    fn test_reimpl() {
        typecheck("fn a [ ] fn a [ ]");
    }

    #[test]
    fn test_if() {
        typecheck("fn a -> [ t if [ ] else [ ] ]");
        typecheck("fn b -> i32 [ t if [ 1 ] else [ 2 ] ]");
        typecheck("fn c -> f32 i32 [ t if [ 1.0 1 ] else [ 2.0 2 ] ]");
        typecheck("fn d -> f32 i32 [ f if [ 1.0 1 ] else [ 1 1.0 swap 1 + ] ]");
        typecheck("fn e -> i32 [ 1 f if [ 1 + ] else [ drop 1 2 + ] ]");
    }

    #[test]
    fn test_nested_if() {
        typecheck(
            "
        fn a [
            t if [
                1
                f if [
                    1 +
                ] else [
                    2 +
                ]
            ] else [
                1 2 +
            ]
            drop
        ]
        ",
        );
    }

    #[test]
    #[should_panic]
    fn test_if_use_nonexistant_val() {
        typecheck("fn a [ t if [ 1 + ] else [ drop ] ]")
    }

    #[test]
    #[should_panic]
    fn test_if_nonequal_branches() {
        typecheck("fn a [ t if [ 1.0 ] else [ 1 ] drop ] ]")
    }

    #[test]
    #[should_panic]
    fn test_if_no_bool_input() {
        typecheck("fn a [ 1 if [ ] else [ ] ]");
    }

    #[test]
    fn test_while() {
        typecheck("fn a [ while [ t ] do [ ] ]");
        typecheck("fn b [ 0 while [ dup 10 < ] do [ 1 + ] drop ]");
    }

    #[test]
    #[should_panic]
    fn test_while_not_bool() {
        typecheck("fn a [ while [ 1 ] do [ ] ]");
    }

    #[test]
    #[should_panic]
    fn test_while_body_has_stack_effect() {
        typecheck("fn a [ while [ t ] do [ 1 ] ]");
    }