use std::{
    collections::HashSet,
    hash::{Hash, Hasher},
};

use anyhow::Context;
use cascade::cascade;
use itertools::Itertools;
use nom::{
    bytes::complete::is_not,
    character::complete::{char, digit1, multispace0, space0, space1},
    combinator::peek,
    sequence::{separated_pair, tuple},
    IResult, Parser,
};
use nom_supreme::{
    error::ErrorTree,
    final_parser::{final_parser, Location},
    multi::parse_separated_terminated,
    parse_from_str,
    parser_ext::ParserExt,
    tag::complete::tag,
};

#[derive(Debug, Clone)]
struct RangeInclusive {
    min: i64,
    max: i64,
}

impl RangeInclusive {
    fn is_valid(&self, value: i64) -> bool {
        self.min <= value && value <= self.max
    }
}

fn parse_number(input: &str) -> IResult<&str, i64, ErrorTree<&str>> {
    parse_from_str(digit1).context("number").parse(input)
}

fn parse_range(input: &str) -> IResult<&str, RangeInclusive, ErrorTree<&str>> {
    separated_pair(parse_number, char('-'), parse_number)
        .map(|(min, max)| RangeInclusive { min, max })
        .context("range")
        .parse(input)
}

#[derive(Debug, Clone)]
struct Rule {
    ranges: Vec<RangeInclusive>,
}

impl Rule {
    fn is_valid(&self, value: i64) -> bool {
        self.ranges.iter().any(|range| range.is_valid(value))
    }
}

fn parse_rule(input: &str) -> IResult<&str, Rule, ErrorTree<&str>> {
    parse_separated_terminated(
        parse_range,
        tag("or").delimited_by(space1),
        peek(char('\n')),
        Vec::new,
        |vec, range| cascade! {vec; ..push(range);},
    )
    .map(|ranges| Rule { ranges })
    .context("rule")
    .parse(input)
}

#[derive(Debug, Clone)]
struct FieldRule<'a> {
    name: &'a str,
    rule: Rule,
}

impl PartialEq for FieldRule<'_> {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name
    }
}

impl Eq for FieldRule<'_> {}

impl Hash for FieldRule<'_> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.name.hash(state)
    }
}

impl FieldRule<'_> {
    fn is_valid(&self, value: i64) -> bool {
        self.rule.is_valid(value)
    }
}

fn parse_field_rule(input: &str) -> IResult<&str, FieldRule, ErrorTree<&str>> {
    is_not(":")
        .context("field rule name")
        .terminated(char(':').and(space0))
        .and(parse_rule)
        .map(|(name, rule)| FieldRule { name, rule })
        .context("field rule")
        .parse(input)
}

fn parse_field_rules(input: &str) -> IResult<&str, Vec<FieldRule>, ErrorTree<&str>> {
    parse_separated_terminated(
        parse_field_rule,
        char('\n'),
        peek(tag("\n\n")),
        Vec::new,
        |vec, rule| cascade! {vec; ..push(rule);},
    )
    .context("field rule list")
    .parse(input)
}

#[derive(Debug, Clone)]
struct Ticket {
    fields: Vec<i64>,
}

fn parse_ticket(input: &str) -> IResult<&str, Ticket, ErrorTree<&str>> {
    parse_separated_terminated(
        parse_number,
        char(','),
        peek(char('\n')),
        Vec::new,
        |vec, field| cascade! {vec; ..push(field);},
    )
    .map(|fields| Ticket { fields })
    .context("ticket")
    .parse(input)
}

fn parse_my_ticket(input: &str) -> IResult<&str, Ticket, ErrorTree<&str>> {
    parse_ticket
        .preceded_by(tag("your ticket:\n"))
        .context("your ticket")
        .parse(input)
}

fn parse_nearby_tickets(input: &str) -> IResult<&str, Vec<Ticket>, ErrorTree<&str>> {
    parse_separated_terminated(
        parse_ticket,
        char('\n'),
        multispace0.all_consuming(),
        Vec::new,
        |vec, ticket| cascade! {vec; ..push(ticket);},
    )
    .preceded_by(tag("nearby tickets:\n"))
    .context("nearby tickets")
    .parse(input)
}

#[derive(Debug, Clone)]
struct Day16Data<'a> {
    rules: Vec<FieldRule<'a>>,
    your_ticket: Ticket,
    nearby_tickets: Vec<Ticket>,
}

fn parse_day16_input(input: &str) -> Result<Day16Data, ErrorTree<Location>> {
    final_parser(
        tuple((
            parse_field_rules.terminated(tag("\n\n")),
            parse_my_ticket.terminated(tag("\n\n")),
            parse_nearby_tickets,
        ))
        .context("day 16 input")
        .map(|(rules, your_ticket, nearby_tickets)| Day16Data {
            rules,
            your_ticket,
            nearby_tickets,
        }),
    )(input)
}

pub fn part1(input: &str) -> anyhow::Result<i64> {
    let input = parse_day16_input(input).context("Failed to parse input")?;

    let result = input
        .nearby_tickets
        .iter()
        .flat_map(|ticket| ticket.fields.iter().copied())
        .filter(|&field| input.rules.iter().all(|rule| !rule.is_valid(field)))
        .sum();

    Ok(result)
}

pub fn part2(input: &str) -> anyhow::Result<i64> {
    let input = parse_day16_input(input).context("Failed to parse input")?;

    let filtered_tickets = input.nearby_tickets.iter().filter(|&ticket| {
        ticket
            .fields
            .iter()
            .all(|&field| input.rules.iter().any(|rule| rule.is_valid(field)))
    });

    let mut possibility_space: Vec<HashSet<&FieldRule>> = input
        .rules
        .iter()
        .map(|_| input.rules.iter().collect())
        .collect();

    filtered_tickets.for_each(|ticket| {
        ticket
            .fields
            .iter()
            .zip(&mut possibility_space)
            .for_each(|(&field, possible_set)| {
                possible_set.retain(|&candidate| candidate.is_valid(field))
            })
    });

    let mut computed_rule_positions: Vec<Option<&FieldRule>> = vec![None; possibility_space.len()];

    for _ in 0..computed_rule_positions.len() {
        let (idx, rule) = possibility_space
            .iter()
            .enumerate()
            .find_map(|(idx, rules)| rules.iter().exactly_one().ok().map(|&rule| (idx, rule)))
            .context("No unique solution")?;

        computed_rule_positions[idx] = Some(rule);
        possibility_space.iter_mut().for_each(|rules| {
            rules.remove(rule);
        });
    }

    let computed_rule_positions: anyhow::Result<Vec<&FieldRule>> = computed_rule_positions
        .into_iter()
        .enumerate()
        .map(|(idx, rules)| rules.with_context(|| format!("No unique solution for index {}", idx)))
        .collect();

    let computed_rule_positions = computed_rule_positions?;

    let result = input
        .your_ticket
        .fields
        .iter()
        .zip(&computed_rule_positions)
        .filter(|&(_field, &rule)| rule.name.starts_with("departure"))
        .map(|(&field, _rule)| field)
        .product();

    Ok(result)
}
