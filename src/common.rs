//! Common functionality that different puzzles may need

use std::{iter::FromIterator, str::FromStr};

/// Advent of Code commonly gives input as a uniform list of newline or
/// whitespace separated items. This function is generic over any parsable
/// input item T, and any collection C which is FromIterator<T>.
pub fn parse_items<T: FromStr, C: FromIterator<T>>(input: &str) -> Result<C, T::Err> {
    input
        .split_whitespace()
        .map(|value| value.parse())
        .collect()
}
