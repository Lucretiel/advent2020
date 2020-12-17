use std::collections::HashMap;

use anyhow::Context;

use crate::library::parse_items;

fn solve_nth(input: &str, target: usize) -> anyhow::Result<usize> {
    let values: Vec<usize> =
        parse_items(input.split(',').map(|s| s.trim())).context("Failed to parse input")?;

    let mut records: HashMap<usize, (usize, usize)> = values
        .iter()
        .enumerate()
        .map(|(idx, &value)| (value, (idx, idx)))
        .collect();

    let mut last_said = *values.last().unwrap();

    for step in values.len()..target {
        let &(last_seen1, last_seen2) = records.get(&last_said).context("Unrecorded value")?;
        last_said = last_seen1 - last_seen2;
        match records.entry(last_said) {
            std::collections::hash_map::Entry::Occupied(mut entry) => {
                let &(s1, _s2) = entry.get();
                *entry.get_mut() = (step, s1);
            }
            std::collections::hash_map::Entry::Vacant(entry) => {
                entry.insert((step, step));
            }
        }
    }

    Ok(last_said)
}

pub fn part1(input: &str) -> anyhow::Result<usize> {
    solve_nth(input, 2020)
}

pub fn part2(input: &str) -> anyhow::Result<usize> {
    solve_nth(input, 30000000)
}
