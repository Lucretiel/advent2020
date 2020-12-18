use std::{collections::HashSet, iter};

use anyhow::{bail, Context};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct Location {
    w: isize,
    x: isize,
    y: isize,
    z: isize,
}

impl Location {
    fn neighbors(self, include_w: bool) -> impl Iterator<Item = Location> {
        (-1..=1)
            .flat_map(|x| {
                (-1..=1).flat_map(move |y| {
                    (-1..=1).flat_map(move |z| (-1..=1).map(move |w| Location { w, x, y, z }))
                })
            })
            .filter(move |loc| include_w || loc.w == 0)
            .filter(|loc| loc.x != 0 || loc.y != 0 || loc.z != 0 || loc.w != 0)
            .map(move |loc| Location {
                x: self.x + loc.x,
                y: self.y + loc.y,
                z: self.z + loc.z,
                w: self.w + loc.w,
            })
    }
}

#[derive(Debug, Clone, Default)]
struct ConwayCube {
    cells: HashSet<Location>,
}

impl ConwayCube {
    fn alive(&self, location: &Location) -> bool {
        self.cells.contains(location)
    }

    fn step(&self, include_w: bool) -> ConwayCube {
        let mut interesting_places = self.cells.clone();
        interesting_places.extend(self.cells.iter().flat_map(|&loc| loc.neighbors(include_w)));

        let mut cells = HashSet::with_capacity(self.cells.len());

        for loc in interesting_places {
            let active_neighbors = loc
                .neighbors(include_w)
                .filter(|neighbor| self.alive(neighbor))
                .count();

            if self.alive(&loc) {
                if let 2 | 3 = active_neighbors {
                    cells.insert(loc);
                }
            } else if active_neighbors == 3 {
                cells.insert(loc);
            }
        }

        ConwayCube { cells }
    }
}

fn parse_cube<I>(cells: I) -> anyhow::Result<ConwayCube>
where
    I: IntoIterator,
    I::Item: IntoIterator<Item = char>,
{
    let mut cube = ConwayCube::default();

    for (x, row) in (0..).zip(cells) {
        for (y, cell) in (0..).zip(row) {
            match cell {
                '#' => {
                    cube.cells.insert(Location { x, y, z: 0, w: 0 });
                }
                '.' => {}
                cell => bail!("Invalid cell {} at row {}, column {}", cell, x, y),
            }
        }
    }

    Ok(cube)
}

fn solve(input: &'static str, include_w: bool) -> anyhow::Result<usize> {
    let initial_cube = parse_cube(input.lines().map(|line| line.trim().chars()))
        .context("Failed to parse cube")?;

    let mut steps = iter::successors(Some(initial_cube), |cube| Some(cube.step(include_w)));

    let final_cube = steps.nth(6).unwrap();

    Ok(final_cube.cells.len())
}

pub fn part1(input: &'static str) -> anyhow::Result<usize> {
    solve(input, false)
}

pub fn part2(input: &'static str) -> anyhow::Result<usize> {
    solve(input, true)
}
