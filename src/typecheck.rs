use std::collections::HashMap;

use crate::ast::{
    visitor::{self, CodeBlockVisitor, ModuleVisitor},
    ConcreteType, FunctionDecl, FunctionImpl, FunctionType, IfStatement, Type, WhileStatement,
};

pub struct FunctionMapBuilder {
    pub map: HashMap<String, FunctionType>,
}

impl FunctionMapBuilder {
    pub fn new() -> Self {
        Self {
            map: HashMap::new(),
        }
    }
}

impl ModuleVisitor for FunctionMapBuilder {
    fn visit_decl(&mut self, f_decl: &FunctionDecl) {
        // TODO no clone implementation (keep refs?)
        self.map
            .insert(f_decl.head.name.clone(), f_decl.head.typ.clone());
    }

    fn visit_impl(&mut self, f_impl: &FunctionImpl) {
        self.map
            .insert(f_impl.head.name.clone(), f_impl.head.typ.clone());
    }
}

struct CodeBlockTypeChecker<'a> {
    function_map: &'a HashMap<String, FunctionType>,
    type_stack: Vec<Type>,
    expected_stack_effect: &'a FunctionType,
}

impl<'a> CodeBlockTypeChecker<'a> {
    fn new(self_type: &'a FunctionType, function_map: &'a HashMap<String, FunctionType>) -> Self {
        let type_stack = self_type.inputs.iter().cloned().collect();
        Self {
            function_map,
            type_stack,
            expected_stack_effect: self_type,
        }
    }
}

impl CodeBlockVisitor for CodeBlockTypeChecker<'_> {
    fn visit_i32_literal(&mut self, n: i32) {
        self.type_stack.push(Type::Concrete(ConcreteType::I32))
    }

    fn visit_f32_literal(&mut self, n: f32) {
        self.type_stack.push(Type::Concrete(ConcreteType::F32))
    }

    fn visit_function(&mut self, name: &str) {
        match self.function_map.get(name) {
            Some(typ) => {
                // Validate that the inputs to the function are on the stack
                for input_type in typ.inputs.iter().rev() {
                    let top_type = self.type_stack
                        .pop()
                        .expect(
                            &format!(
                                "Expected a {:?} on the stack to pass to {}, but there was nothing on the stack", 
                                input_type,
                                name
                            )
                        );
                    assert!(
                        top_type == *input_type,
                        "Expected an {:?} on the stack to pass to {}, but got a {:?}",
                        input_type,
                        name,
                        top_type
                    );
                }
                // Push the function's output to the stack
                self.type_stack.extend(typ.outputs.iter().cloned());
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

pub struct ModuleTypeChecker<'a> {
    function_map: &'a HashMap<String, FunctionType>,
}

impl<'a> ModuleTypeChecker<'a> {
    pub fn new(function_map: &'a HashMap<String, FunctionType>) -> Self {
        Self { function_map }
    }
}

impl ModuleVisitor for ModuleTypeChecker<'_> {
    fn visit_decl(&mut self, f_decl: &FunctionDecl) {}

    fn visit_impl(&mut self, f_impl: &FunctionImpl) {
        let mut type_checker = CodeBlockTypeChecker::new(&f_impl.head.typ, self.function_map);
        visitor::walk_code_block(&mut type_checker, &f_impl.body);
        println!("Function {} typechecked OK.", f_impl.head.name);
    }
}
