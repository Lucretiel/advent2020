use std::{convert::TryInto, iter, num::ParseIntError, str::FromStr};

use anyhow::{bail, Context};
use bitvec::{bitvec, vec::BitVec};

use thiserror::Error;

#[derive(Debug, Clone, Error)]
#[error("Unrecognized Instruction")]
struct BadInstruction;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Instruction {
    Accum,
    Jmp,
    Noop,
}

impl FromStr for Instruction {
    type Err = BadInstruction;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "acc" => Ok(Instruction::Accum),
            "jmp" => Ok(Instruction::Jmp),
            "nop" => Ok(Instruction::Noop),
            _ => Err(BadInstruction),
        }
    }
}

#[derive(Debug, Clone, Error)]
enum BadOperation {
    #[error("Invalid operation format")]
    BadFormat,

    #[error("Error parsing instruction for operation")]
    Instruction(#[from] BadInstruction),

    #[error("Error parsing argument for operation")]
    Argument(#[from] ParseIntError),
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Operation {
    instruction: Instruction,
    argument: i32,
}

impl FromStr for Operation {
    type Err = BadOperation;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut parts = s.split_whitespace();

        let instruction = parts.next().ok_or(BadOperation::BadFormat)?.parse()?;
        let argument = parts.next().ok_or(BadOperation::BadFormat)?.parse()?;

        if parts.next().is_some() {
            return Err(BadOperation::BadFormat);
        }

        Ok(Operation {
            instruction,
            argument,
        })
    }
}

enum MachineTermination {
    Terminated(i32),
    InfiniteLoop(i32),
}

#[derive(Debug, Clone, Error)]
enum MachineError {
    #[error("Instruction pointer out of bounds: {0}")]
    IPOutOfBounds(i32),
}

#[derive(Debug, Clone, Default)]
struct Machine {
    visited: BitVec,
    code: Vec<Operation>,
    ip: i32,
    accum: i32,
}

impl Machine {
    /// Convert the ip to a usize but don't bounds check it
    fn get_just_ip(&self) -> Result<usize, MachineError> {
        self.ip
            .try_into()
            .map_err(|_| MachineError::IPOutOfBounds(self.ip))
    }

    pub fn new(code: Vec<Operation>) -> Self {
        Machine {
            visited: bitvec![0; code.len()],
            code,
            ip: 0,
            accum: 0,
        }
    }

    pub fn step(&mut self) -> Option<Result<MachineTermination, MachineError>> {
        let ip = match self.get_just_ip() {
            Err(err) => return Some(Err(err)),
            Ok(ip) if ip > self.code.len() => {
                return Some(Err(MachineError::IPOutOfBounds(self.ip)))
            }
            Ok(ip) if ip == self.code.len() => {
                return Some(Ok(MachineTermination::Terminated(self.accum)))
            }
            Ok(ip) => ip,
        };

        let visited = self.visited.get_mut(ip).unwrap();

        if *visited {
            return Some(Ok(MachineTermination::InfiniteLoop(self.accum)));
        } else {
            visited.set(true);
        }

        let op = &self.code[ip];

        match op.instruction {
            Instruction::Accum => {
                self.accum += op.argument;
                self.ip += 1;
            }
            Instruction::Jmp => self.ip += op.argument,
            Instruction::Noop => self.ip += 1,
        }

        None
    }

    pub fn run(&mut self) -> Result<MachineTermination, MachineError> {
        loop {
            if let Some(done) = self.step() {
                return done;
            }
        }
    }

    /// Swap a jmp to a nop or a nop to a jmp at the target. Returns None if
    /// the instruction is a acc. Panics if target is out of range.
    pub fn mutate(mut self, target: usize) -> Option<Self> {
        let op = &mut self.code[target];

        op.instruction = match op.instruction {
            Instruction::Accum => return None,
            Instruction::Jmp => Instruction::Noop,
            Instruction::Noop => Instruction::Jmp,
        };

        Some(self)
    }
}

fn load_code(input: &str) -> anyhow::Result<Vec<Operation>> {
    input
        .lines()
        .enumerate()
        .map(|(index, op)| {
            op.parse()
                .with_context(|| format!("error parsing instruction index {}", index))
        })
        .collect()
}

pub fn part1(input: &str) -> anyhow::Result<i32> {
    let program = load_code(input).context("error loading program")?;

    let mut machine = Machine::new(program);

    match machine.run()? {
        MachineTermination::Terminated(..) => bail!("Machine terminated unexpectedly"),
        MachineTermination::InfiniteLoop(value) => Ok(value),
    }
}

// This is a cool multithreaded that nonetheless takes twice as long to finish
// (~25ms vs ~12ms) on my machine
/*
use rayon::prelude::*;
pub fn part2(input: &str) -> anyhow::Result<i32> {
    let program = load_code(input).context("error loading program")?;
    let machine = Machine::new(program);
    let inst_count = machine.code.len();

    let mut candidates: Vec<(usize, Machine)> = iter::repeat(machine)
        .enumerate()
        .take(inst_count)
        .filter_map(|(i, machine)| machine.mutate(i).map(|machine| (i, machine)))
        .collect();

    candidates
        .par_iter_mut()
        .map(|&mut (i, ref mut machine)| (i, machine.run()))
        .find_map_any(|(i, result)| match result {
            Err(err) => {
                Some(Err(err).with_context(|| format!("Machine {} encountered an error", i)))
            }
            Ok(MachineTermination::Terminated(value)) => Some(Ok(value)),
            _ => None,
        })
        // We're unpacking an Option<result<..>> here
        .context("Couldn't find a solution")?
        .context("Machine encountered an error")
}
*/

pub fn part2(input: &str) -> anyhow::Result<i32> {
    let program = load_code(input).context("error loading program")?;
    let machine = Machine::new(program);
    let inst_count = machine.code.len();

    iter::repeat(machine)
        .enumerate()
        .take(inst_count)
        .filter_map(|(i, machine)| machine.mutate(i).map(|machine| (i, machine)))
        .map(|(i, mut machine)| (i, machine.run()))
        .find_map(|(i, result)| match result {
            Err(err) => {
                Some(Err(err).with_context(|| format!("Machine {} encountered an error", i)))
            }
            Ok(MachineTermination::Terminated(value)) => Some(Ok(value)),
            _ => None,
        })
        // We're unpacking an Option<result<..>> here
        .context("Couldn't find a solution")?
        .context("Machine encountered an error")
}
