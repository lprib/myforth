    use crate::parser::module;

    use super::*;

    // TODO typechecker should return result with error types, so we can make sure #[should_panic]
    // panics with the right error message

    fn typecheck(input: &str) {
        let mut program = String::from(include_str!("../../std.f"));
        program.push_str(input);

        let mut module = module(&program).unwrap().1;
        let functions = FunctionMapBuilder::new().walk(&mut module);
        ModuleTypeChecker::new(&functions).walk(&mut module);
    }

    #[test]
    fn test_no_args() {
        typecheck("a : 3 4 + drop ;");
    }

    #[test]
    fn test_return() {
        typecheck("a -> i : 1 ;");
        typecheck("b -> i i i : 1 2 3 ;");
        typecheck("c -> i f : 1 1.0 ;");
    }

    #[test]
    #[should_panic]
    fn test_bad_return() {
        typecheck("a -> i : 1.0 ;");
    }

    #[test]
    #[should_panic]
    fn test_extra_return() {
        typecheck("a -> i : 1 1 ;");
    }

    #[test]
    #[should_panic]
    fn test_not_enough_return() {
        typecheck("a -> f i : i ;");
    }

    #[test]
    fn test_args() {
        typecheck("a i -> : drop ;");
    }

    #[test]
    fn test_input_output() {
        typecheck("a i -> i : 1 + ;");
        typecheck("b f -> f : ;");
        typecheck("c i f b -> i f b : ;");
    }

    #[test]
    fn test_drop() {
        typecheck("a : 3 drop ;");
        typecheck("c : 1.0 drop ;");
        typecheck("b : t drop ;");
        typecheck("d : 3 1.0 drop drop ;");
        typecheck("e i i -> i : drop ;");
    }

    #[test]
    fn test_swap() {
        typecheck("a i f -> f i : swap ;");
    }

    #[test]
    fn test_dup() {
        typecheck("a f -> f f : dup ;");
    }

    #[test]
    fn test_generics() {
        typecheck(
            "
        nop 'T -> 'T;
        foo 'T 'U 'V -> 'V 'T 'U;
        deref *'T -> 'T;
        ref 'T -> *'T;
        
        test1 -> i *f b : 1 nop 1.0 ref t ref deref ;
        test2 f *i -> *f i : deref swap ref swap ;
        test3 i -> **i : ref deref ref ref ;
        test4 ***b -> b : deref deref deref ;
        
        deref3 ***'T -> 'T;
        test5 **i -> i : ref deref3 ;
        ",
        )
    }

    #[test]
    #[should_panic]
    fn test_undef_generic() {
        // NOTE: undefined generics ('Q) only get caught when the generic is reified/monomorphized
        typecheck("a i 'T 'U -> 'U 'Q 'T b; test : 1 t f drop drop drop drop ;");
    }

    #[test]
    #[should_panic]
    fn test_bad_deref() {
        typecheck("deref *'T -> 'T; test : 1 deref ;");
    }

    #[test]
    fn test_literals() {
        typecheck("a -> i : 1 ;");
        typecheck("b -> f : 1.0 ;");
        typecheck("c -> b : t ;");
    }

    #[test]
    fn test_call() {
        typecheck("a i -> f; testa i -> f : a ;");
        typecheck("a i i -> f; testb -> f : 1 2 a ;");
        typecheck("a f i -> f; testc -> f : 1.0 2 a ;");
    }

    #[test]
    #[should_panic]
    fn test_bad_call_args() {
        typecheck("a f -> ; b : 1 a ;");
    }

    #[test]
    #[should_panic]
    fn test_no_call_args() {
        typecheck("a f -> ; b : a ;");
    }

    #[test]
    fn test_decl_then_impl() {
        typecheck("a; a : ;");
    }

    #[test]
    #[should_panic]
    fn test_redecl() {
        typecheck("a; a;");
    }

    #[test]
    #[should_panic]
    fn test_reimpl() {
        typecheck("a : ; a : ;");
    }

    #[test]
    fn test_if() {
        typecheck("a -> : t ? : ; ;");
        typecheck("b -> i : t ? 1 : 2 ; ;");
        typecheck("c -> f i : t ? 1.0 1 : 2.0 2 ; ;");
        typecheck("d -> f i : f ? 1.0 1 : 1 1.0 swap 1 + ; ;");
        typecheck("e -> i : 1 f ? 1 + : drop 1 2 + ; ;");
    }

    #[test]
    fn test_nested_if() {
        typecheck(
            "
        a :
            t ?
                1
                f ?
                    1 +
                :
                    2 +
                ;
            :
                1 2 +
            ;
            drop
        ;
        ",
        );
    }

    #[test]
    #[should_panic]
    fn test_if_use_nonexistant_val() {
        typecheck("a : t ? 1 + : drop ; ;")
    }

    #[test]
    #[should_panic]
    fn test_if_nonequal_branches() {
        typecheck("a : t ? 1.0 : 1 ; drop ;")
    }

    #[test]
    #[should_panic]
    fn test_if_no_bool_input() {
        typecheck("a : 1 ? : ; ]");
    }

    #[test]
    fn test_while() {
        typecheck("a : @ t : ; ;");
        typecheck("b : 0 @ dup 10 < : 1 + ; drop ;");
    }

    #[test]
    #[should_panic]
    fn test_while_not_bool() {
        typecheck("a : @ 1 : ; ;");
    }

    #[test]
    #[should_panic]
    fn test_while_body_has_stack_effect() {
        typecheck("a : @ t : 1 ; ;");
    }