use std::collections::HashMap;

use anyhow::Context;
use cascade::cascade;
use nom::{
    branch::alt,
    bytes::complete::is_not,
    character::complete::{anychar, char, digit1, space0, space1},
    combinator::peek,
    multi::many1,
    Err, IResult, Parser,
};
use nom_supreme::{
    error::ErrorTree,
    final_parser::{final_parser, Location},
    multi::{parse_separated_terminated, parse_separated_terminated_res},
    parse_from_str,
    parser_ext::ParserExt,
    tag::complete::tag,
};
use thiserror::Error;
struct BoxedParser<'a, I, O, E> {
    parser: Box<dyn Parser<I, O, E> + 'a>,
}

impl<'a, I, O, E> Parser<I, O, E> for BoxedParser<'a, I, O, E> {
    fn parse(&mut self, input: I) -> IResult<I, O, E> {
        self.parser.parse(input)
    }
}

impl<'a, I, O, E> BoxedParser<'a, I, O, E> {
    fn new<P: Parser<I, O, E> + 'a>(parser: P) -> Self {
        Self {
            parser: Box::new(parser),
        }
    }
}

#[derive(Debug, Clone)]
enum Rule {
    Char(char),
    SubRules(RuleChoices),
}

impl Rule {
    fn build_parser<'a>(&self, rules: &RuleSet, special: bool) -> BoxedParser<'a, &'a str, (), ()> {
        match *self {
            Rule::Char(c) => BoxedParser::new(char(c).value(())),
            Rule::SubRules(ref choices) => choices.build_parser(rules, special),
        }
    }
}

fn parse_rule(input: &str) -> IResult<&str, Rule, ErrorTree<&str>> {
    alt((
        parse_rule_choices.map(Rule::SubRules),
        anychar.delimited_by(char('"'), char('"')).map(Rule::Char),
    ))
    .context("rule")
    .parse(input)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct RuleID {
    id: i64,
}

fn parse_rule_id(input: &str) -> IResult<&str, RuleID, ErrorTree<&str>> {
    parse_from_str(digit1)
        .map(|id| RuleID { id })
        .context("Rule ID")
        .parse(input)
}

/// A list of rules that must all match in sequence
#[derive(Debug, Clone)]
struct RuleChain {
    rules: Vec<RuleID>,
}

impl RuleChain {
    fn build_parser<'a>(&self, rules: &RuleSet, special: bool) -> BoxedParser<'a, &'a str, (), ()> {
        let mut parsers: Vec<_> = self
            .rules
            .iter()
            .map(|id| rules.build_parser_for(id, special))
            .collect();

        BoxedParser::new(move |input| {
            parsers
                .iter_mut()
                .try_fold((input, ()), |(input, ()), parser| parser.parse(input))
        })
    }
}

fn parse_rule_chain(input: &str) -> IResult<&str, RuleChain, ErrorTree<&str>> {
    parse_separated_terminated(
        parse_rule_id,
        space1,
        peek(space0.terminated(char('\n').or(char('|')))),
        Vec::new,
        |chain, id| cascade! {chain; ..push(id);},
    )
    .map(|rules| RuleChain { rules })
    .context("Rule chain")
    .parse(input)
}

/// A list of rules, at least one of them must match
#[derive(Debug, Clone)]
struct RuleChoices {
    choices: Vec<RuleChain>,
}

impl RuleChoices {
    fn build_parser<'a>(&self, rules: &RuleSet, special: bool) -> BoxedParser<'a, &'a str, (), ()> {
        let mut parsers: Vec<_> = self
            .choices
            .iter()
            .map(|choice| choice.build_parser(rules, special))
            .collect();

        BoxedParser::new(move |input| {
            for parser in &mut parsers {
                match parser.parse(input) {
                    Ok((tail, ())) => return Ok((tail, ())),
                    Err(Err::Error(())) => continue,
                    Err(err) => return Err(err),
                }
            }

            Err(Err::Error(()))
        })
    }
}

fn parse_rule_choices(input: &str) -> IResult<&str, RuleChoices, ErrorTree<&str>> {
    parse_separated_terminated(
        parse_rule_chain,
        char('|').delimited_by_both(space1),
        peek(space0.terminated(char('\n'))),
        Vec::new,
        |choices, choice| cascade! {choices; ..push(choice);},
    )
    .map(|choices| RuleChoices { choices })
    .context("Rule choices")
    .parse(input)
}

#[derive(Debug, Clone)]
struct RuleSet {
    rules: HashMap<RuleID, Rule>,
}

impl RuleSet {
    fn build_parser_for<'a>(&self, id: &RuleID, special: bool) -> BoxedParser<'a, &'a str, (), ()> {
        match (special, id.id) {
            (true, 8) => {
                let rule = self.rules.get(id).unwrap();
                let subrule = match rule {
                    Rule::Char(..) => panic!("Invalid rule 8"),
                    Rule::SubRules(choices) => &choices.choices[0].rules[0],
                };

                let subparser = self.build_parser_for(subrule, special);

                BoxedParser::new(many1(subparser).value(()))
            }
            (true, 11) => {
                let rule = self.rules.get(id).unwrap();
                let subrule = match rule {
                    Rule::Char(..) => panic!("Invalid rule 11"),
                    Rule::SubRules(choices) => &choices.choices[0].rules,
                };

                let prefix = self.build_parser_for(&subrule[0], special);
                let suffix = self.build_parser_for(&subrule[1], special);

                BoxedParser::new(SelfNestedParser { prefix, suffix })
            }
            _ => self.rules.get(id).unwrap().build_parser(self, special),
        }
    }
}

#[derive(Debug, Clone)]
struct SelfNestedParser<P1, P2> {
    prefix: P1,
    suffix: P2,
}

impl<'a, P1, P2> Parser<&'a str, (), ()> for SelfNestedParser<P1, P2>
where
    P1: Parser<&'a str, (), ()>,
    P2: Parser<&'a str, (), ()>,
{
    fn parse(&mut self, input: &'a str) -> IResult<&'a str, (), ()> {
        let (input, ()) = self.prefix.parse(input)?;

        match self.suffix.parse(input) {
            Ok((tail, ())) => return Ok((tail, ())),
            Err(Err::Error(())) => {}
            Err(err) => return Err(err),
        }

        let input = match self.parse(input) {
            Ok((tail, ())) => tail,
            Err(Err::Error(())) => input,
            Err(err) => return Err(err),
        };

        self.suffix.parse(input)
    }
}

#[derive(Debug, Clone, Error)]
#[error("Duplicate rule id: {id:?}")]
struct DuplicateRuleId {
    id: RuleID,
}

fn parse_rule_set(input: &str) -> IResult<&str, RuleSet, ErrorTree<&str>> {
    parse_separated_terminated_res(
        parse_rule_id
            .terminated(char(':').terminated(space0))
            .and(parse_rule)
            .context("rule set item"),
        char('\n'),
        tag("\n\n"),
        HashMap::new,
        |mut rules, (id, rule)| {
            if rules.insert(id, rule).is_some() {
                Err(DuplicateRuleId { id })
            } else {
                Ok(rules)
            }
        },
    )
    .map(|rules| RuleSet { rules })
    .context("rule set")
    .parse(input)
}

fn parse_lines(input: &str) -> IResult<&str, Vec<&str>, ErrorTree<&str>> {
    parse_separated_terminated(
        is_not("\n"),
        char('\n'),
        char('\n').all_consuming(),
        Vec::new,
        |lines, line| cascade! {lines; ..push(line);},
    )
    .context("messages")
    .parse(input)
}

fn parse_input(input: &str) -> Result<(RuleSet, Vec<&str>), ErrorTree<Location>> {
    final_parser(parse_rule_set.and(parse_lines))(input)
}

pub fn part1(input: &str) -> anyhow::Result<usize> {
    let (rules, lines) = parse_input(input).context("Failed to parse input")?;
    let mut parser = rules
        .build_parser_for(&RuleID { id: 0 }, false)
        .all_consuming();

    let matching = lines
        .iter()
        .filter(|line| parser.parse(line).is_ok())
        .count();

    Ok(matching)
}

pub fn part2(input: &str) -> anyhow::Result<usize> {
    let (rules, lines) = parse_input(input).context("Failed to parse input")?;
    let mut parser = rules
        .build_parser_for(&RuleID { id: 0 }, true)
        .all_consuming();

    let matching = lines
        .iter()
        .filter(|line| parser.parse(line).is_ok())
        .count();

    Ok(matching)
}
