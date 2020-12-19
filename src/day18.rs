use anyhow::Context;
use nom::{
    branch::alt,
    character::complete::{char, digit1, multispace0},
    combinator::{eof, not, peek},
    error::ParseError,
    Err, IResult, Parser,
};
use nom_supreme::{
    error::ErrorTree,
    final_parser::{final_parser, Location},
    multi::parse_separated_terminated,
    parse_from_str,
    parser_ext::ParserExt,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Operator {
    Plus,
    Times,
}

impl Operator {
    fn apply(&self, x: i64, y: i64) -> i64 {
        match *self {
            Operator::Plus => x + y,
            Operator::Times => x * y,
        }
    }
}

/// Parse an operator + or *
fn parse_operator(input: &str) -> IResult<&str, Operator, ErrorTree<&str>> {
    alt((
        char('+').value(Operator::Plus),
        char('*').value(Operator::Times),
    ))
    .context("operator")
    .parse(input)
}

/// Parse a single number like 25
fn parse_number(input: &str) -> IResult<&str, i64, ErrorTree<&str>> {
    parse_from_str(digit1).parse(input)
}

/// Parse a single number or a parenthesized expression
fn parse_item<'a>(
    expression: impl Parser<&'a str, i64, ErrorTree<&'a str>>,
) -> impl Parser<&'a str, i64, ErrorTree<&'a str>> {
    alt((parse_number, parse_parenthesized(expression))).context("item")
}

/// Parse a parenthesized expression using an expression parser
fn parse_parenthesized<'a>(
    expression: impl Parser<&'a str, i64, ErrorTree<&'a str>>,
) -> impl Parser<&'a str, i64, ErrorTree<&'a str>> {
    expression
        .preceded_by(char('(').terminated(multispace0))
        .terminated(char(')').preceded_by(multispace0))
        .context("parenthesized expression")
}

/// Peek if the next input is a number or an open parenthesis
fn peek_item(input: &str) -> IResult<&str, (), ErrorTree<&str>> {
    peek(alt((digit1.value(()), char('(').value(())))).parse(input)
}

fn parse_generic_expression<'a, O, T>(
    mut item: impl Parser<&'a str, T, ErrorTree<&'a str>>,
    operator: impl Parser<&'a str, O, ErrorTree<&'a str>>,
    apply: impl Fn(O, T, T) -> T,
) -> impl Parser<&'a str, T, ErrorTree<&'a str>> {
    let mut operator = operator
        .delimited_by_both(multispace0)
        .context("expression operator");
    move |input| {
        let (mut input, mut value) = item.parse(input)?;

        loop {
            let (tail, op) = match operator.parse(input) {
                Ok(result) => result,
                Err(Err::Error(..)) => break Ok((input, value)),
                Err(err) => return Err(err),
            };

            input = tail;

            let (tail, item) = item.parse(input)?;

            input = tail;
            value = apply(op, value, item);
        }
    }
}

fn parse_expression(input: &str) -> IResult<&str, i64, ErrorTree<&str>> {
    parse_generic_expression(parse_item(parse_expression), parse_operator, |op, x, y| {
        op.apply(x, y)
    })
    .parse(input)
}

fn parse_expression_list(input: &str) -> IResult<&str, i64, ErrorTree<&str>> {
    parse_separated_terminated(
        parse_expression,
        multispace0,
        multispace0.all_consuming(),
        || 0,
        |sum, value| sum + value,
    )
    .context("expression list")
    .parse(input)
}

fn evaluate_expression<'a>(
    expression: impl Parser<&'a str, i64, ErrorTree<&'a str>>,
    input: &'a str,
) -> Result<i64, ErrorTree<Location>> {
    final_parser(expression)(input)
}

pub fn part1(input: &str) -> anyhow::Result<i64> {
    evaluate_expression(parse_expression_list, input).context("Failed to parse input")
}

fn parse_product_expression(input: &str) -> IResult<&str, i64, ErrorTree<&str>> {
    parse_generic_expression(parse_sum_expression, char('*'), |_c, x, y| x * y)
        .context("product expression")
        .parse(input)
}

fn parse_sum_expression(input: &str) -> IResult<&str, i64, ErrorTree<&str>> {
    parse_generic_expression(
        parse_item(parse_product_expression),
        char('+'),
        |_c, x, y| x + y,
    )
    .context("sum expression")
    .parse(input)
}

fn parse_product_expression_list(input: &str) -> IResult<&str, i64, ErrorTree<&str>> {
    parse_separated_terminated(
        parse_product_expression,
        multispace0,
        multispace0.all_consuming(),
        || 0,
        |sum, value| sum + value,
    )
    .context("sum expression list")
    .parse(input)
}

pub fn part2(input: &str) -> anyhow::Result<i64> {
    evaluate_expression(parse_product_expression_list, input).context("Failed to parse input")
}
