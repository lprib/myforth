// Types:
#[derive(Debug)]
pub enum ConcreteType {
    I32,
}

#[derive(Debug)]
pub enum Type {
    Concrete(ConcreteType),
    Generic(String),
}

#[derive(Debug, Default)]
pub struct FunctionType {
    pub inputs: Vec<Type>,
    pub outputs: Vec<Type>,
}

// AST:

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
pub struct FunctionDeclaration {
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
    Function(String),
    IfStatement(IfStatement),
    WhilteStatement(WhileStatement),
}
