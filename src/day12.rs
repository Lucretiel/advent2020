use anyhow::Context;
use gridly::prelude::*;
use nom::{
    branch::alt,
    character::complete::{char, digit1, multispace0, multispace1},
    IResult, Parser,
};
use nom_supreme::{
    error::ErrorTree, final_parser::final_parser, multi::parse_separated_terminated,
    parse_from_str, parser_ext::ParserExt, tag::complete::tag,
};

enum Instruction {
    AbsoluteMove(Direction, isize),
    Turn(Rotation),
    MoveForward(isize),
}

use Instruction::*;

fn parse_direction(input: &str) -> IResult<&str, Direction, ErrorTree<&str>> {
    alt((
        char('N').value(Up),
        char('S').value(Down),
        char('W').value(Left),
        char('E').value(Right),
    ))
    .context("direction")
    .parse(input)
}

fn parse_rotation(input: &str) -> IResult<&str, Rotation, ErrorTree<&str>> {
    alt((char('L').value(Anticlockwise), char('R').value(Clockwise)))
        .and(alt((
            tag("90").value(1),
            tag("180").value(2),
            tag("270").value(3),
        )))
        .map(|(rot, amount)| rot * amount)
        .context("rotation")
        .parse(input)
}

fn parse_instruction(input: &str) -> IResult<&str, Instruction, ErrorTree<&str>> {
    alt((
        // Parse an absolute direction (N, E, S, W) and a magnitude
        parse_direction
            .and(parse_from_str(digit1))
            .map(|(direction, distance)| AbsoluteMove(direction, distance))
            .context("absolute movement"),
        // Parse a rotation
        parse_rotation.map(Turn),
        // Parse "F" and a magnitude
        parse_from_str(digit1)
            .preceded_by(char('F'))
            .map(MoveForward)
            .context("relative movement"),
    ))
    .context("instruction")
    .parse(input)
}

trait ApplyInstruction: Sized {
    fn apply_instruction(self, instruction: Instruction) -> Self;
}

#[derive(Debug, Clone)]
struct Ship {
    location: Location,
    facing: Direction,
}

impl ApplyInstruction for Ship {
    fn apply_instruction(self, instruction: Instruction) -> Self {
        match instruction {
            AbsoluteMove(dir, length) => Self {
                location: self.location.relative(dir, length),
                ..self
            },
            Turn(rot) => Self {
                facing: self.facing.rotate(rot),
                ..self
            },
            MoveForward(length) => Self {
                location: self.location.relative(self.facing, length),
                ..self
            },
        }
    }
}

fn execute_ship<T: ApplyInstruction + Clone>(
    ship: T,
    input: &str,
) -> Result<T, ErrorTree<nom_supreme::final_parser::Location>> {
    final_parser(
        parse_separated_terminated(
            parse_instruction,
            multispace1,
            multispace0.all_consuming(),
            || ship.clone(),
            T::apply_instruction,
        )
        .context("instruction list"),
    )(input)
}

pub fn part1(input: &str) -> anyhow::Result<isize> {
    execute_ship(
        Ship {
            location: Location::zero(),
            facing: Right,
        },
        input,
    )
    .context("Failed to execute all instructions")
    .map(|ship| (ship.location - Location::zero()).manhattan_length())
}

#[derive(Debug, Clone)]
struct Ship2 {
    location: Location,
    waypoint: Vector,
}

impl Default for Ship2 {
    fn default() -> Self {
        Ship2 {
            location: Location::zero(),
            waypoint: Vector::rightward(10) + Up,
        }
    }
}

impl ApplyInstruction for Ship2 {
    fn apply_instruction(self, instruction: Instruction) -> Self {
        match instruction {
            AbsoluteMove(direction, distance) => Self {
                waypoint: self.waypoint + (direction * distance),
                ..self
            },
            MoveForward(distance) => Self {
                location: self.location + (self.waypoint * distance),
                ..self
            },
            Turn(rotation) => Self {
                waypoint: self.waypoint.rotate(rotation),
                ..self
            },
        }
    }
}

pub fn part2(input: &str) -> anyhow::Result<isize> {
    execute_ship(Ship2::default(), input)
        .context("Failed to execute all instructions")
        .map(|ship| (ship.location - Location::zero()).manhattan_length())
}
