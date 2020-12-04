use std::convert::TryInto;

use nom::{
    character::complete::multispace1,
    character::complete::space0,
    character::complete::{alpha1, anychar, char, digit1, space1},
    combinator::iterator,
    error::Error,
    sequence::{pair, separated_pair, terminated},
    IResult, Parser,
};

use anyhow::Context;

use crate::common::{parse_from_str, BoolExt};

fn parse_number(input: &str) -> IResult<&str, usize> {
    parse_from_str(digit1).parse(input)
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct Range {
    pub min: usize,
    pub max: usize,
}

impl Range {
    fn in_range(&self, value: usize) -> bool {
        self.min <= value && value <= self.max
    }
}

fn parse_range(input: &str) -> IResult<&str, Range> {
    separated_pair(parse_number, char('-'), parse_number)
        .map(|(min, max)| Range { min, max })
        .parse(input)
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct Policy {
    pub range: Range,
    pub character: char,
}

fn char_at(input: &str, index: usize) -> Option<char> {
    let slice = input.get(index..)?;
    slice.chars().next()
}

impl Policy {
    fn check(&self, password: &str) -> bool {
        let count = password.chars().filter(|&c| c == self.character).count();
        self.range
            .in_range(count.try_into().expect("Overflowed a u32"))
    }

    fn check_version_2(&self, password: &str) -> bool {
        let c1 = char_at(password, self.range.min - 1) == Some(self.character);
        let c2 = char_at(password, self.range.max - 1) == Some(self.character);

        c1 ^ c2
    }
}

fn parse_policy(input: &str) -> IResult<&str, Policy> {
    separated_pair(parse_range, space1, anychar)
        .map(|(range, character)| Policy { range, character })
        .parse(input)
}

fn parse_password(input: &str) -> IResult<&str, &str> {
    alpha1(input)
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Entry<'a> {
    policy: Policy,
    password: &'a str,
}

impl Entry<'_> {
    fn is_valid(&self) -> bool {
        self.policy.check(self.password)
    }

    fn is_valid_v2(&self) -> bool {
        self.policy.check_version_2(self.password)
    }
}

fn parse_entry(input: &str) -> IResult<&str, Entry> {
    let separator = pair(char(':'), space0);
    separated_pair(parse_policy, separator, parse_password)
        .map(|(policy, password)| Entry { policy, password })
        .parse(input)
}

pub fn part1(input: &str) -> anyhow::Result<usize> {
    let mut entries = iterator(input, terminated(parse_entry, multispace1));
    let solution = entries.filter(|entry| entry.is_valid()).count();
    let (tail, ()) = entries
        .finish()
        .map_err(|err| {
            err.map(|inner| Error {
                input: (),
                code: inner.code,
            })
        })
        .context("Error parsing input")?;

    tail.is_empty()
        .then(|| solution)
        .context("Didn't parse all of the input")
}

pub fn part2(input: &str) -> anyhow::Result<usize> {
    let mut entries = iterator(input, terminated(parse_entry, multispace1));
    let solution = entries.filter(|entry| entry.is_valid_v2()).count();
    let (tail, ()) = entries
        .finish()
        .map_err(|err| {
            err.map(|inner| Error {
                input: (),
                code: inner.code,
            })
        })
        .context("Error parsing input")?;

    tail.is_empty()
        .then(|| solution)
        .context("Didn't parse all of the input")
}
