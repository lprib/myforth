use crate::ast::*;
use nom::{
    branch::alt,
    bytes::complete::{tag, take_until},
    character::complete::{alphanumeric1, char, digit1, i32, multispace1, none_of},
    combinator::{all_consuming, map, map_opt, map_res, opt, recognize},
    multi::{many1, separated_list0},
    sequence::{delimited, pair, preceded, separated_pair, terminated, tuple},
    IResult,
};

type PResult<'a, T> = IResult<&'a str, T>;

fn whitespace(input: &str) -> PResult<&str> {
    recognize(many1(alt((multispace1, comment))))(input)
}

fn maybe_whitespace(input: &str) -> PResult<&str> {
    recognize(opt(whitespace))(input)
}

fn comment(input: &str) -> PResult<&str> {
    delimited(char('['), take_until("]"), char(']'))(input)
}

fn word_text(input: &str) -> PResult<String> {
    map(recognize(many1(none_of(" []\t\r\n:;?@"))), String::from)(input)
}

fn word_function_call(input: &str) -> PResult<Word> {
    map(word_text, |name| {
        Word::FunctionCall(FunctionCall {
            name,
            reified_type: None,
        })
    })(input)
}

fn word_i32_literal(input: &str) -> PResult<Word> {
    map(i32, Word::I32Literal)(input)
}

fn word_f32_literal(input: &str) -> PResult<Word> {
    map_res(
        recognize(tuple((opt(char('-')), digit1, char('.'), digit1))),
        |s: &str| s.parse::<f32>().map(Word::F32Literal),
    )(input)
}

fn true_literal(input: &str) -> PResult<Word> {
    map_opt(word_text, |text| {
        (text == "t").then(|| Word::BoolLiteral(true))
    })(input)
}

fn false_literal(input: &str) -> PResult<Word> {
    map_opt(word_text, |text| {
        (text == "f").then(|| Word::BoolLiteral(false))
    })(input)
}

fn word_if_statement(input: &str) -> PResult<Word> {
    map(if_statement, Word::IfStatement)(input)
}

fn word_while_statement(input: &str) -> PResult<Word> {
    map(while_statement, Word::WhileStatement)(input)
}

fn word(input: &str) -> PResult<Word> {
    alt((
        word_if_statement,
        word_while_statement,
        word_f32_literal,
        word_i32_literal,
        true_literal,
        false_literal,
        word_function_call,
    ))(input)
}

fn words(input: &str) -> PResult<Vec<Word>> {
    separated_list0(whitespace, word)(input)
}

fn code_block(input: &str) -> PResult<CodeBlock> {
    map(words, CodeBlock)(input)
}

fn if_statement(input: &str) -> PResult<IfStatement> {
    map(
        tuple((
            terminated(char('?'), whitespace),
            terminated(code_block, maybe_whitespace),
            terminated(char(':'), whitespace),
            terminated(code_block, maybe_whitespace),
            char(';'),
        )),
        |(_, true_branch, _, false_branch, _)| IfStatement {
            true_branch,
            false_branch,
        },
    )(input)
}

fn while_statement(input: &str) -> PResult<WhileStatement> {
    map(
        tuple((
            terminated(char('@'), whitespace),
            terminated(code_block, maybe_whitespace),
            terminated(tag(":"), whitespace),
            terminated(code_block, maybe_whitespace),
            char(';'),
        )),
        |(_, condition, _, body, _)| WhileStatement { condition, body },
    )(input)
}

macro_rules! concrete_type_parser {
    ($input:expr, $($name:literal => $type:expr),*) => {
        let (input, typ) = alt(( $(tag($name)),* ))($input)?;
        match typ {
            $(
                $name => Ok((input, Type::Concrete($type))),
            )*
            _ => unreachable!(),
        }
    };
}

fn concrete_type(input: &str) -> PResult<Type> {
    concrete_type_parser! {
        input,
        "i" => ConcreteType::I32,
        "ui" => ConcreteType::U32,
        "f" => ConcreteType::F32,
        "d" => ConcreteType::F64,
        "q" => ConcreteType::I64,
        "uq" => ConcreteType::U64,
        "c" => ConcreteType::I8,
        "uc" => ConcreteType::U8,
        "b" => ConcreteType::Bool
    }
}

fn generic_type(input: &str) -> PResult<Type> {
    map(pair(char('\''), alphanumeric1), |x| {
        Type::Generic(String::from(x.1))
    })(input)
}

fn pointer_type(input: &str) -> PResult<Type> {
    map(preceded(char('*'), typ), |typ| Type::Pointer(Box::new(typ)))(input)
}

fn typ(input: &str) -> PResult<Type> {
    alt((pointer_type, concrete_type, generic_type))(input)
}

fn type_list(input: &str) -> PResult<Vec<Type>> {
    separated_list0(whitespace, typ)(input)
}

fn defined_function_type(input: &str) -> PResult<FunctionType> {
    map(
        separated_pair(
            terminated(type_list, maybe_whitespace),
            tag("->"),
            preceded(maybe_whitespace, type_list),
        ),
        |(inputs, outputs)| FunctionType { inputs, outputs },
    )(input)
}

fn not_defined_function_type(input: &str) -> PResult<FunctionType> {
    Ok((input, Default::default()))
}

fn function_type(input: &str) -> PResult<FunctionType> {
    alt((defined_function_type, not_defined_function_type))(input)
}

// TODO the lack of whitespace in this `fn a;` makes it not parse
fn function_header(input: &str) -> PResult<FunctionHeader> {
    map(
        tuple((terminated(word_text, maybe_whitespace), function_type)),
        |(name, typ)| FunctionHeader { name, typ },
    )(input)
}

fn function_decl(input: &str) -> PResult<FunctionDecl> {
    map(
        tuple((
            opt(terminated(tag("extern"), whitespace)),
            opt(terminated(tag("intrinsic"), whitespace)),
            terminated(function_header, maybe_whitespace),
            char(';'),
        )),
        |(extern_opt, intrinsic_opt, head, _)| FunctionDecl {
            head,
            is_extern: extern_opt.is_some(),
            is_intrinsic: intrinsic_opt.is_some(),
        },
    )(input)
}

fn function_impl(input: &str) -> PResult<FunctionImpl> {
    map(
        tuple((
            terminated(function_header, maybe_whitespace),
            terminated(char(':'), maybe_whitespace),
            code_block,
            preceded(maybe_whitespace, char(';')),
        )),
        |(head, _, body, _)| FunctionImpl { head, body },
    )(input)
}

fn function_decl_tli(input: &str) -> PResult<TopLevelItem> {
    map(function_decl, TopLevelItem::Decl)(input)
}

fn function_impl_tli(input: &str) -> PResult<TopLevelItem> {
    map(function_impl, TopLevelItem::Impl)(input)
}

pub fn top_level_item(input: &str) -> PResult<TopLevelItem> {
    alt((function_impl_tli, function_decl_tli))(input)
}

// TODO note that TLIs must be separates by some whitespace. If we want to be able to define
// directly adjacent TLIs eg. `fn a;fn b;`, cannot use separated_list1. Separating by
// maybe_whitespace doesn't work because maybe_whitespace matches an empty string, meaning the
// parser will look for another TLI even if there are none because it already saw a separator on
// the end of the file (ie. the  "" separator)
pub fn module(input: &str) -> PResult<Vec<TopLevelItem>> {
    all_consuming(delimited(
        maybe_whitespace,
        separated_list0(whitespace, top_level_item),
        maybe_whitespace,
    ))(input)
}

#[cfg(test)]
mod tests {
    use super::*;

    type TestResult = Result<(), String>;
    trait ParserTester {
        fn test(self) -> TestResult;
    }

    impl<T> ParserTester for PResult<'_, T> {
        fn test(self) -> TestResult {
            match self {
                Ok((remaining, _)) => {
                    if !remaining.is_empty() {
                        Err("Parser did not consume entire str".to_string())
                    } else {
                        Ok(())
                    }
                }
                Err(e) => Err(e.to_string()),
            }
        }
    }

    #[test]
    fn test_impl() -> TestResult {
        top_level_item("foo: ;").test()?;
        top_level_item("foo:;").test()?;
        top_level_item("foo :;").test()?;
        top_level_item("foo i -> i : ;").test()?;
        top_level_item("foo i -> i: ;").test()?;
        top_level_item("foo i -> :;").test()?;
        top_level_item("foo -> i :;").test()?;
        top_level_item("foo i f b -> i :;").test()?;
        top_level_item("foo 'Typ 'Typ2 *'Typ i f b -> 'Typ 'Typ2 *'Typ i f b:;").test()
    }

    #[test]
    fn test_fn_type() -> TestResult {
        function_type("i -> i").test()
    }

    #[test]
    fn test_decl() -> TestResult {
        top_level_item("foo ;").test()?;
        top_level_item("foo;").test()?;
        top_level_item("extern foo;").test()?;
        top_level_item("foo -> ;").test()?;
        top_level_item("intrinsic foo i -> i ;").test()?;
        top_level_item("extern foo i -> ;").test()?;
        top_level_item("foo -> i ;").test()?;
        top_level_item("foo i f b -> i ;").test()?;
        top_level_item("foo 'Typ 'Typ2 *'Typ i f b -> 'Typ 'Typ2 *'Typ i f b;").test()?;
        top_level_item("foo;").test()?;
        top_level_item("foo ->;").test()?;
        top_level_item("foo -> i;").test()?;
        top_level_item("foo i ->;").test()?;
        top_level_item("foo i -> i;").test()
    }

    #[test]
    fn test_module() -> TestResult {
        module("a; b;").test()?;
        module("a;").test()?;
        module(" a; b; ").test()?;
        module("").test()?;
        module("[comment]").test()
    }

    #[test]
    fn test_whitespace() -> TestResult {
        whitespace(" ").test()?;
        whitespace("[comment]").test()?;
        whitespace(" \t\n[co\nmment] ").test()?;
        whitespace("[comment][com \r\nm\rent]").test()?;
        maybe_whitespace(" ").test()?;
        maybe_whitespace(" \t\n[comment] ").test()?;
        maybe_whitespace("[comment][comment]").test()?;
        maybe_whitespace("").test()
    }

    #[test]
    fn test_if() -> TestResult {
        if_statement("? : ;").test()?;
        if_statement("? dup 3 + : dup 4 + ;").test()?;
        if_statement("? dup 3 + print : ;").test()?;
        function_impl("a b->i : ? 1 : 2 ;;").test()?;
        function_impl("a -> : 3 4 = ? 1 : 2 ; drop ;").test()
    }

    #[test]
    fn testwhile() -> TestResult {
        while_statement("@ t : ;").test()?;
        while_statement("@ 3 4 = : dup print ;").test()?;
        function_impl("a b->i : drop 0 @ dup 10 < : 1 + ; ;").test()
    }
}
