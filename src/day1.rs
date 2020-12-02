use std::collections::HashSet;

use anyhow::Context;

use crate::common::parse_items;

fn solve_recursive(values: &HashSet<i64>, target: i64, depth: u32) -> Option<i64> {
    match depth {
        0 => None,
        1 => values.get(&target).copied(),
        depth => values
            .iter()
            .filter(|&&value| value < target)
            .find_map(|&value| {
                solve_recursive(values, target - value, depth - 1).map(|solution| solution * value)
            }),
    }
}

fn solve(input: &str, depth: u32) -> anyhow::Result<i64> {
    let values: HashSet<i64> = parse_items(input)?;

    solve_recursive(&values, 2020, depth).context("The problem has no solution!")
}

pub fn part1(input: &str) -> anyhow::Result<i64> {
    solve(input, 2)
}

pub fn part2(input: &str) -> anyhow::Result<i64> {
    solve(input, 3)
}
