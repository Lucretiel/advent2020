use std::collections::BTreeSet;

use anyhow::Context;

use crate::common::parse_items;

fn solve_recursive(values: &BTreeSet<i64>, min: i64, target: i64, depth: u32) -> Option<i64> {
    match depth {
        0 => None,
        1 => values.get(&target).copied(),
        depth => values.range(min..target).copied().find_map(|value| {
            solve_recursive(values, value + 1, target - value, depth - 1)
                .map(|solution| value * solution)
        }),
    }
}

fn solve(input: &str, depth: u32) -> anyhow::Result<i64> {
    let values: BTreeSet<i64> = parse_items(input)?;

    solve_recursive(&values, 0, 2020, depth).context("The problem has no solution!")
}

pub fn part1(input: &str) -> anyhow::Result<i64> {
    solve(input, 2)
}

pub fn part2(input: &str) -> anyhow::Result<i64> {
    solve(input, 3)
}
