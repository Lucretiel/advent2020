use std::{
    collections::{HashMap, HashSet},
    mem,
};

use anyhow::Context;
use bitvec::__count_elts;
use gridly::prelude::*;
use nom::{
    branch::alt,
    character::complete::{multispace0, multispace1},
    combinator::eof,
    IResult, Parser,
};
use nom_supreme::{
    error::ErrorTree,
    final_parser::{self, final_parser},
    multi::parse_separated_terminated,
    parser_ext::ParserExt,
    tag::complete::tag,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum HexDirection {
    East,
    Southeast,
    Southwest,
    West,
    Northwest,
    Northeast,
}

use HexDirection::*;

static ALL_HEX_DIRECTIONS: [HexDirection; 6] =
    [East, Southeast, Southwest, West, Northwest, Northeast];

impl VectorLike for HexDirection {
    #[inline]
    fn rows(&self) -> Rows {
        match *self {
            Southeast | Southwest => Rows(1),
            East | West => Rows(0),
            Northeast | Northwest => Rows(-1),
        }
    }

    fn columns(&self) -> Columns {
        match *self {
            Northeast | East => Columns(1),
            Southeast | Northwest => Columns(0),
            West | Southwest => Columns(-1),
        }
    }

    fn as_vector(&self) -> Vector {
        self.rows() + self.columns()
    }
}

#[inline]
fn parse_nothing(input: &str) -> IResult<&str, (), ErrorTree<&str>> {
    Ok((input, ()))
}

fn parse_hex_direction(input: &str) -> IResult<&str, HexDirection, ErrorTree<&str>> {
    alt((
        tag("se").value(Southeast),
        tag("sw").value(Southwest),
        tag("ne").value(Northeast),
        tag("nw").value(Northwest),
        tag("e").value(East),
        tag("w").value(West),
    ))
    .context("direction")
    .parse(input)
}

fn parse_direction_list(input: &str) -> IResult<&str, Location, ErrorTree<&str>> {
    parse_separated_terminated(
        parse_hex_direction,
        parse_nothing,
        multispace1,
        Location::zero,
        |location, direction| location + direction,
    )
    .context("direction list")
    .parse(input)
}

fn parse_tile_set(input: &str) -> Result<HashSet<Location>, ErrorTree<final_parser::Location>> {
    final_parser(
        parse_separated_terminated(
            parse_direction_list,
            multispace0,
            multispace0.all_consuming(),
            HashSet::new,
            |mut set, location| {
                if !set.insert(location) {
                    set.remove(&location);
                }
                set
            },
        )
        .context("all instructions"),
    )(input)
}

pub fn part1(input: &str) -> anyhow::Result<usize> {
    let tiles = parse_tile_set(input).context("Failed to parse tile set")?;
    let num_black = tiles.len();
    Ok(num_black)
}

pub fn part2(input: &str) -> anyhow::Result<usize> {
    let mut tiles = parse_tile_set(input).context("Failed to parse tile set")?;
    let mut next_tiles = HashSet::with_capacity(tiles.len());
    let mut empty_neighbor_set: HashMap<Location, usize> = HashMap::with_capacity(tiles.len());

    for _ in 0..100 {
        for &location in &tiles {
            let mut count = 0;

            ALL_HEX_DIRECTIONS
                .iter()
                .map(|&direction| location + direction)
                .for_each(|neighbor| match tiles.contains(&neighbor) {
                    true => count += 1,
                    false => *empty_neighbor_set.entry(neighbor).or_default() += 1,
                });

            if count > 0 && count <= 2 {
                next_tiles.insert(location);
            }
        }

        next_tiles.extend(
            empty_neighbor_set
                .iter()
                .filter(|&(_, &count)| count == 2)
                .map(|(&loc, _)| loc),
        );

        mem::swap(&mut tiles, &mut next_tiles);
        next_tiles.clear();
        empty_neighbor_set.clear();
    }

    Ok(tiles.len())
}
