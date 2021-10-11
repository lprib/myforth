use core::panic;
use std::collections::HashMap;

use crate::ast::{
    visitor::{CodeBlockVisitor, ModuleVisitor},
    ConcreteType, FunctionDecl, FunctionImpl, FunctionType, IfStatement, Type, WhileStatement,
};

pub struct FunctionMapBuilder {
    // Maps name -> (type, is_implemented)
    functions: HashMap<String, (FunctionType, bool)>,
}

impl FunctionMapBuilder {
    pub fn new() -> Self {
        Self {
            functions: HashMap::new(),
        }
    }
}

// TODO make sure implementation type matches declaration type
impl ModuleVisitor<HashMap<String, FunctionType>> for FunctionMapBuilder {
    fn visit_decl(&mut self, f_decl: &FunctionDecl) {
        if self.functions.contains_key(&f_decl.head.name) {
            // TODO "previous declaration at X:X:X"
            panic!("Attempting to redeclare function {}", &f_decl.head.name);
        }
        self.functions
            .insert(f_decl.head.name.clone(), (f_decl.head.typ.clone(), false));
    }

    fn visit_impl(&mut self, f_impl: &FunctionImpl) {
        if self.functions.contains_key(&f_impl.head.name) && self.functions[&f_impl.head.name].1 {
            // TODO "previous implementation at X:X:X"
            panic!("Attempting to re-implement function {}", &f_impl.head.name);
        }

        self.functions
            .insert(f_impl.head.name.clone(), (f_impl.head.typ.clone(), true));
    }

    fn finalize(self) -> HashMap<String, FunctionType> {
        self.functions
            .into_iter()
            // remove bool in tuple
            .map(|(name, (typ, _))| (name, typ))
            .collect()
    }
}

impl Type {
    /// Checks whether self matches other. If self is a generic, the match always returns true and sets the generic's name to point to it's new reified type in generics_map
    pub(super) fn matches(&self, other: &Type, generics_map: &mut HashMap<String, Type>) -> bool {
        assert!(
            !matches!(other, &Type::Generic(_)),
            "Generics should only appear on LHS of type matcher"
        );
        match self {
            Type::Concrete(concrete) => {
                if let Type::Concrete(other_concrete) = other {
                    concrete == other_concrete
                } else {
                    false
                }
            }
            Type::Generic(name) => {
                // There is already a generic defined with a reified type
                if generics_map.contains_key(name) {
                    let previously_matched_generic = generics_map[name].clone();
                    previously_matched_generic.matches(other, generics_map)
                } else {
                    generics_map.insert(name.clone(), other.clone());
                    true
                }
            }
            Type::Pointer(inner) => {
                if let Type::Pointer(other_inner) = other {
                    inner.matches(other_inner, generics_map)
                } else {
                    false
                }
            }
        }
    }

    // Given a map of generics, reify the type of self (ie replace it with a concrete type if it
    // was a generic
    pub(super) fn reify(&self, generics_map: &mut HashMap<String, Type>) -> Type {
        match self {
            Type::Concrete(_) => self.clone(),
            Type::Generic(name) => generics_map
                .get(name)
                .expect("Undefined generic {} on RHS of type declaration")
                .clone(),
            Type::Pointer(inner) => Type::Pointer(Box::new(inner.reify(generics_map))),
        }
    }
}

struct CodeBlockTypeChecker<'a> {
    function_map: &'a HashMap<String, FunctionType>,
    type_stack: Vec<Type>,
}

impl<'a> CodeBlockTypeChecker<'a> {
    fn new(stack_before: Vec<Type>, function_map: &'a HashMap<String, FunctionType>) -> Self {
        Self {
            function_map,
            type_stack: stack_before,
        }
    }
}

// TODO this should somehow annotate any generic types with their reified type, to be passed to codegen
impl CodeBlockVisitor<Vec<Type>> for CodeBlockTypeChecker<'_> {
    fn visit_i32_literal(&mut self, _: i32) {
        self.type_stack.push(Type::Concrete(ConcreteType::I32))
    }

    fn visit_f32_literal(&mut self, _: f32) {
        self.type_stack.push(Type::Concrete(ConcreteType::F32))
    }

    fn visit_bool_literal(&mut self, _: bool) {
        self.type_stack.push(Type::Concrete(ConcreteType::Bool))
    }

    fn visit_function(&mut self, name: &str) {
        match self.function_map.get(name) {
            Some(typ) => {
                let mut generics_map = HashMap::new();
                // Validate that the inputs to the function are on the stack

                // TODO maybe should not reverse the iteration here, and instead check types from
                // left to right in order to provide better error messages. eg. matching ('T 'T)
                // with (i32 f32) will fail on the i32 (because of reverse) instead of failing on
                // the f32.
                for input_type in typ.inputs.iter().rev() {
                    let top_type = self.type_stack
                        .pop()
                        .unwrap_or_else(||
                            panic!(
                                "Expected a {:?} on the stack to pass to {}, but there was nothing on the stack", 
                                input_type,
                                name
                            )
                        );
                    assert!(
                        input_type.matches(&top_type, &mut generics_map),
                        "Expected an {:?} on the stack to pass to {}, but got a {:?}",
                        input_type,
                        name,
                        top_type
                    );
                }
                // Push the function's output to the stack
                self.type_stack.extend(
                    typ.outputs
                        .iter()
                        .map(|output_typ| output_typ.reify(&mut generics_map)),
                );
            }
            None => panic!("undefined function {}", name),
        }
    }

    fn visit_if_statement(&mut self, statement: &IfStatement) {
        match self.type_stack.pop() {
            None => panic!("Expected a bool value for if statment, but stack was empty"),
            Some(Type::Concrete(ConcreteType::Bool)) => {}
            Some(typ) => panic!("Expected a bool value for if statement, got {:?}", typ),
        }
        let true_branch = CodeBlockTypeChecker::new(self.type_stack.to_vec(), self.function_map)
            .walk(&statement.true_branch);
        let false_branch = CodeBlockTypeChecker::new(self.type_stack.to_vec(), self.function_map)
            .walk(&statement.false_branch);

        assert!(
            true_branch == false_branch,
            "If branches should have identical stack effects"
        );

        // We only need to append the true branch, as both branches are asserted to have identical stack effects above
        self.type_stack = true_branch;
    }

    fn visit_while_statement(&mut self, _statment: &WhileStatement) {
        todo!()
    }

    fn finalize(self) -> Vec<Type> {
        self.type_stack
    }
}

pub struct ModuleTypeChecker<'a> {
    functions: &'a HashMap<String, FunctionType>,
}

impl<'a> ModuleTypeChecker<'a> {
    pub fn new(functions: &'a HashMap<String, FunctionType>) -> Self {
        Self { functions }
    }
}

impl ModuleVisitor<()> for ModuleTypeChecker<'_> {
    fn visit_decl(&mut self, _: &FunctionDecl) {}

    fn visit_impl(&mut self, f_impl: &FunctionImpl) {
        let return_stack =
            CodeBlockTypeChecker::new(f_impl.head.typ.inputs.to_vec(), self.functions)
                .walk(&f_impl.body);

        assert!(
            return_stack == f_impl.head.typ.outputs,
            "Expected function to leave {:?} on the stack, instead it left {:?}",
            f_impl.head.typ.outputs,
            return_stack
        );
        println!("Function {} typechecked OK.", f_impl.head.name);
    }

    fn finalize(self) {}
}

#[cfg(test)]
mod tests {
    use crate::parser::module;

    use super::*;

    const INTRINSICS_DECLS: &'static str = "

        intrinsic fn dup 'T -> 'T 'T;
        intrinsic fn drop 'T -> ;
        intrinsic fn swap 'T 'U -> 'U 'T;
        intrinsic fn + i32 i32 -> i32;
        intrinsic fn = i32 i32 -> bool;

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
}
