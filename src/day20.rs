use std::{
    collections::{HashMap, HashSet, VecDeque},
    hash::{Hash, Hasher},
    iter::{self, FromIterator},
};

use anyhow::{bail, Context};
use cascade::cascade;
use gridly::prelude::*;
use gridly_adapters::{Translate, Window, ZeroRoot};
use gridly_grids::{SparseGrid, VecGrid};
use nom::{
    bytes::complete::take_until,
    character::complete::{digit1, multispace0, space1},
    IResult, Parser,
};
use nom_supreme::{
    error::ErrorTree, final_parser, multi::parse_separated_terminated, parse_from_str,
    parser_ext::ParserExt, tag::complete::tag,
};

use library::BoolExt;

use crate::library;

#[derive(Debug, Clone, Default, Copy, PartialEq, Eq)]
struct Orientation {
    mirror_top_to_bottom: bool,
    mirror_left_to_right: bool,
    transposed: bool,
}

#[derive(Debug, Clone)]
struct OrientedGrid<G> {
    orientation: Orientation,
    grid: G,
}

impl<G: GridBounds> OrientedGrid<G> {
    fn convert_location(&self, mut location: Location) -> Location {
        let location = location - Location::zero();

        if self.orientation.transposed {
            location = location.transpose();
        }

        let root = self.root() - Location::zero();
        let dims = self.dimensions();

        if self.orientation.mirror_top_to_bottom {
            location.rows = (root.rows * 2 + dims.rows - 1) - location.rows;
        }

        if self.orientation.mirror_left_to_right {
            location.columns = (root.columns * 2 + root.columns - 1) - location.columns;
        }

        Location::zero() + location
    }
}

impl<G: GridBounds> GridBounds for OrientedGrid<G> {
    fn dimensions(&self) -> Vector {
        if self.orientation.transposed {
            self.grid.dimensions().transpose()
        } else {
            self.grid.dimensions()
        }
    }

    fn root(&self) -> Location {
        if self.orientation.transposed {
            self.grid.root().transpose()
        } else {
            self.grid.root()
        }
    }
}

impl<G: Grid> Grid for OrientedGrid<G> {
    type Item = G::Item;

    unsafe fn get_unchecked(&self, location: Location) -> &Self::Item {
        self.grid.get_unchecked(self.convert_location(location))
    }
}

fn get_edge(grid: &impl Grid<Item = bool>, side: Direction) -> Edge {
    match side {
        Up => grid.rows().iter().next().unwrap().iter().copied().collect(),
        Down => grid
            .rows()
            .iter()
            .next_back()
            .unwrap()
            .iter()
            .copied()
            .collect(),
        Left => grid
            .columns()
            .iter()
            .next()
            .unwrap()
            .iter()
            .copied()
            .collect(),
        Right => grid
            .columns()
            .iter()
            .next_back()
            .unwrap()
            .iter()
            .copied()
            .collect(),
    }
}

#[derive(Debug, Clone)]
struct Tile {
    id: i64,
    grid: VecGrid<bool>,
}

fn build_edge_pair(
    single_view: impl Iterator<Item = bool> + Clone + DoubleEndedIterator,
) -> impl Iterator<Item = Edge> {
    iter::once(single_view.clone().collect()).chain(iter::once(single_view.rev().collect()))
}

impl Tile {
    fn generate_edges<'a>(&'a self) -> impl Iterator<Item = Edge> + 'a {
        let mut rows = self.grid.rows().iter();
        let first_row = rows.next().unwrap();
        let last_row = rows.next_back().unwrap();

        let mut columns = self.grid.columns().iter();
        let first_col = columns.next().unwrap();
        let last_col = columns.next_back().unwrap();

        build_edge_pair(first_row.iter().copied())
            .chain(build_edge_pair(last_row.iter().copied()))
            .chain(build_edge_pair(first_col.iter().copied()))
            .chain(build_edge_pair(last_col.iter().copied()))
    }
}

impl PartialEq for Tile {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl Eq for Tile {}

impl Hash for Tile {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.id.hash(state)
    }
}

fn parse_tile(input: &str) -> IResult<&str, Tile, ErrorTree<&str>> {
    parse_from_str(digit1)
        .preceded_by(tag("Tile").terminated(space1))
        .terminated(tag(":\n"))
        .context("tile ID")
        .and(take_until("\n\n").context("tile body"))
        .map(|(id, grid_rows)| {
            let lines = grid_rows.lines();
            Tile {
                id,
                grid: VecGrid::new_from_rows(lines.map(|line| {
                    line.chars().map(|c| match c {
                        '#' => true,
                        '.' => false,
                        c => panic!("invalid grid character {:?}", c),
                    })
                }))
                .expect("Error creating grid"),
            }
        })
        .context("tile")
        .parse(input)
}

fn parse_tile_list(input: &str) -> Result<Vec<Tile>, ErrorTree<final_parser::Location>> {
    final_parser::final_parser(parse_separated_terminated(
        parse_tile,
        tag("\n\n"),
        multispace0.all_consuming(),
        Vec::new,
        |vec, tile| cascade! {vec; ..push(tile);},
    ))(input)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct Edge {
    pixels: [bool; 10],
}

impl FromIterator<bool> for Edge {
    fn from_iter<T: IntoIterator<Item = bool>>(iter: T) -> Self {
        let mut pixels = [false; 10];

        iter.into_iter()
            .zip(&mut pixels)
            .for_each(|(px, slot)| *slot = px);

        Self { pixels }
    }
}

pub fn part1(input: &str) -> anyhow::Result<i64> {
    let tiles = parse_tile_list(input).context("Failed to parse tiles")?;

    let mut neighbor_sets: HashMap<&Tile, HashSet<&Tile>> = tiles
        .iter()
        .map(|tile| (tile, HashSet::with_capacity(4)))
        .collect();

    let mut tile_db: HashMap<Edge, HashSet<&Tile>> = HashMap::new();

    for &tile in neighbor_sets.keys() {
        for edge in tile.generate_edges() {
            tile_db.entry(edge).or_default().insert(tile);
        }
    }

    for (_edge, pair) in tile_db {
        let mut pair = pair.into_iter();
        let tile1 = pair.next().unwrap();
        if let Some(tile2) = pair.next() {
            neighbor_sets.get_mut(tile1).unwrap().insert(tile2);
            neighbor_sets.get_mut(tile2).unwrap().insert(tile1);

            if pair.next().is_some() {
                bail!("Invariant violated")
            }
        }
    }

    let corner_product = neighbor_sets
        .iter()
        .filter_map(|(tile, neighbors)| (neighbors.len() == 2).then_some(tile.id))
        .product();

    Ok(corner_product)
}

pub fn part2(input: &str) -> anyhow::Result<usize> {
    let tiles = parse_tile_list(input).context("Failed to parse tiles")?;

    let mut tile_db: HashMap<Edge, HashSet<&Tile>> = HashMap::new();

    for tile in &tiles {
        for edge in tile.generate_edges() {
            tile_db.entry(edge).or_default().insert(tile);
        }
    }

    let mut final_image: SparseGrid<bool> =
        SparseGrid::new_rooted(Row(-100) + Column(-100), Rows(200) + Columns(200));

    let &first_tile = tile_db.values().next().unwrap().iter().next().unwrap();

    stamp_tile(&mut final_image, Vector::zero(), first_tile);

    let mut tile_orientation_db: HashMap<&Tile, (Vector, Orientation)> =
        HashMap::with_capacity(tiles.len());
    tile_orientation_db.insert(first_tile, (Vector::zero(), Orientation::default()));

    let mut tile_explore_queue: VecDeque<&Tile> = iter::once(first_tile).collect();

    while let Some((tile, &(offset, orientation))) =
        tile_explore_queue.pop_front().and_then(|tile| {
            tile_orientation_db
                .get(tile)
                .map(|orientation| (tile, orientation))
        })
    {
        for &direction in &EACH_DIRECTION {
            let edge = tile.get_oriented_edge(orientation, direction);

            let neighbor = match tile_db.get(&edge).unwrap().iter().find(|&&t| t != tile) {
                Some(&t) => t,
                None => continue,
            };

            let neighbor_direction = direction.reverse();

            let neighbor_orientation = [false, true]
                .iter()
                .flat_map(move |&b1| {
                    [false, true].iter().flat_map(move |&b2| {
                        [false, true].iter().map(move |&b3| Orientation {
                            mirror_top_to_bottom: b1,
                            mirror_left_to_right: b2,
                            transposed: b3,
                        })
                    })
                })
                .find(|&candidate_orientation| {
                    neighbor.get_oriented_edge(candidate_orientation, neighbor_direction) == edge
                })
                .unwrap();

            // TODO: Create a version of stamp_tile that respects orientation.
            // TODO: stamp the neighbor
            // TODO: add the neighbor to the explore queue and orientation_db
            // TODO: skip this neighbor if it's already in the orientation_db
        }
    }

    todo!()
}

/// Apply the tile to the final image. The root of the tile (ignoring its edges)
/// will be at the offset of the final image
fn stamp_tile(
    final_image: &mut impl GridSetter<Item = bool>,
    offset: Vector,
    orientation: Orientation,
    tile: &Tile,
) {
    let grid = &tile.grid;

    // Erase the edges
    let grid = Window::new(
        grid,
        grid.root() + Rows(1) + Columns(1),
        grid.dimensions() - Rows(2) - Columns(2),
    );
    let grid = ZeroRoot::new(grid);
    let grid = Translate::new(grid, offset);

    for row in grid.rows().iter() {
        for (location, &cell) in row.iter_with_locations() {
            final_image
                .set(location, cell)
                .expect("Final image somehow too small");
        }
    }
}
