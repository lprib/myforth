// Types:
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConcreteType {
    I32,
    F32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Type {
    Concrete(ConcreteType),
    Generic(String),
    Pointer(Box<Type>),
}

#[derive(Debug, Default, Clone)]
pub struct FunctionType {
    pub inputs: Vec<Type>,
    pub outputs: Vec<Type>,
}

// AST:

#[derive(Debug)]
pub struct IfStatement {
    pub true_branch: CodeBlock,
    pub false_branch: CodeBlock,
}

#[derive(Debug)]
pub struct WhileStatement {
    pub condition: CodeBlock,
    pub body: CodeBlock,
}

#[derive(Debug)]
pub enum Word {
    I32Literal(i32),
    F32Literal(f32),
    Function(String),
    IfStatement(IfStatement),
    WhilteStatement(WhileStatement),
}

// maybe add an Option<FuncitonType> here, which will initially be None and then gets set in the
// typechecking pass?
#[derive(Debug)]
pub struct CodeBlock(pub Vec<Word>);

#[derive(Debug)]
pub struct FunctionHeader {
    pub name: String,
    pub typ: FunctionType,
}

#[derive(Debug)]
pub struct FunctionDecl {
    pub head: FunctionHeader,
    pub is_intrinsic: bool,
    pub is_extern: bool,
}

#[derive(Debug)]
pub struct FunctionImpl {
    pub head: FunctionHeader,
    pub body: CodeBlock,
}

#[derive(Debug)]
pub enum TopLevelItem {
    Decl(FunctionDecl),
    Impl(FunctionImpl),
}

pub mod visitor {
    use super::*;

    pub trait ModuleVisitor {
        fn visit_decl(&mut self, f_decl: &FunctionDecl);
        fn visit_impl(&mut self, f_impl: &FunctionImpl);
        fn finalize(&mut self) {}
    }

    pub fn walk_module<V: ModuleVisitor>(visitor: &mut V, module: &Vec<TopLevelItem>) {
        for top_level_item in module {
            match top_level_item {
                TopLevelItem::Decl(f_decl) => visitor.visit_decl(f_decl),
                TopLevelItem::Impl(f_impl) => visitor.visit_impl(f_impl),
            }
        }
        visitor.finalize();
    }

    pub trait CodeBlockVisitor {
        fn visit_i32_literal(&mut self, n: i32);
        fn visit_f32_literal(&mut self, n: f32);
        fn visit_function(&mut self, name: &str);
        fn visit_if_statement(&mut self, statment: &IfStatement);
        fn visit_while_statement(&mut self, statment: &WhileStatement);
        fn finalize(&mut self) {}
    }

    pub fn walk_code_block<V: CodeBlockVisitor>(visitor: &mut V, block: &CodeBlock) {
        for word in &block.0 {
            match word {
                Word::I32Literal(i) => visitor.visit_i32_literal(*i),
                Word::F32Literal(f) => visitor.visit_f32_literal(*f),
                Word::Function(s) => visitor.visit_function(s),
                Word::IfStatement(if_statement) => visitor.visit_if_statement(if_statement),
                Word::WhilteStatement(while_statement) => {
                    visitor.visit_while_statement(while_statement)
                }
            }
        }
        visitor.finalize();
    }
}
