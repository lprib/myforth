// Types:
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConcreteType {
    I32,
    U32,
    F32,
    F64,
    U64,
    I64,
    U8,
    I8,
    Bool,
}

macro_rules! concrete_type_properties {
    ($($type:pat => $is_signed:expr, $is_integral:expr, $width:expr;)*) => {
        impl ConcreteType {
            fn is_signed(&self) -> bool {
                match self {
                    $($type => $is_signed,)*
                }
            }

            fn is_integral(&self) -> bool {
                match self {
                    $($type => $is_integral,)*
                }
            }

            fn width(&self) -> u32 {
                match self {
                    $($type => $width,)*
                }
            }
        }
    };
}

concrete_type_properties! {
    ConcreteType::I32 => true, true, 32;
    ConcreteType::U32 => false, true, 32;
    ConcreteType::F32 => true, false, 32;
    ConcreteType::F64 => true, false, 64;
    ConcreteType::I64 => true, true, 64;
    ConcreteType::U64 => false, true, 64;
    ConcreteType::I8 => true, true, 8;
    ConcreteType::U8 => false, true, 8;
    ConcreteType::Bool => false, true, 1;
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
pub struct FunctionCall {
    pub name: String,
    pub reified_type: Option<FunctionType>,
}

#[derive(Debug)]
pub enum Word {
    I32Literal(i32),
    F32Literal(f32),
    BoolLiteral(bool),
    FunctionCall(FunctionCall),
    IfStatement(IfStatement),
    WhileStatement(WhileStatement),
}

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

    pub trait ModuleVisitor<TOut>
    where
        Self: Sized,
    {
        fn visit_decl(&mut self, function: &mut FunctionDecl);
        fn visit_impl(&mut self, function: &mut FunctionImpl);
        fn finalize(self) -> TOut;
        fn walk(mut self, module: &mut [TopLevelItem]) -> TOut {
            for top_level_item in module {
                match top_level_item {
                    TopLevelItem::Decl(f_decl) => self.visit_decl(f_decl),
                    TopLevelItem::Impl(f_impl) => self.visit_impl(f_impl),
                }
            }
            self.finalize()
        }
    }

    pub trait CodeBlockVisitor<TOut>
    where
        Self: Sized,
    {
        fn visit_i32_literal(&mut self, n: i32);
        fn visit_f32_literal(&mut self, n: f32);
        fn visit_bool_literal(&mut self, n: bool);

        // need to pass a mut ref to word here so that the typechecker can annotate the function
        // with it's reified type signature.
        fn visit_function(&mut self, function: &mut FunctionCall);
        
        // Any AST nodes which may contain function calls (eg. code blocks in if) must also be mut
        // so the typechecker can annotate them
        fn visit_if_statement(&mut self, statement: &mut IfStatement);
        fn visit_while_statement(&mut self, statement: &mut WhileStatement);
        fn finalize(self) -> TOut;
        fn walk(mut self, block: &mut CodeBlock) -> TOut {
            for word in &mut block.0 {
                match word {
                    Word::I32Literal(n) => self.visit_i32_literal(*n),
                    Word::F32Literal(n) => self.visit_f32_literal(*n),
                    Word::BoolLiteral(n) => self.visit_bool_literal(*n),
                    Word::FunctionCall(function) => self.visit_function(function),
                    Word::IfStatement(if_statement) => self.visit_if_statement(if_statement),
                    Word::WhileStatement(while_statement) => {
                        self.visit_while_statement(while_statement)
                    }
                }
            }
            self.finalize()
        }
    }
}
