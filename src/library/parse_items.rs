//! Functions for parsing a flat list of whitespace-separated items

use std::{error::Error, iter::FromIterator, str::FromStr};

use thiserror::Error;

/// Error from `parse_items`. Contains the parse error as well as the index
/// of the failed parse.
#[derive(Debug, Clone, Error)]
#[error("error parsing item at index {index}")]
pub struct ParseItemsError<E: Error + 'static> {
    index: usize,

    #[source]
    error: E,
}

/// Advent of Code commonly gives input as a uniform list of newline or
/// whitespace separated items. This parses such a list by splitting on
/// whitespace and then parsing each individual element. This function is
/// generic over any parsable input item T, and any collection C which is
/// FromIterator<T>.
pub fn parse_items<T, C>(input: &str) -> Result<C, ParseItemsError<T::Err>>
where
    T: FromStr,
    C: FromIterator<T>,
    T::Err: Error,
{
    parse_items_iter(input.split_whitespace())
}

/// Even more generic version of `parse_items`; takes an iterator of &str for
/// parsing.
pub fn parse_items_iter<'a, I, T, C>(input: I) -> Result<C, ParseItemsError<T::Err>>
where
    I: IntoIterator<Item = &'a str>,
    T: FromStr,
    C: FromIterator<T>,
    T::Err: Error,
{
    input
        .into_iter()
        .enumerate()
        .map(|(index, value)| {
            value
                .parse()
                .map_err(|error| ParseItemsError { index, error })
        })
        .collect()
}
