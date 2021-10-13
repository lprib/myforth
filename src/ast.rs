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
pub struct IfStatement<T = Word> {
    pub true_branch: CodeBlock<T>,
    pub false_branch: CodeBlock<T>,
}

#[derive(Debug)]
pub struct WhileStatement<T = Word> {
    pub condition: CodeBlock<T>,
    pub body: CodeBlock<T>,
}

#[derive(Debug)]
pub enum Word {
    I32Literal(i32),
    F32Literal(f32),
    BoolLiteral(bool),
    Function(String),
    IfStatement(IfStatement<Word>),
    WhileStatement(WhileStatement<Word>),
}

// Equivalent of word in the ast, but represents types instead of values
pub enum TypedWord {
    Literal(Type),
    Function(Vec<TypedWord>),
    IfStatement(IfStatement<TypedWord>),
    WhileStatement(WhileStatement<TypedWord>),
}

#[derive(Debug)]
pub struct CodeBlock<T = Word>(pub Vec<T>);

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
pub struct FunctionImpl<T = Word> {
    pub head: FunctionHeader,
    pub body: CodeBlock<T>,
}

#[derive(Debug)]
pub enum TopLevelItem<T = Word> {
    Decl(FunctionDecl),
    Impl(FunctionImpl<T>),
}

pub mod visitor {
    use super::*;

    pub trait ModuleVisitor<TOut>
    where
        Self: Sized,
    {
        fn visit_decl(&mut self, function: &FunctionDecl);
        fn visit_impl(&mut self, function: &FunctionImpl);
        fn finalize(self) -> TOut;
        fn walk(mut self, module: &[TopLevelItem]) -> TOut {
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
        fn visit_function(&mut self, name: &str);
        fn visit_if_statement(&mut self, statement: &IfStatement);
        fn visit_while_statement(&mut self, statement: &WhileStatement);
        fn finalize(self) -> TOut;
        fn walk(mut self, block: &CodeBlock) -> TOut {
            for word in &block.0 {
                match word {
                    Word::I32Literal(n) => self.visit_i32_literal(*n),
                    Word::F32Literal(n) => self.visit_f32_literal(*n),
                    Word::BoolLiteral(n) => self.visit_bool_literal(*n),
                    Word::Function(s) => self.visit_function(s),
                    Word::IfStatement(if_statement) => self.visit_if_statement(if_statement),
                    Word::WhileStatement(while_statement) => {
                        self.visit_while_statement(while_statement)
                    }
                }
            }
            self.finalize()
        }
    }

    // walks module (Vec<Word>) and typed module (Vec<TypedWord>) at the same time. Both must be
    // identical in structure, but with Word and TypedWord respectively
    pub trait TypedModuleVisitor<TOut>
    where
        Self: Sized,
    {
        fn visit_decl(&mut self, function: FunctionDecl);
        fn visit_impl(
            &mut self,
            function: FunctionImpl<Word>,
            function_typed: FunctionImpl<TypedWord>,
        );
        fn finalize(self) -> TOut;
        fn walk(
            mut self,
            module: Vec<TopLevelItem<Word>>,
            typed_module: Vec<TopLevelItem<TypedWord>>,
        ) -> TOut {
            for (tli, tli_type) in module.into_iter().zip(typed_module.into_iter()) {
                match (tli, tli_type) {
                    (TopLevelItem::Decl(function), TopLevelItem::Decl(_)) => {
                        self.visit_decl(function)
                    }
                    (TopLevelItem::Impl(function), TopLevelItem::Impl(typed_function)) => {
                        self.visit_impl(function, typed_function)
                    }
                    (_, _) => panic!("value module does not match types module"),
                }
            }
            self.finalize()
        }
    }

    // Walks CodeBlock<Word> and CodeBlock<TypedWord> at the same time. Must be identical in
    // structure, but with Word and TypedWord respectively
    pub trait TypedCodeBlockVisitor<TOut>
    where
        Self: Sized,
    {
        fn visit_i32_literal(&mut self, n: i32, typ: Type);
        fn visit_f32_literal(&mut self, n: f32, typ: Type);
        fn visit_bool_literal(&mut self, n: bool, typ: Type);
        fn visit_function(&mut self, name: String, typ: Vec<TypedWord>);
        fn visit_if_statement(
            &mut self,
            statement: IfStatement<Word>,
            statement_types: IfStatement<TypedWord>,
        );
        fn visit_while_statement(
            &mut self,
            statement: WhileStatement<Word>,
            statement_type: WhileStatement<TypedWord>,
        );
        fn finalize(self) -> TOut;
        fn walk(mut self, code: CodeBlock<Word>, types: CodeBlock<TypedWord>) -> TOut {
            for (word, typed_word) in code.0.into_iter().zip(types.0.into_iter()) {
                match (word, typed_word) {
                    (Word::I32Literal(n), TypedWord::Literal(t)) => self.visit_i32_literal(n, t),
                    (Word::F32Literal(n), TypedWord::Literal(t)) => self.visit_f32_literal(n, t),
                    (Word::BoolLiteral(n), TypedWord::Literal(t)) => self.visit_bool_literal(n, t),
                    (Word::Function(name), TypedWord::Function(outputs)) => {
                        self.visit_function(name, outputs)
                    }
                    (
                        Word::IfStatement(if_statement),
                        TypedWord::IfStatement(typed_if_statment),
                    ) => self.visit_if_statement(if_statement, typed_if_statment),
                    (
                        Word::WhileStatement(while_statement),
                        TypedWord::WhileStatement(typed_while_statement),
                    ) => self.visit_while_statement(while_statement, typed_while_statement),
                    (_, _) => panic!("Value codeblock (CodeBlock<Word>) and type codeblock (CodeBlock<Type>) do not match")
                }
            }
            self.finalize()
        }
    }
}
