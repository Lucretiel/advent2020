use std::collections::HashMap;

use anyhow::Context;
use nom::{
    branch::alt,
    character::complete::{char, digit1, multispace0, multispace1},
    multi::fold_many_m_n,
    sequence::separated_pair,
    IResult, Parser,
};
use nom_supreme::{
    error::ErrorTree,
    final_parser::{final_parser, Location},
    multi::parse_separated_terminated,
    parse_from_str,
    parser_ext::ParserExt,
    tag::complete::tag,
};

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
enum MaskBit {
    Ignore,
    Set,
    Clear,
}
/*

1001XX01

mask =    00001100
setting = 10010001

*/

impl Default for MaskBit {
    fn default() -> Self {
        MaskBit::Ignore
    }
}

fn parse_mask_bit(input: &str) -> IResult<&str, MaskBit, ErrorTree<&str>> {
    alt((
        char('X').value(MaskBit::Ignore),
        char('0').value(MaskBit::Clear),
        char('1').value(MaskBit::Set),
    ))
    .context("mask bit")
    .parse(input)
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
struct Mask {
    mask: i64,
    setting: i64,
}

impl Mask {
    fn apply(&self, value: i64) -> i64 {
        (self.mask & value) | self.setting
    }
}

fn parse_mask(input: &str) -> IResult<&str, Mask, ErrorTree<&str>> {
    fold_many_m_n(
        36,
        36,
        parse_mask_bit,
        (Mask::default(), 1i64 << 36),
        |(mut mask, idx), maskbit| {
            let idx = idx >> 1;
            match maskbit {
                MaskBit::Ignore => mask.mask |= idx,
                MaskBit::Set => mask.setting |= idx,
                MaskBit::Clear => {}
            };
            (mask, idx)
        },
    )
    .map(|(mask, _)| mask)
    .context("mask")
    .parse(input)
}

#[test]
fn test_parse_mask() {
    let input = concat!("XXXXXX", "XXXXXX", "XXXXXX", "XXXXXX", "010101", "01XX01");

    let (tail, mask) = parse_mask(input).expect("Error parsing mask");

    assert_eq!(tail, "");
    assert_eq!(
        mask,
        Mask {
            mask: 0b111111_111111_111111_111111_000000_001100,
            setting: 0b000000_000000_000000_000000_010101_010001,
        }
    );
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Write {
    destination: usize,
    value: i64,
}

fn parse_write(input: &str) -> IResult<&str, Write, ErrorTree<&str>> {
    separated_pair(
        parse_from_str(digit1).delimited_by(char('['), char(']')),
        tag(" = "),
        parse_from_str(digit1),
    )
    .map(|(destination, value)| Write { destination, value })
    .context("write")
    .parse(input)
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Instruction<M> {
    SetMask(M),
    Write(Write),
}

fn parse_instruction<'a, M>(
    parse_mask: impl Parser<&'a str, M, ErrorTree<&'a str>>,
) -> impl Parser<&'a str, Instruction<M>, ErrorTree<&'a str>> {
    alt((
        tag("mem")
            .precedes(parse_write.cut())
            .map(Instruction::Write),
        tag("mask = ")
            .precedes(parse_mask.cut())
            .map(Instruction::SetMask),
    ))
    .context("instruction")
}

#[derive(Debug, Default, Clone)]
struct Machine {
    mask: Mask,
    memory: Vec<i64>,
}

impl Machine {
    fn set_mask(&mut self, mask: Mask) {
        self.mask = mask
    }

    fn write(&mut self, write: Write) {
        let Write { destination, value } = write;

        if destination >= self.memory.len() {
            self.memory.resize(destination + 1, 0);
        }

        let value = self.mask.apply(value);

        self.memory[destination] = value;
    }

    fn exec(&mut self, instruction: Instruction<Mask>) {
        match instruction {
            Instruction::SetMask(mask) => self.set_mask(mask),
            Instruction::Write(write) => self.write(write),
        }
    }
}

pub fn part1(input: &str) -> anyhow::Result<i64> {
    let result: Result<Machine, ErrorTree<Location>> = final_parser(
        parse_separated_terminated(
            parse_instruction(parse_mask),
            multispace1,
            multispace0.all_consuming(),
            Machine::default,
            |mut machine, instruction| {
                machine.exec(instruction);
                machine
            },
        )
        .context("instruction list"),
    )(input);

    result
        .context("Failed to execute machine")
        .map(|machine| machine.memory.iter().copied().sum())
}

#[derive(Debug, Clone)]
struct MemoryMask {
    mask: [MaskBit; 36],
}

impl Default for MemoryMask {
    fn default() -> Self {
        Self {
            mask: [MaskBit::Ignore; 36],
        }
    }
}

#[derive(Debug, Clone, Default)]
struct MachineV2 {
    mask: MemoryMask,
    memory: HashMap<i64, i64>,
}

impl MachineV2 {
    fn set_mask(&mut self, mask: MemoryMask) {
        self.mask = mask
    }

    fn write_recursive(&mut self, value: i64, dest: i64, depth: usize) {
        match self.mask.mask.get(depth) {
            None => {
                // eprintln!("write to {:#b}: {}", dest as i32, value);
                self.memory.insert(dest, value);
            }
            Some(MaskBit::Ignore) => {
                let bit = 1 << (35 - depth);

                let dest = dest | bit;
                self.write_recursive(value, dest, depth + 1);

                let dest = dest - bit;
                self.write_recursive(value, dest, depth + 1);
            }
            Some(MaskBit::Set) => {
                let bit = 1 << (35 - depth);

                let dest = dest | bit;
                self.write_recursive(value, dest, depth + 1);
            }
            Some(MaskBit::Clear) => {
                self.write_recursive(value, dest, depth + 1);
            }
        }
    }

    fn write(&mut self, write: Write) {
        let Write { destination, value } = write;

        self.write_recursive(value, destination as i64, 0);
    }

    fn exec(&mut self, instruction: Instruction<MemoryMask>) {
        match instruction {
            Instruction::SetMask(mask) => self.set_mask(mask),
            Instruction::Write(write) => self.write(write),
        }
    }
}

fn parse_mem_mask(input: &str) -> IResult<&str, MemoryMask, ErrorTree<&str>> {
    fold_many_m_n(
        36,
        36,
        parse_mask_bit,
        (MemoryMask::default(), 0),
        |(mut mask, idx), maskbit| {
            mask.mask[idx] = maskbit;
            (mask, idx + 1)
        },
    )
    .map(|(mask, _)| mask)
    .context("memory mask")
    .parse(input)
}

pub fn part2(input: &str) -> anyhow::Result<i64> {
    let result: Result<MachineV2, ErrorTree<Location>> = final_parser(
        parse_separated_terminated(
            parse_instruction(parse_mem_mask),
            multispace1,
            multispace0.all_consuming(),
            MachineV2::default,
            |mut machine, instruction| {
                machine.exec(instruction);
                machine
            },
        )
        .context("instruction list"),
    )(input);

    result
        .context("Failed to execute machine")
        .map(|machine| machine.memory.values().copied().sum())
}
