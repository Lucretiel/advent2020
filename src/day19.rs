use std::{collections::HashMap, fmt::Display};

use anyhow::Context;
use cascade::cascade;
use joinery::prelude::*;
use lazy_format::{lazy_format, make_lazy_format};
use nom::{
    branch::alt,
    bytes::complete::is_not,
    character::complete::{anychar, char, digit1, space0, space1},
    combinator::peek,
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
use regex::{Regex, RegexBuilder};
use thiserror::Error;

#[derive(Debug, Clone)]
enum Rule {
    Char(char),
    SubRules(RuleChoices),
}

impl Rule {
    fn build_pattern<'a>(&'a self, rules: &'a RuleSet, special: bool) -> impl Display + 'a {
        lazy_format!(match (self) {
            Rule::Char(c) => ("{}", c),
            Rule::SubRules(choices) => ("{}", choices.build_pattern(rules, special)),
        })
    }

    fn extract_rule_11_patterns<'a>(
        &'a self,
        rules: &'a RuleSet,
        special: bool,
    ) -> (impl Display + 'a, impl Display + 'a) {
        let subrule = match self {
            Rule::Char(..) => panic!("Rule 11 invalid"),
            Rule::SubRules(choices) => &choices.choices[0].rules,
        };

        (
            rules.build_pattern_for(&subrule[0], special),
            rules.build_pattern_for(&subrule[1], special),
        )
    }
}

fn parse_rule(input: &str) -> IResult<&str, Rule, ErrorTree<&str>> {
    alt((
        parse_rule_choices.map(Rule::SubRules),
        anychar.delimited_by(char('"')).map(Rule::Char),
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
    fn build_pattern<'a>(&'a self, rules: &'a RuleSet, special: bool) -> impl Display + 'a {
        lazy_format!(("{}", rules.build_pattern_for(id, special)) for id in &self.rules)
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
    fn build_pattern<'a>(&'a self, rules: &'a RuleSet, special: bool) -> impl Display + 'a {
        let choices = self
            .choices
            .iter()
            .map(move |choice| choice.build_pattern(rules, special))
            .join_with("|");

        lazy_format!("(?:{})", choices)
    }
}

fn parse_rule_choices(input: &str) -> IResult<&str, RuleChoices, ErrorTree<&str>> {
    parse_separated_terminated(
        parse_rule_chain,
        char('|').delimited_by(space1),
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
    fn build_pattern_for<'a>(&'a self, id: &RuleID, special: bool) -> impl Display + 'a {
        let rule = self.rules.get(id).unwrap();
        let id = id.id;

        // Rather than do the "correct" thing and create a stack machine, we're
        // gonna repeat the regex in on itself a few hundred times
        lazy_format! {
            match ((special, id)) {
                (true, 8) => ("(?:{})+", rule.build_pattern(self, special)),
                (true, 11) => ("{}", make_lazy_format!(fmt => {
                    let (prefix, suffix) = rule.extract_rule_11_patterns(self, special);
                    write!(fmt, "(?:{}", prefix)?;

                    for _ in 0..100 {
                        write!(fmt, "(?:{}", prefix)?;
                    }

                    for _ in 0..100 {
                        write!(fmt, "{})?", suffix)?;
                    }

                    write!(fmt, "{})", suffix)
                })),
                _ => ("{}", rule.build_pattern(self, special)),
            }
        }
    }

    fn build_regex(&self, special: bool) -> Regex {
        let pattern = format!("^{}$", self.build_pattern_for(&RuleID { id: 0 }, special));
        eprintln!("{}", pattern);
        RegexBuilder::new(&pattern)
            .nest_limit(2000)
            .build()
            .unwrap()
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
    let pattern = rules.build_regex(false);

    let matching = lines.iter().filter(|line| pattern.is_match(line)).count();

    Ok(matching)
}

pub fn part2(input: &str) -> anyhow::Result<usize> {
    let (rules, lines) = parse_input(input).context("Failed to parse input")?;
    let pattern = rules.build_regex(true);

    let matching = lines.iter().filter(|line| pattern.is_match(line)).count();

    Ok(matching)
}
