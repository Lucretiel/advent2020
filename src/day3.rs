use std::iter;

use anyhow::Context;
use gridly::prelude::*;
use gridly_grids::VecGrid;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Cell {
    Empty,
    Tree,
}

fn read_grid(input: &str) -> anyhow::Result<VecGrid<Cell>> {
    let lines = input.lines();
    let rows = lines.map(|line| {
        line.chars().map(|c| match c {
            '.' => Cell::Empty,
            '#' => Cell::Tree,
            c => panic!("Invalid cell: {}", c),
        })
    });
    VecGrid::new_from_rows(rows).context("error constructing grid from input")
}

fn count_trees(map: &impl Grid<Item = Cell>, motion: Vector) -> usize {
    let mut location = map.root();

    iter::repeat_with(move || {
        location += motion;
        location
    })
    // Handle wrapping around the column
    .map(|loc| Location {
        row: loc.row,
        column: Column(loc.column.0 % map.num_columns().0),
    })
    // Continue until we hit the bottom
    .take_while(|loc| map.row_in_bounds(loc.row))
    // Get the cell at this location. Use filter map for bounds checking
    .filter_map(|loc| map.get(loc).ok())
    // Check if this cell is a tree
    .filter(|&&cell| cell == Cell::Tree)
    // Count the trees
    .count()
}

pub fn part1(input: &str) -> anyhow::Result<usize> {
    read_grid(input).map(|map| count_trees(&map, Down + (Right * 3)))
}

pub fn part2(input: &str) -> anyhow::Result<usize> {
    let map = read_grid(input)?;

    let product = [
        Down + Right,
        Down + (Right * 3),
        Down + (Right * 5),
        Down + (Right * 7),
        (Down * 2) + Right,
    ]
    .iter()
    .map(|&motion| count_trees(&map, motion))
    .product();

    Ok(product)
}
