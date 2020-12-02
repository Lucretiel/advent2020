//! Common functionality that different puzzles may need

use std::{iter::FromIterator, str::FromStr};

/// Even more generic version of `parse_items`; takes an iterator of &str for
/// parsing.
pub fn parse_items_iter<'a, T: FromStr, C: FromIterator<T>>(
    input: impl IntoIterator<Item = &'a str>,
) -> Result<C, T::Err> {
    input.into_iter().map(str::parse).collect()
}

/// Advent of Code commonly gives input as a uniform list of newline or
/// whitespace separated items. This function is generic over any parsable
/// input item T, and any collection C which is FromIterator<T>.
pub fn parse_items<T: FromStr, C: FromIterator<T>>(input: &str) -> Result<C, T::Err> {
    parse_items_iter(input.split_whitespace())
}
