use std::{cmp::max, collections::HashMap, fmt::Display};

use lazy_format::{lazy_format, make_lazy_format};

#[derive(Debug, Clone, Default)]
struct CupNode {
    next: usize,
    prev: usize,
}

#[derive(Debug, Default, Clone)]
struct CupLoop {
    cups: Vec<CupNode>,
}

impl CupLoop {
    fn resize_for(&mut self, cup: usize) {
        if cup >= self.cups.len() {
            self.cups.resize_with(cup + 1, Default::default);
        }
    }

    fn insert_free(&mut self, cup: usize) {
        let node = CupNode {
            next: cup,
            prev: cup,
        };

        self.resize_for(cup);
        self.cups[cup] = node;
    }

    fn insert_after(&mut self, dest: usize, cup: usize) {
        self.resize_for(cup);

        let before_node_id = dest;
        let before_node = &mut self.cups[before_node_id];

        let after_node_id = before_node.next;
        before_node.next = cup;

        let after_node = &mut self.cups[after_node_id];
        after_node.prev = cup;

        let node = CupNode {
            prev: before_node_id,
            next: after_node_id,
        };

        self.cups[cup] = node;
    }

    /// Returns the ID of the next cup
    fn remove(&mut self, cup: usize) -> Option<usize> {
        let node = self.cups[cup].clone();

        let before_node_id = node.prev;
        let after_node_id = node.next;

        self.cups[before_node_id].next = after_node_id;
        self.cups[after_node_id].prev = before_node_id;

        Some(after_node_id)
    }

    fn next_cup(&self, cup: usize) -> usize {
        self.cups[cup].next
    }

    fn print(self) -> impl Display {
        lazy_format!("{}", self.print_ref())
    }

    fn print_ref(&self) -> impl Display + '_ {
        make_lazy_format!(fmt => {
            let mut cup = 1;

            for _ in 0..8 {
                cup = self.next_cup(cup);
                write!(fmt, "{}", cup)?;
            }

            Ok(())
        })
    }
}

fn run_simulation(input: &str, max_cup: usize, rounds: usize) -> CupLoop {
    let mut input_cups = input
        .trim()
        .chars()
        .map(|c| c.to_digit(10).unwrap() as usize);

    let mut cups = CupLoop::default();

    let first_cup = input_cups.next().unwrap();

    {
        cups.insert_free(first_cup);
        let mut current_cup = first_cup;

        for next_cup in input_cups {
            cups.insert_after(current_cup, next_cup);
            current_cup = next_cup;
        }

        for i in 10..=max_cup {
            cups.insert_after(current_cup, i);
            current_cup = i
        }
    }

    let mut current_cup = first_cup;

    for _ in 0..rounds {
        let next1 = cups.next_cup(current_cup);
        let next2 = cups.remove(next1).unwrap();
        let next3 = cups.remove(next2).unwrap();
        cups.remove(next3).unwrap();

        let dest = {
            let mut candidate = current_cup;
            loop {
                candidate -= 1;
                if candidate < 1 {
                    candidate = max_cup
                }
                if candidate == next1 {
                    continue;
                }
                if candidate == next2 {
                    continue;
                }
                if candidate == next3 {
                    continue;
                }
                break candidate;
            }
        };

        cups.insert_after(dest, next1);
        cups.insert_after(next1, next2);
        cups.insert_after(next2, next3);

        current_cup = cups.next_cup(current_cup);
    }

    cups
}

pub fn part1(input: &str) -> anyhow::Result<impl Display> {
    let result = run_simulation(input, 9, 100);
    Ok(result.print())
}

pub fn part2(input: &str) -> anyhow::Result<String> {
    let result = run_simulation(input, 1_000_000, 10_000_000);
    let winner1 = result.next_cup(1);
    let winner2 = result.next_cup(winner1);

    Ok(format!(
        "W1: {}, W2: {}, product: {}",
        winner1,
        winner2,
        winner1 * winner2,
    ))
}
