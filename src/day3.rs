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

    iter::from_fn(move || {
        // Advance our location
        location += motion;

        // Handle wrapping around the column
        location.column.0 %= map.num_columns().0;

        // Continue until we hit the bottom
        map.get(location).ok()
    })
    // Check if the cell is a tree
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
