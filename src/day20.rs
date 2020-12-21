use std::{
    collections::{HashMap, HashSet, VecDeque},
    hash::{Hash, Hasher},
    iter::FromIterator,
};

use anyhow::{bail, Context};
use cascade::cascade;
use gridly::{prelude::*, range::CrossRange};
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

static ALL_ORIENTATIONS: [Orientation; 8] = [
    Orientation {
        mirror_top_to_bottom: true,
        mirror_left_to_right: true,
        transposed: true,
    },
    Orientation {
        mirror_top_to_bottom: true,
        mirror_left_to_right: true,
        transposed: false,
    },
    Orientation {
        mirror_top_to_bottom: true,
        mirror_left_to_right: false,
        transposed: true,
    },
    Orientation {
        mirror_top_to_bottom: true,
        mirror_left_to_right: false,
        transposed: false,
    },
    Orientation {
        mirror_top_to_bottom: false,
        mirror_left_to_right: true,
        transposed: true,
    },
    Orientation {
        mirror_top_to_bottom: false,
        mirror_left_to_right: true,
        transposed: false,
    },
    Orientation {
        mirror_top_to_bottom: false,
        mirror_left_to_right: false,
        transposed: true,
    },
    Orientation {
        mirror_top_to_bottom: false,
        mirror_left_to_right: false,
        transposed: false,
    },
];

#[derive(Debug, Clone)]
struct OrientedGrid<G> {
    orientation: Orientation,
    grid: G,
}

impl<G: GridBounds> OrientedGrid<G> {
    fn convert_location(&self, location: Location) -> Location {
        let mut location = location - Location::zero();

        if self.orientation.transposed {
            location = location.transpose();
        }

        let root = self.root() - Location::zero();
        let dims = self.dimensions();

        if self.orientation.mirror_top_to_bottom {
            location.rows = (root.rows * 2 + dims.rows - 1) - location.rows;
        }

        if self.orientation.mirror_left_to_right {
            location.columns = (root.columns * 2 + dims.columns - 1) - location.columns;
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

impl<G: GridSetter> GridSetter for OrientedGrid<G> {
    unsafe fn replace_unchecked(&mut self, location: Location, value: Self::Item) -> Self::Item {
        self.grid
            .replace_unchecked(self.convert_location(location), value)
    }

    unsafe fn set_unchecked(&mut self, location: Location, value: Self::Item) {
        self.grid
            .set_unchecked(self.convert_location(location), value)
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

impl Tile {
    fn generate_edges<'a>(&'a self) -> impl Iterator<Item = Edge> + 'a {
        ALL_ORIENTATIONS
            .iter()
            .map(move |&orientation| OrientedGrid {
                grid: &self.grid,
                orientation,
            })
            .map(|grid| get_edge(&grid, Up))
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

    // All tiles, keyed by every known edge.
    let mut edge_db: HashMap<Edge, HashSet<&Tile>> = HashMap::new();

    for tile in tiles.iter() {
        for edge in tile.generate_edges() {
            edge_db.entry(edge).or_default().insert(tile);
        }
    }

    // All the tiles that haven't been places yet
    let mut unplaced: HashSet<&Tile> = tiles.iter().skip(1).collect();

    // The final, rendered image
    let mut final_image: SparseGrid<bool> =
        SparseGrid::new_rooted(Row(-100) + Column(-100), Rows(200) + Columns(200));

    // This list of tiles which have been stamped, whose neighbors need to be
    // explored
    let mut queue: VecDeque<(&Tile, Orientation, Vector)> = VecDeque::new();

    // The first tile is "canonical" in terms of orientation
    let first_tile = tiles.first().unwrap();
    stamp_tile(&mut final_image, Vector::zero(), &first_tile.grid);
    queue.push_back((first_tile, Orientation::default(), Vector::zero()));

    while let Some((tile, orientation, offset)) = queue.pop_front() {
        let grid = OrientedGrid {
            grid: &tile.grid,
            orientation,
        };

        for &direction in &EACH_DIRECTION {
            // Get the edge on this face
            let edge = get_edge(&grid, direction);

            // Find the unplaced grid with a matching edge. If there is no
            // neighbor, we assume we're at an edge, or it's already been
            // placed, so we skip to the next iteration.
            let neighbor = match edge_db
                .get(&edge)
                .unwrap()
                .iter()
                .find(|&&candidate| unplaced.contains(candidate))
            {
                None => continue,
                Some(&t) => t,
            };

            // The edge of the neighbor we're interested in
            let neighbor_edge = direction.reverse();

            // Find the orientation of the neighbor that makes it match
            let neighbor_orientation = ALL_ORIENTATIONS
                .iter()
                .copied()
                .find(|&orientation| {
                    let oriented = OrientedGrid {
                        grid: &neighbor.grid,
                        orientation,
                    };

                    get_edge(&oriented, neighbor_edge) == edge
                })
                .expect("Grid had no matching edge");

            // Get the offset of the tile in the final image. We're hardcoding
            // the knowledge that all tiles are 8x8 after removing edges.
            let neighbor_offset = offset + (direction * 8);

            // Stamp the tile
            stamp_tile(
                &mut final_image,
                neighbor_offset,
                &OrientedGrid {
                    grid: &neighbor.grid,
                    orientation: neighbor_orientation,
                },
            );

            // This tile is now placed. Remove it from unplaced and add it to
            // the queue.
            unplaced.remove(neighbor);
            queue.push_back((neighbor, neighbor_orientation, neighbor_offset));
        }
    }

    // We now have a complete image. Scan it for sea serpents.
    // The problem didn't state this outright, but we're assuming that exactly
    // 1 orientation of the final image contains any sea serpents. First find
    // that orientation.
    let correct_orientation = ALL_ORIENTATIONS
        .iter()
        .copied()
        .find(|&orientation| {
            let grid = OrientedGrid {
                grid: &final_image,
                orientation,
            };

            let row_range = RowRange::span(grid.root_row(), grid.num_rows());
            let col_range = ColumnRange::span(grid.root_column(), grid.num_columns());
            let loc_range = CrossRange::new(row_range, col_range);

            let mut windows =
                loc_range.map(|root| Window::new(&grid, root, SeaSerpent.dimensions()));

            windows.any(|window| SeaSerpent.contains_serpent(&window))
        })
        .context("No serpents found in any orientation")?;

    // We now have the correct orientation. Scan it for sea serpents. For each
    // found serpent, set all the pixels to false. We assume no overlapping
    // serpents.
    let mut grid = OrientedGrid {
        grid: &mut final_image,
        orientation: correct_orientation,
    };

    let row_range = RowRange::span(grid.root_row(), grid.num_rows());
    let col_range = ColumnRange::span(grid.root_column(), grid.num_columns());
    let loc_range = CrossRange::new(row_range, col_range);

    for root in loc_range {
        let window = Window::new(&mut grid, root, SeaSerpent.dimensions());
        let mut window = ZeroRoot::new(window);

        if SeaSerpent.contains_serpent(&window) {
            for row in SeaSerpent.rows().iter() {
                for (location, &body_part) in row.iter_with_locations() {
                    if body_part {
                        window.set(location, false).unwrap();
                    }
                }
            }
        }
    }

    // We've cleared all the serpents. Count the remaining pixels.
    let count = final_image
        .rows()
        .iter()
        .flat_map(|row| row.iter())
        .filter(|cell| **cell)
        .count();

    Ok(count)
}

struct SeaSerpent;

impl SeaSerpent {
    fn contains_serpent(&self, grid: &impl Grid<Item = bool>) -> bool {
        let grid = ZeroRoot::new(grid);

        for row in self.rows().iter() {
            for (location, &cell) in row.iter_with_locations() {
                if cell {
                    match grid.get(location) {
                        Err(..) => return false,
                        Ok(&false) => return false,
                        _ => {}
                    }
                }
            }
        }

        true
    }
}

impl GridBounds for SeaSerpent {
    fn dimensions(&self) -> Vector {
        Rows(3) + Columns(20)
    }

    fn root(&self) -> Location {
        Location::zero()
    }
}

impl Grid for SeaSerpent {
    type Item = bool;

    unsafe fn get_unchecked(&self, location: Location) -> &Self::Item {
        const ROWS: [&[u8; 20]; 3] = [
            b"                  # ",
            b"#    ##    ##    ###",
            b" #  #  #  #  #  #   ",
        ];

        match ROWS[location.row.0 as usize][location.column.0 as usize] == b'#' {
            true => &true,
            false => &false,
        }
    }
}

/// Apply the tile to the final image. The root of the tile (ignoring its edges)
/// will be at the offset of the final image
fn stamp_tile(
    final_image: &mut impl GridSetter<Item = bool>,
    offset: Vector,
    tile: &impl Grid<Item = bool>,
) {
    // Erase the edges
    let grid = Window::new(
        tile,
        tile.root() + Rows(1) + Columns(1),
        tile.dimensions() - Rows(2) - Columns(2),
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
