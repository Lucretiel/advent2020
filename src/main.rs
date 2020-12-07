#![allow(unstable_name_collisions)]

mod common;
// mod nom_helpers;

mod day1;
mod day2;
mod day3;
mod day4;
mod day5;
mod day6;

use std::{
    fs,
    io::{self, Read},
    num::ParseIntError,
    path::PathBuf,
    str::FromStr,
};

use anyhow::Context;
use structopt::StructOpt;
use thiserror::Error;

#[derive(Debug, Clone, Error)]
pub enum SolutionDayError {
    #[error("Failed to parse day: {0}")]
    Parse(#[from] ParseIntError),

    #[error("{0} is not an Adevent Puzzle Day")]
    BadDay(u8),
}

macro_rules! solution_days {
    (
        $($Day:ident)*
    ) => {
        #[allow(non_camel_case_types)]
        #[derive(Debug, Clone, Copy, PartialEq, Eq)]
        pub enum SolutionDay {
            $($Day,)*
        }

        impl FromStr for SolutionDay {
            type Err = SolutionDayError;

            fn from_str(s: &str) -> Result<Self, Self::Err> {
                let value: u8 = s.parse()?;

                let candidate = 1;

                $(
                    #[allow(unused_variables)]
                    let candidate = {
                        if value == candidate {
                            return Ok(SolutionDay::$Day)
                        } else {
                            candidate + 1
                        }
                    };
                )*

                Err(SolutionDayError::BadDay(value))
            }
        }
    };
}

solution_days! {
    day1
    day2
    day3
    day4
    day5
    day6
    day7
    day8
    day9
    day10
    day11
    day12
    day13
    day14
    day15
    day16
    day17
    day18
    day19
    day20
    day21
    day22
    day23
    day24
    day25
}

#[derive(Debug, Clone, Error)]
pub enum SolutionPartError {
    #[error("Failed to parse day: {0}")]
    Parse(#[from] ParseIntError),

    #[error("{0} is not an Advent Puzzle Part; must be 1 or 2")]
    BadPart(u8),
}
#[allow(non_camel_case_types)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SolutionPart {
    part1,
    part2,
}

impl FromStr for SolutionPart {
    type Err = SolutionPartError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let value: u8 = s.parse()?;

        match value {
            1 => Ok(SolutionPart::part1),
            2 => Ok(SolutionPart::part2),
            value => Err(SolutionPartError::BadPart(value)),
        }
    }
}

/// Lucretiel's solutions for the Advent of Code, 2020.
#[derive(Debug, StructOpt)]
struct Args {
    /// Which day's solution are you looking for?
    #[structopt(short, long)]
    pub day: SolutionDay,

    /// Part 1 or Part 2?
    #[structopt(short, long)]
    pub part: SolutionPart,

    /// The file from which to read input. If omitted, we read from stdin.
    pub input: Option<PathBuf>,
}

macro_rules! solver_picker {
    ($day:expr, $part:expr, $input:expr; $(
        $Day:ident, $Part:ident;
    )*) => {{

        #[allow(unreachable_patterns)]
        match ($day, $part) {
            $(
                (SolutionDay::$Day, SolutionPart::$Part) => println!("{}", $Day::$Part($input)?),
            )*
            (day, part) => anyhow::bail!("No solution for {:?}, {:?}", day, part),
        }
    }};
}

fn main() -> anyhow::Result<()> {
    let args: Args = Args::from_args();

    let mut input = String::new();

    match args.input {
        Some(path) => {
            let mut file = fs::File::open(&path)
                .with_context(|| format!("Failed to open input file '{}'", path.display()))?;

            file.read_to_string(&mut input)
                .with_context(|| format!("Failed to read from input file '{}'", path.display()))?;
        }
        None => {
            io::stdin()
                .read_to_string(&mut input)
                .context("Failed to read input from stdin")?;
        }
    }

    solver_picker! (
        args.day, args.part, &input;

        day1, part1;
        day1, part2;
        day2, part1;
        day2, part2;
        day3, part1;
        day3, part2;
        day4, part1;
        day4, part2;
        day5, part1;
        day5, part2;
        day6, part1;
        day6, part2;
    );

    Ok(())
}
