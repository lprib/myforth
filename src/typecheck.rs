use std::collections::HashMap;

use crate::ast::{
    visitor::{self, CodeBlockVisitor, ModuleVisitor},
    ConcreteType, FunctionDecl, FunctionImpl, FunctionType, IfStatement, Type, WhileStatement,
};

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
    fn visit_i32_literal(&mut self, _: i32) {
        self.type_stack.push(Type::Concrete(ConcreteType::I32))
    }

    fn visit_f32_literal(&mut self, _: f32) {
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
        let mut type_checker = CodeBlockTypeChecker::new(&f_impl.head.typ, &self.functions);
        visitor::walk_code_block(&mut type_checker, &f_impl.body);
        println!("Function {} typechecked OK.", f_impl.head.name);
    }
}
