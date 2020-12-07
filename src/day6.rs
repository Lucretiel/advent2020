use std::collections::HashSet;

pub fn part1(input: &str) -> anyhow::Result<usize> {
    Ok(input
        .split("\n\n")
        .map(|group| -> HashSet<char> {
            group.chars().filter(|c| c.is_ascii_alphabetic()).collect()
        })
        .map(|questions| questions.len())
        .sum())
}

pub fn part2(input: &str) -> anyhow::Result<usize> {
    Ok(input
        .split("\n\n")
        .map(|group| {
            let mut people = group
                .split_whitespace()
                .map(|person| -> HashSet<char> { person.chars().collect() });

            match people.next() {
                None => HashSet::new(),
                Some(set) => people.fold(set, |current_set, person| {
                    current_set.intersection(&person).copied().collect()
                }),
            }
            .len()
        })
        .sum())
}
