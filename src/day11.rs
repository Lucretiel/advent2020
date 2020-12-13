use std::{iter, mem};

use anyhow::Context;
use gridly::prelude::*;
use gridly_grids::VecGrid;
use itertools::Itertools;
use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Seat {
    Empty,
    Occupied,
}

#[derive(Debug, Clone, Error)]
#[error("Invalid seat character")]
struct InvalidSeat;

fn parse_seat(cell: char) -> Result<Option<Seat>, InvalidSeat> {
    match cell {
        '.' => Ok(None),
        'L' => Ok(Some(Seat::Empty)),
        '#' => Ok(Some(Seat::Occupied)),
        _ => Err(InvalidSeat),
    }
}

pub fn part1(input: &str) -> anyhow::Result<usize> {
    let lines = input.lines().map(|line| {
        line.chars().map(|c| {
            parse_seat(c)
                .with_context(|| format!("Error parsing seat {:?}", c))
                .unwrap()
        })
    });

    let mut grid: VecGrid<Option<Seat>> =
        VecGrid::new_from_rows(lines).context("Failed to create grid")?;

    let mut scratch = grid.clone();

    loop {
        let mut changed = false;

        for row in grid.rows().iter() {
            for (target, cell) in row.iter_with_locations() {
                if let Some(seat) = cell {
                    match seat {
                        Seat::Empty => {
                            if TOUCHING_ADJACENCIES
                                .iter()
                                .map(|&dir| target + dir)
                                .filter_map(|nearby| grid.get(nearby).ok())
                                .all(|&nearby| nearby != Some(Seat::Occupied))
                            {
                                scratch.set(target, Some(Seat::Occupied)).unwrap();
                                changed = true;
                            } else {
                                scratch.set(target, Some(Seat::Empty)).unwrap();
                            }
                        }
                        Seat::Occupied => {
                            if TOUCHING_ADJACENCIES
                                .iter()
                                .map(|&dir| target + dir)
                                .filter_map(|nearby| grid.get(nearby).ok())
                                .filter(|&&nearby| nearby == Some(Seat::Occupied))
                                .count()
                                >= 4
                            {
                                scratch.set(target, Some(Seat::Empty)).unwrap();
                                changed = true;
                            } else {
                                scratch.set(target, Some(Seat::Occupied)).unwrap();
                            }
                        }
                    }
                }
            }
        }

        mem::swap(&mut grid, &mut scratch);

        if !changed {
            break;
        }
    }

    let occupied = grid
        .rows()
        .iter()
        .flat_map(|row| row.iter())
        .filter(|&&cell| cell == Some(Seat::Occupied))
        .count();

    Ok(occupied)
}

fn scan_visible(grid: &impl Grid<Item = Option<Seat>>, target: Location) -> usize {
    TOUCHING_ADJACENCIES
        .iter()
        .copied()
        // Find all directions that can see a seat
        .filter(|&direction| {
            // Create an iterator of locations moving in the given direction
            iter::successors(Some(target + direction), |&loc| Some(loc + direction))
                // Get the cell at each location in this direction
                .map(|location| grid.get(location).ok())
                // While we're in the grid bounds
                .while_some()
                // Find any occupied seat in the chain
                .find_map(|&cell| cell)
                .unwrap_or(Seat::Empty)
                == Seat::Occupied
        })
        .count()
}

pub fn part2(input: &str) -> anyhow::Result<usize> {
    let lines = input.lines().map(|line| {
        line.chars().map(|c| {
            parse_seat(c)
                .with_context(|| format!("Error parsing seat {:?}", c))
                .unwrap()
        })
    });

    let mut grid: VecGrid<Option<Seat>> =
        VecGrid::new_from_rows(lines).context("Failed to create grid")?;

    let mut scratch = grid.clone();

    loop {
        let mut changed = false;

        grid.rows()
            .iter()
            .flat_map(|row| row.iter_with_locations())
            .filter_map(|(loc, cell)| cell.as_ref().map(|&seat| (loc, seat)))
            .for_each(|(target, seat): (Location, Seat)| {
                let visible_occupied = scan_visible(&grid, target);

                scratch
                    .set(
                        target,
                        Some(match seat {
                            Seat::Empty => match visible_occupied == 0 {
                                true => {
                                    changed = true;
                                    Seat::Occupied
                                }
                                false => Seat::Empty,
                            },
                            Seat::Occupied => match visible_occupied >= 5 {
                                true => {
                                    changed = true;
                                    Seat::Empty
                                }
                                false => Seat::Occupied,
                            },
                        }),
                    )
                    .unwrap();
            });

        mem::swap(&mut grid, &mut scratch);

        if !changed {
            break;
        }
    }

    let seated = grid
        .rows()
        .iter()
        .flat_map(|row| row.iter())
        .filter(|&&cell| cell == Some(Seat::Occupied))
        .count();

    Ok(seated)
}
