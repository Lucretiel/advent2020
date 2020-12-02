use anyhow::Context;
use std::fmt::Display;

use crate::common::parse_items;

pub fn part1(input: &str) -> anyhow::Result<impl Display> {
    let values: Vec<i64> = parse_items(input)?;

    values
        .iter()
        .find_map(|&value1| {
            let &value2 = values.iter().find(|&value2| value1 + value2 == 2020)?;
            Some(value1 * value2)
        })
        .context("The problem has no solution!")
}

pub fn part2(input: &str) -> anyhow::Result<impl Display> {
    let values: Vec<i64> = parse_items(input)?;

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
