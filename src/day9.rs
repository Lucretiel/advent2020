use std::{cmp::Ordering, collections::VecDeque};

use anyhow::{bail, Context};

use crate::library::parse_items_lines;

#[derive(Debug, Clone, Default)]
struct XmasDecoder {
    preamble: VecDeque<i64>,
}

impl XmasDecoder {
    fn reserve(&mut self, size: usize) {
        self.preamble.reserve(size);
    }

    fn seed(&mut self, items: impl Iterator<Item = i64>) {
        self.preamble.extend(items);
    }

    /// Attempt to process a value. If it *is* the sum of two numbers in the
    /// preamble, push it into the back preamble, and pop off the front of the
    /// preamble. Return true if we succeeded at this.
    fn process(&mut self, target: i64) -> bool {
        // I'm not entirely sure what the performance implications are of doing
        // this in a loop. My hope is that, by reserving an amount of memory
        // sufficient for ALL the values (not just the first 25), it won't
        // happen too often.
        //
        // For reference, a VecDeque is implemented as a flat circular array
        // that "wraps around". `make_contiguous` shifts the items in the
        // array such that they form a single, contiguous slice. Done naively,
        // doing this in a loop means that we're basically just doing
        // `vec.remove(0)`, which obviously we don't want to do.
        //
        // We do this because it isn't possible to take a subslice of a plain
        // deque, because the contents might wrap around the back to the front.
        // In the future we'll just use .range, but right now it's nightly only
        let slice: &[i64] = self.preamble.make_contiguous();

        let success = slice
            .iter()
            .copied()
            .enumerate()
            .flat_map(move |(idx, value1)| {
                slice[idx + 1..]
                    .iter()
                    .copied()
                    .map(move |value2| value1 + value2)
            })
            .any(|sum| sum == target);

        if success {
            self.preamble.pop_front();
            self.preamble.push_back(target);
        }

        success
    }
}

/// Rather than hardcode our part 1 solution into part 2, we refactor it out
/// to here so that part 2 can make use of it.
fn solve_part_1(stream: &[i64]) -> anyhow::Result<i64> {
    let mut stream = stream.iter().copied();

    let mut decoder = XmasDecoder::default();
    decoder.reserve(stream.len());
    decoder.seed(stream.by_ref().take(25));

    for value in stream {
        if let false = decoder.process(value) {
            return Ok(value);
        }
    }

    bail!("No solution found");
}

pub fn part1(input: &str) -> anyhow::Result<i64> {
    let stream: Vec<i64> = parse_items_lines(input)?;
    solve_part_1(&stream)
}

pub fn part2(input: &str) -> anyhow::Result<i64> {
    let stream: Vec<i64> = parse_items_lines(input).context("Error parsing input")?;
    let target = solve_part_1(&stream).context("Couldn't get target vulnerability")?;

    let mut rolling_sum = 0;
    let mut head = 0;
    let mut tail = 0;

    /*
    [a, b, c, d, e, f, g]
        ^tail   ^head

    Algorithm: we track a rolling sum of the values between head and tail by
    adding to it when head moves forward and subtracting when tail moves
    forward. head is one-past-the-end, tail is inclusive.

    If the rolling sum is too small, advance the head
    If the rolling sum is too large, advance the tail
    */

    loop {
        match rolling_sum.cmp(&target) {
            Ordering::Less => {
                rolling_sum += stream
                    .get(head)
                    .context("Head pointer advanced past the stream boundary")?;
                head += 1;
            }
            Ordering::Greater => {
                // It shouldn't be possible for this to happen before the tail
                // advances past the boundary, so we do a regular panic-checked
                // index here
                rolling_sum -= stream[tail];
                tail += 1
            }
            Ordering::Equal => {
                let range = stream[tail..head].iter().copied();
                let min = range.clone().min().unwrap();
                let max = range.max().unwrap();
                break Ok(min + max);
            }
        }
    }
}
