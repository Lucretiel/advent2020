use std::{
    collections::{BTreeSet, HashMap},
    convert::Infallible,
    ops::Bound,
};

use anyhow::{bail, Context};

use crate::library::{
    dynamic::{execute, Subtask, Task, TaskInterrupt},
    parse_items_ws, BoolExt,
};

#[derive(Debug, Clone, Copy, Default)]
struct CollectDiffs {
    ones: i32,
    threes: i32,
}

fn get_all_joltages(input: &'static str) -> anyhow::Result<BTreeSet<i64>> {
    let mut values: BTreeSet<i64> = parse_items_ws(input).context("Failed to parse input")?;
    values.insert(0);
    values.insert(values.iter().copied().max().unwrap() + 3);
    Ok(values)
}

pub fn part1(input: &'static str) -> anyhow::Result<i32> {
    let values = get_all_joltages(input)?;

    let j1_iter = values.iter().copied();
    let mut j2_iter = values.iter().copied();
    j2_iter.next();

    let diffs = j1_iter
        .zip(j2_iter)
        .map(|(j1, j2)| (j1, j2, j2 - j1))
        .try_fold(
            CollectDiffs::default(),
            |diffs, (j1, j2, diff)| match diff {
                0 => bail!("Unexpected identical adapters: {}", j1),
                1 => Ok(CollectDiffs {
                    ones: diffs.ones + 1,
                    threes: diffs.threes,
                }),
                2 => Ok(diffs),
                3 => Ok(CollectDiffs {
                    ones: diffs.ones,
                    threes: diffs.threes + 1,
                }),
                _ => bail!("Too large a gap between {} and {}", j1, j2),
            },
        )
        .context("Failed to compute all diffs")?;

    Ok(diffs.ones * diffs.threes)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct Part2Goal {
    joltage: i64,
    present: bool,

    prev: i64,
}

impl Part2Goal {
    // Returns the next highest with present and absent in that order
    #[inline]
    fn next_highest(&self, next_highest: i64) -> (Self, Self) {
        let present = Part2Goal {
            joltage: next_highest,
            present: true,

            prev: (self.present).then_some(self.joltage).unwrap_or(self.prev),
        };

        let absent = Part2Goal {
            joltage: next_highest,
            present: false,

            prev: (self.present).then_some(self.joltage).unwrap_or(self.prev),
        };

        (present, absent)
    }
}

struct Part2Solver {
    joltages: BTreeSet<i64>,
}

impl Task<Part2Goal, i64, Infallible> for Part2Solver {
    type State = i64;

    fn solve<'sub, T>(
        &self,
        goal: &Part2Goal,
        subtasker: &'sub T,
        state: &mut Option<i64>,
    ) -> Result<i64, TaskInterrupt<'sub, Part2Goal, Infallible>>
    where
        T: Subtask<Part2Goal, i64>,
    {
        let next_highest = match *state {
            Some(next_highest) => next_highest,
            None => {
                let next_highest = self
                    .joltages
                    .range((Bound::Excluded(goal.joltage), Bound::Unbounded))
                    .next()
                    .copied();

                let next_highest = match next_highest {
                    None => {
                        return match goal.present {
                            true => Ok(1),
                            false => Ok(0),
                        }
                    }
                    Some(next_highest) => next_highest,
                };

                if !goal.present && next_highest - goal.prev > 3 {
                    return Ok(0);
                }

                // At this point, we know we're solving subtasks, so save the
                // state so that we can jump back to this point later
                *state = Some(next_highest);
                next_highest
            }
        };

        let (next_present, next_absent) = goal.next_highest(next_highest);
        let &num_arrangements_next_present = subtasker.solve(next_present)?;
        let &num_arrangements_next_absent = subtasker.solve(next_absent)?;

        Ok(num_arrangements_next_present + num_arrangements_next_absent)
    }
}

pub fn part2(input: &'static str) -> anyhow::Result<i64> {
    let task = Part2Solver {
        joltages: get_all_joltages(input)?,
    };

    let goal = Part2Goal {
        joltage: 0,
        present: true,
        prev: 0,
    };

    let store = HashMap::new();

    let solution = execute(goal, &task, store)?;

    Ok(solution)
}
