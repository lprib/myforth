use std::collections::HashMap;

use crate::ast::{visitor::ModuleVisitor, FunctionDecl, FunctionImpl, FunctionType};

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