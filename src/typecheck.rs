use std::collections::HashMap;

use crate::ast::{
    visitor::{self, CodeBlockVisitor, ModuleVisitor},
    ConcreteType, FunctionDecl, FunctionImpl, FunctionType, IfStatement, Type, WhileStatement,
};

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
    expected_stack_effect: &'a FunctionType,
}

impl<'a> CodeBlockTypeChecker<'a> {
    fn new(self_type: &'a FunctionType, function_map: &'a HashMap<String, FunctionType>) -> Self {
        let type_stack = self_type.inputs.to_vec();
        Self {
            function_map,
            type_stack,
            expected_stack_effect: self_type,
        }
    }
}

impl CodeBlockVisitor for CodeBlockTypeChecker<'_> {
    fn visit_i32_literal(&mut self, _: i32) {
        self.type_stack.push(Type::Concrete(ConcreteType::I32))
    }

    fn visit_f32_literal(&mut self, _: f32) {
        self.type_stack.push(Type::Concrete(ConcreteType::F32))
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

    fn visit_if_statement(&mut self, statment: &IfStatement) {
        todo!()
    }

    fn visit_while_statement(&mut self, statment: &WhileStatement) {
        todo!()
    }

    fn finalize(&mut self) {
        assert!(
            self.type_stack == self.expected_stack_effect.outputs,
            "Expected function to leave {:?} on the stack, instead it left {:?}",
            self.expected_stack_effect.outputs,
            self.type_stack
        );
    }
}

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

    pub fn get_final_map(self) -> HashMap<String, FunctionType> {
        self.functions
            .into_iter()
            // remove bool in tuple
            .map(|(name, (typ, _))| (name, typ))
            .collect()
    }
}

impl ModuleVisitor for FunctionMapBuilder {
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
}

pub struct ModuleTypeChecker<'a> {
    functions: &'a HashMap<String, FunctionType>,
}

impl<'a> ModuleTypeChecker<'a> {
    pub fn new(functions: &'a HashMap<String, FunctionType>) -> Self {
        Self { functions }
    }
}

impl ModuleVisitor for ModuleTypeChecker<'_> {
    fn visit_decl(&mut self, _: &FunctionDecl) {}

    fn visit_impl(&mut self, f_impl: &FunctionImpl) {
        let mut type_checker = CodeBlockTypeChecker::new(&f_impl.head.typ, self.functions);
        visitor::walk_code_block(&mut type_checker, &f_impl.body);
        println!("Function {} typechecked OK.", f_impl.head.name);
    }
}
