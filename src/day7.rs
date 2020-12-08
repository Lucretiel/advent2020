use std::collections::{HashMap, HashSet};

use anyhow::Context;
use nom::{
    branch::alt,
    bytes::complete::take_until,
    character::complete::{char, digit1, multispace1, space1},
    combinator::value,
    error::context,
    error::{ErrorKind, FromExternalError},
    multi::separated_list1,
    sequence::{pair, separated_pair, terminated, tuple},
    Err, IResult, Parser,
};
use thiserror::Error;

use crate::library::nom::{final_str_parser, parse_from_str, tag, NomError};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct Bag<'a> {
    name: &'a str,
}

/// Parse a string like "red bag" or "light green bags"
fn parse_bag(input: &str) -> IResult<&str, Bag, NomError<&str>> {
    context(
        "bag",
        terminated(take_until("bag"), alt((tag("bags"), tag("bag"))))
            .map(|s: &str| s.trim_end())
            .map(|name| Bag { name }),
    )
    .parse(input)
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct BagRule<'a> {
    contents: HashMap<Bag<'a>, usize>,
}

/// Parse a string like "1 red bag, 2 green bags."
fn parse_bag_rule(input: &str) -> IResult<&str, BagRule, NomError<&str>> {
    context(
        "bag rule",
        terminated(
            alt((
                value(Vec::new(), tag("no other bags")),
                separated_list1(
                    pair(char(','), space1),
                    separated_pair(parse_from_str(digit1), space1, parse_bag),
                ),
            )),
            pair(char('.'), multispace1),
        ),
    )
    .map(|rules_list| BagRule {
        contents: rules_list
            .into_iter()
            .map(|(count, bag)| (bag, count))
            .collect(),
    })
    .parse(input)
}

fn parse_bag_with_rule(input: &str) -> IResult<&str, (Bag, BagRule), NomError<&str>> {
    context(
        "bag with rule",
        separated_pair(
            parse_bag,
            tuple((space1, tag("contain"), space1)),
            parse_bag_rule,
        ),
    )
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

fn parse_all_rules(mut input: &str) -> IResult<&str, Rules, NomError<&str>> {
    let mut rules = Rules::default();

    while !input.is_empty() {
        let (tail, (bag, rule)) = parse_bag_with_rule(input)?;

        if rules.bags.insert(bag, rule).is_some() {
            return Err(Err::Error(NomError::from_external_error(
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

const SHINY_GOLD: Bag = Bag { name: "shiny gold" };

pub fn part1(input: &str) -> anyhow::Result<usize> {
    let rules = final_str_parser(parse_all_rules)(input)?;

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
