use std::unreachable;

use anyhow::Context;

fn extract_loop_number(subject: i64, target: i64) -> i64 {
    let mut value = 1;

    for i in 0.. {
        if value == target {
            return i;
        }

        value *= subject;
        value %= 20201227;
    }

    unreachable!()
}

fn perform_operation(subject: i64, loop_size: i64) -> i64 {
    let mut value = 1;

    for _ in 0..loop_size {
        value *= subject;
        value %= 20201227;
    }

    value
}

pub fn part1(input: &str) -> anyhow::Result<i64> {
    let mut values = input.trim().split_whitespace();
    let card_pub = values
        .next()
        .context("No value for card")?
        .parse()
        .context("couldn't parse card")?;

    let door_pub = values
        .next()
        .context("No value for door")?
        .parse()
        .context("couldn't parse door")?;

    let card_loop = extract_loop_number(7, card_pub);
    let door_loop = extract_loop_number(7, door_pub);

    let encrypt = perform_operation(perform_operation(7, card_loop), door_loop);

    Ok(encrypt)
}

pub fn part2(input: &str) -> anyhow::Result<i64> {
    todo!()
}
