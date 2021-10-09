use crate::ast::visitor::ModuleVisitor;

struct ModuleCodeGen {

}

impl ModuleVisitor for ModuleCodeGen {
    fn visit_decl(&mut self, f_decl: &crate::ast::FunctionDecl) {
        todo!()
    }

    fn visit_impl(&mut self, f_impl: &crate::ast::FunctionImpl) {
        todo!()
    }

    fn finalize(&mut self) {
       todo!() 
    }
}