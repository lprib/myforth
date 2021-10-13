#[cfg(test)]
mod typecheck_tests;

use core::panic;
use std::collections::HashMap;

use crate::ast::{
    visitor::{CodeBlockVisitor, ModuleVisitor},
    ConcreteType, FunctionCall, FunctionDecl, FunctionImpl, FunctionType, IfStatement, Type,
    WhileStatement,
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
    fn visit_decl(&mut self, function: &mut FunctionDecl) {
        if self.functions.contains_key(&function.head.name) {
            // TODO "previous declaration at X:X:X"
            panic!("Attempting to redeclare function {}", &function.head.name);
        }
        self.functions.insert(
            function.head.name.clone(),
            (function.head.typ.clone(), false),
        );
    }

    fn visit_impl(&mut self, function: &mut FunctionImpl) {
        if self.functions.contains_key(&function.head.name) && self.functions[&function.head.name].1
        {
            // TODO "previous implementation at X:X:X"
            panic!(
                "Attempting to re-implement function {}",
                &function.head.name
            );
        }

        self.functions.insert(
            function.head.name.clone(),
            (function.head.typ.clone(), true),
        );
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

    /// Returns the overall effect on the stack of a given operation.  For example (i32) -> (i32)
    /// has the overall effect of () -> (), since the function will replace the i32 in-place.
    fn get_stack_effect<'i, 'o>(input: &'i [Type], output: &'o [Type]) -> (&'i [Type], &'o [Type]) {
        let mut compare_index = 0usize;
        for (i, o) in input.iter().zip(output.iter()) {
            if i == o {
                compare_index += 1;
            } else {
                break;
            }
        }

        (&input[compare_index..], &output[compare_index..])
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

    fn visit_function(&mut self, function: &mut FunctionCall) {
        // instantiate reified input/output vectors
        function.reified_type = Some(FunctionType {
            inputs: Vec::new(),
            outputs: Vec::new(),
        });

        match self.function_map.get(&function.name) {
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
                                function.name
                            )
                        );
                    assert!(
                        input_type.matches(&top_type, &mut generics_map),
                        "Expected an {:?} on the stack to pass to {}, but got a {:?}",
                        input_type,
                        function.name,
                        top_type
                    );

                    // Annotate function with it's reified inputs.
                    // Can safely unwrap because it has just been set above.
                    // Insert at start of vec because we are iterating on the stack from right to
                    // left (popping off the end of stack), but funtion IOs go from left to right
                    // on the stack
                    function
                        .reified_type
                        .as_mut()
                        .unwrap()
                        .inputs
                        .insert(0, top_type.clone());
                }
                let reified_outputs = typ
                    .outputs
                    .iter()
                    .map(|output_typ| output_typ.reify(&mut generics_map))
                    .collect::<Vec<_>>();

                // Push the function's output to the stack
                self.type_stack.extend(reified_outputs.to_vec());
                // annotate function with it's reified outputs
                function.reified_type.as_mut().unwrap().outputs = reified_outputs;
            }
            None => panic!("undefined function {}", function.name),
        }
    }

    fn visit_if_statement(&mut self, statement: &mut IfStatement) {
        match self.type_stack.pop() {
            None => panic!("Expected a bool value for if statment, but stack was empty"),
            Some(Type::Concrete(ConcreteType::Bool)) => {}
            Some(typ) => panic!("Expected a bool value for if statement, got {:?}", typ),
        }
        let true_branch = CodeBlockTypeChecker::new(self.type_stack.to_vec(), self.function_map)
            .walk(&mut statement.true_branch);
        let false_branch = CodeBlockTypeChecker::new(self.type_stack.to_vec(), self.function_map)
            .walk(&mut statement.false_branch);

        assert!(
            true_branch == false_branch,
            "If branches should have identical stack effects"
        );

        // We only need to append the true branch, as both branches are asserted to have identical stack effects above
        self.type_stack = true_branch;
    }

    fn visit_while_statement(&mut self, statement: &mut WhileStatement) {
        let condition_block_result =
            CodeBlockTypeChecker::new(self.type_stack.to_vec(), self.function_map)
                .walk(&mut statement.condition);
        let (effect_in, effect_out) =
            Self::get_stack_effect(&self.type_stack, &condition_block_result);

        assert!(
            effect_in.is_empty(),
            "expected while condition to not consume anything on the stack, instead it consumed {:?}",
            effect_in
        );
        assert!(
            effect_out == [Type::Concrete(ConcreteType::Bool)],
            "expected while condition to produce a bool, instead it produced {:?}",
            effect_out
        );

        let body_result = CodeBlockTypeChecker::new(self.type_stack.to_vec(), self.function_map)
            .walk(&mut statement.body);
        let (effect_in, effect_out) = Self::get_stack_effect(&self.type_stack, &body_result);

        assert!(
            effect_in.is_empty(),
            "While body consumes {:?}, it should not consume anything",
            effect_in
        );
        assert!(
            effect_out.is_empty(),
            "While body produces {:?}, it should not produce anything",
            effect_out
        );
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
    fn visit_decl(&mut self, _: &mut FunctionDecl) {}

    fn visit_impl(&mut self, function: &mut FunctionImpl) {
        let return_stack =
            CodeBlockTypeChecker::new(function.head.typ.inputs.to_vec(), self.functions)
                .walk(&mut function.body);

        assert!(
            return_stack == function.head.typ.outputs,
            "Expected function to leave {:?} on the stack, instead it left {:?}",
            function.head.typ.outputs,
            return_stack
        );
        println!("Function {} typechecked OK.", function.head.name);
    }

    fn finalize(self) {}
}
