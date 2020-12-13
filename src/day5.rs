use std::str::FromStr;

use anyhow::Context;
use thiserror::Error;

use crate::library::{parse_items_ws, BoolExt};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Instruction {
    Low,
    High,
}

use Instruction::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct BoardingPass {
    row: [Instruction; 7],
    column: [Instruction; 3],
}

impl BoardingPass {
    fn seat_id(&self) -> Option<i32> {
        let row = evaluate_instructions(self.row.iter().copied(), Range { min: 0, max: 128 })?;
        let column = evaluate_instructions(self.column.iter().copied(), Range { min: 0, max: 8 })?;
        Some((row * 8) + column)
    }
}

impl Default for BoardingPass {
    fn default() -> Self {
        BoardingPass {
            row: [Low; 7],
            column: [Low; 3],
        }
    }
}

#[derive(Debug, Clone, Error)]
enum BoardingPassParseError {
    #[error("unexpected character {character:?} at row index {idx}")]
    Row { idx: usize, character: char },
    #[error("unexpected character {character:?} at column index {idx}")]
    Column { idx: usize, character: char },
}

impl FromStr for BoardingPass {
    type Err = BoardingPassParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut this = BoardingPass::default();

        this.row
            .iter_mut()
            .zip(s[..7].chars())
            .enumerate()
            .try_for_each(|(idx, (slot, character))| {
                *slot = match character {
                    'F' => Low,
                    'B' => High,
                    _ => return Err(BoardingPassParseError::Row { character, idx }),
                };
                Ok(())
            })?;

        this.column
            .iter_mut()
            .zip(s[7..].chars())
            .enumerate()
            .try_for_each(|(idx, (slot, character))| {
                *slot = match character {
                    'L' => Low,
                    'R' => High,
                    _ => return Err(BoardingPassParseError::Column { character, idx }),
                };
                Ok(())
            })?;

        Ok(this)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
struct Range {
    min: i32,
    max: i32,
}

impl Range {
    fn span(&self) -> i32 {
        self.max - self.min
    }

    fn get(&self) -> Option<i32> {
        (self.span() == 1).then_some(self.min)
    }

    fn apply_instruction(self, instruction: Instruction) -> Self {
        let span = self.span();
        let half_span = span / 2;

        match instruction {
            Low => Self {
                min: self.min,
                max: self.min + half_span,
            },
            High => Self {
                max: self.max,
                min: self.max - half_span,
            },
        }
    }
}

fn evaluate_instructions(
    instructions: impl IntoIterator<Item = Instruction>,
    range: Range,
) -> Option<i32> {
    instructions
        .into_iter()
        .fold(range, |range, instruction| {
            range.apply_instruction(instruction)
        })
        .get()
}

pub fn part1(input: &str) -> anyhow::Result<i32> {
    let boarding_passes: Vec<BoardingPass> = parse_items_ws(input)?;

    boarding_passes
        .into_iter()
        .map(|pass| (pass, pass.seat_id()))
        .try_fold(0, |current_id, (pass, id)| {
            id.map(|id| current_id.max(id))
                .with_context(|| format!("couldn't get seat id for {:?}", pass))
        })
}

pub fn part2(input: &str) -> anyhow::Result<i32> {
    let boarding_passes: Vec<BoardingPass> = parse_items_ws(input)?;
    let seat_ids: anyhow::Result<Vec<i32>> = boarding_passes
        .into_iter()
        .map(|pass| {
            pass.seat_id()
                .with_context(|| format!("Couldn't get seat id for {:?}", pass))
        })
        .collect();
    let mut seat_ids = seat_ids?;

    seat_ids.sort_unstable();
    seat_ids
        .windows(2)
        .find_map(|window| {
            let s1 = window[0];
            let s2 = window[1];
            (s1 + 2 == s2).then(|| s1 + 1)
        })
        .context("Couldn't find seat")
}
