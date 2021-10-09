use crate::ast::*;
use nom::{
    branch::alt,
    bytes::complete::{tag, take_until},
    character::complete::{alphanumeric1, char, i32, multispace1, none_of},
    combinator::{map, opt, recognize},
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
    delimited(char('('), take_until(")"), char(')'))(input)
}

fn word_text(input: &str) -> PResult<String> {
    map(recognize(many1(none_of(" []()\t\r\n"))), String::from)(input)
}

fn word_function_call(input: &str) -> PResult<Word> {
    map(word_text, |word| Word::Function(String::from(word)))(input)
}

fn word_i32_literal(input: &str) -> PResult<Word> {
    map(i32, |n| Word::I32Literal(n))(input)
}

fn word_if_statement(input: &str) -> PResult<Word> {
    map(if_statement, |if_statement| Word::IfStatement(if_statement))(input)
}

fn word_while_statement(input: &str) -> PResult<Word> {
    map(while_statement, |while_statement| {
        Word::WhilteStatement(while_statement)
    })(input)
}

fn word(input: &str) -> PResult<Word> {
    alt((
        word_if_statement,
        word_while_statement,
        word_i32_literal,
        word_function_call,
    ))(input)
}

fn words(input: &str) -> PResult<Vec<Word>> {
    separated_list0(whitespace, word)(input)
}

fn code_block(input: &str) -> PResult<CodeBlock> {
    map(
        delimited(
            pair(char('['), opt(whitespace)),
            words,
            pair(opt(whitespace), char(']')),
        ),
        |words| CodeBlock(words),
    )(input)
}

fn concrete_type(input: &str) -> PResult<Type> {
    let (input, typ) = alt((tag("i32"), tag("f32")))(input)?;

    match typ {
        "i32" => Ok((input, Type::Concrete(ConcreteType::I32))),
        "f32" => todo!(),
        _ => unreachable!(),
    }
}

fn generic_type(input: &str) -> PResult<Type> {
    map(pair(char('\''), alphanumeric1), |x| {
        Type::Generic(String::from(x.1))
    })(input)
}

fn typ(input: &str) -> PResult<Type> {
    alt((concrete_type, generic_type))(input)
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

fn function_header(input: &str) -> PResult<FunctionHeader> {
    map(
        tuple((
            terminated(tag("fn"), whitespace),
            terminated(word_text, whitespace),
            function_type,
        )),
        |(_, name, typ)| FunctionHeader { name, typ },
    )(input)
}

pub fn function_declaration(input: &str) -> PResult<FunctionDeclaration> {
    map(
        tuple((
            opt(terminated(tag("extern"), whitespace)),
            opt(terminated(tag("intrinsic"), whitespace)),
            terminated(function_header, maybe_whitespace),
            tag(";"),
        )),
        |(extern_opt, intrinsic_opt, head, _)| FunctionDeclaration {
            head: head,
            is_extern: extern_opt.is_some(),
            is_intrinsic: intrinsic_opt.is_some(),
        },
    )(input)
}

pub fn function_impl(input: &str) -> PResult<FunctionImpl> {
    map(
        tuple((
            terminated(function_header, maybe_whitespace),
            code_block,
        )),
        |(head, body)| FunctionImpl { head, body },
    )(input)
}

fn if_statement(input: &str) -> PResult<IfStatement> {
    map(
        tuple((
            terminated(tag("if"), whitespace),
            terminated(code_block, whitespace),
            terminated(tag("else"), whitespace),
            code_block,
        )),
        |(_, true_branch, _, false_branch)| IfStatement {
            true_branch,
            false_branch,
        },
    )(input)
}

fn while_statement(input: &str) -> PResult<WhileStatement> {
    map(
        tuple((
            terminated(tag("while"), whitespace),
            terminated(code_block, whitespace),
            terminated(tag("do"), whitespace),
            code_block,
        )),
        |(_, condition, _, body)| WhileStatement { condition, body },
    )(input)
}
