use anyhow::Context;
use std::{fmt::Display, num::ParseIntError};

pub fn parse_values(input: &str) -> Result<Vec<i64>, ParseIntError> {
    input
        .split_whitespace()
        .map(|value| value.parse())
        .collect()
}

pub fn part1(input: &str) -> anyhow::Result<impl Display> {
    let values = parse_values(input)?;

    values
        .iter()
        .find_map(|&value1| {
            let &value2 = values.iter().find(|&value2| value1 + value2 == 2020)?;
            Some(value1 * value2)
        })
        .context("The problem has no solution!")
}

pub fn part2(input: &str) -> anyhow::Result<impl Display> {
    let values: Vec<i64> = parse_values(input)?;

    values
        .iter()
        .find_map(|&value1| {
            values.iter().find_map(|&value2| {
                let &value3 = values
                    .iter()
                    .find(|&&value3| value1 + value2 + value3 == 2020)?;
                Some(value1 * value2 * value3)
            })
        })
        .context("The problem has no solution")
}
