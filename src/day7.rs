use std::collections::{HashMap, HashSet};

use anyhow::Context;
use nom::{
    bytes::complete::take_until,
    character::complete::{char, digit1, multispace1, space0, space1},
    error::{ErrorKind, FromExternalError},
    sequence::separated_pair,
    Err, IResult, Parser,
};
use nom_supreme::{
    error::ErrorTree,
    final_parser::{final_parser, Location},
    multi::parse_separated_terminated,
    parse_from_str,
    parser_ext::ParserExt,
    tag::complete::tag,
};
use thiserror::Error;

use crate::library::{self, dynamic::StatelessTask};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct Bag<'a> {
    name: &'a str,
}

/// Parse a string like "red bag" or "light green bags"
fn parse_bag(input: &str) -> IResult<&str, Bag, ErrorTree<&str>> {
    take_until("bag")
        .terminated(tag("bags").or(tag("bag")))
        .map(|s: &str| s.trim_end())
        .map(|name| Bag { name })
        .context("bag name")
        .parse(input)
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct BagRule<'a> {
    contents: HashMap<Bag<'a>, usize>,
}

/// Parse a string like "1 red bag"
fn parse_counted_bag(input: &str) -> IResult<&str, (usize, Bag), ErrorTree<&str>> {
    separated_pair(parse_from_str(digit1), space1, parse_bag)
        .context("bag count")
        .parse(input)
}

/// Parse a string like "1 red bag, 2 green bags.", or a string like "no other bags."
fn parse_bag_rule(input: &str) -> IResult<&str, BagRule, ErrorTree<&str>> {
    parse_separated_terminated(
        parse_counted_bag,
        char(',').terminated(space0),
        char('.'),
        HashMap::new,
        |mut contents, (count, bag)| {
            contents.insert(bag, count);
            contents
        },
    )
    .or(tag("no other bags.").value(HashMap::new()))
    .terminated(multispace1)
    .map(|contents| BagRule { contents })
    .parse(input)
}

/// Parse a string like "red bags contain 2 blue bags, 1 green bag."
fn parse_bag_with_rule(input: &str) -> IResult<&str, (Bag, BagRule), ErrorTree<&str>> {
    separated_pair(
        parse_bag.context("rule: target"),
        tag("contain").delimited_by_both(space1),
        parse_bag_rule.context("rule: contents"),
    )
    .context("rule")
    .parse(input)
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct Rules<'a> {
    bags: HashMap<Bag<'a>, BagRule<'a>>,
}

#[derive(Debug, Clone, Error)]
#[error("duplicate bag {bag_name:?} while parsing rules")]
struct DuplicateBagError {
    bag_name: String,
}

fn parse_all_rules(mut input: &str) -> IResult<&str, Rules, ErrorTree<&str>> {
    let mut rules = Rules::default();

    while !input.is_empty() {
        let (tail, (bag, rule)) = parse_bag_with_rule(input)?;

        if rules.bags.insert(bag, rule).is_some() {
            return Err(Err::Error(ErrorTree::from_external_error(
                input,
                ErrorKind::Many0,
                DuplicateBagError {
                    bag_name: bag.name.to_owned(),
                },
            )));
        }

        input = tail;
    }

    Ok((input, rules))
}

fn final_parse_all_rules(input: &str) -> Result<Rules, ErrorTree<Location>> {
    final_parser(parse_all_rules)(input)
}

const SHINY_GOLD: Bag = Bag { name: "shiny gold" };

pub fn part1(input: &str) -> anyhow::Result<usize> {
    let rules = final_parse_all_rules(input)?;

    // Set of bags from which "shiny gold" is reachable
    let mut bags: HashSet<Bag> = HashSet::default();

    loop {
        let new_bags: Vec<Bag> = rules
            .bags
            .iter()
            // Don't bother scanning bags we already know are reachable
            .filter(|&(bag, _)| !bags.contains(bag))
            // Can this rule reach anything in bags?
            .filter(|&(_, rule)| {
                rule.contents.contains_key(&SHINY_GOLD)
                    || rule.contents.keys().any(|inner| bags.contains(inner))
            })
            .map(|(&bag, _)| bag)
            .collect();

        if new_bags.is_empty() {
            return Ok(bags.len());
        } else {
            bags.extend(new_bags)
        }
    }
}

/*
fn get_total_bag_count<'a>(
    bag: Bag<'a>,
    rules: &Rules<'a>,
    cache: &mut HashMap<Bag<'a>, usize>,
) -> anyhow::Result<usize> {
    if let Some(&count) = cache.get(&bag) {
        return Ok(count);
    }

    let rule = rules
        .bags
        .get(&bag)
        .with_context(|| format!("No such bag {:?} in rule list", bag.name))?;

    let mut count = 0;

    for (&inner_bag, &num_inner_bags) in &rule.contents {
        let inner_count = get_total_bag_count(inner_bag, rules, cache)
            .with_context(|| format!("error getting count for {:?}", bag.name))?;

        // Add the bags IN inner bag, plus inner_bag itself
        count += (inner_count + 1) * num_inner_bags;
    }

    cache.insert(bag, count);

    Ok(count)
}

pub fn part2(input: &str) -> anyhow::Result<usize> {
    let rules = final_str_parser(parse_all_rules)(input)?;

    let mut cache: HashMap<Bag, usize> = HashMap::with_capacity(rules.bags.len());

    get_total_bag_count(SHINY_GOLD, &rules, &mut cache)
}
*/

use library::dynamic::{execute, Subtask, TaskInterrupt};

struct Day7Solver<'a> {
    rules: Rules<'a>,
}

#[derive(Debug, Error)]
#[error("error getting count for {bag:?}")]
struct NoRule<'a> {
    bag: Bag<'a>,
}

impl<'a> StatelessTask<Bag<'a>, usize, NoRule<'a>> for Day7Solver<'a> {
    fn solve<'sub, T>(
        &self,
        bag: &Bag<'a>,
        subtasker: &'sub T,
    ) -> Result<usize, TaskInterrupt<'sub, Bag<'a>, NoRule<'a>>>
    where
        T: Subtask<Bag<'a>, usize>,
    {
        let rule = self
            .rules
            .bags
            .get(&bag)
            .ok_or(NoRule { bag: *bag })
            .map_err(TaskInterrupt::Error)?;

        subtasker.precheck(rule.contents.keys().copied())?;

        let mut count = 0;
        for (&inner_bag, &num_inner_bags) in &rule.contents {
            let &inner_count = subtasker.solve(inner_bag)?;
            count += (1 + inner_count) * num_inner_bags;
        }

        Ok(count)
    }
}

pub fn part2(input: &'static str) -> anyhow::Result<usize> {
    let rules = final_parse_all_rules(input)?;
    let solver = Day7Solver { rules };

    execute(SHINY_GOLD, &solver, HashMap::new()).context("error solving puzzle")
}
