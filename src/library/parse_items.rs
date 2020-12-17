//! Functions for parsing a flat list of whitespace-separated items

use std::{error::Error, iter::FromIterator, str::FromStr};

use thiserror::Error;

/// Error from `parse_items`. Contains the parse error as well as the index
/// of the failed parse.
#[derive(Debug, Clone, Error)]
#[error("error parsing item {input:?} at index {index}")]
pub struct ParseItemsError<E: Error + 'static> {
    index: usize,
    input: String,

    #[source]
    error: E,
}

/// `parse_items`, but it specifically parses whitespace separated components
/// of the input
pub fn parse_items_ws<T, C>(input: &str) -> Result<C, ParseItemsError<T::Err>>
where
    T: FromStr,
    C: FromIterator<T>,
    T::Err: Error,
{
    parse_items(input.split_whitespace())
}

/// `parse_items`, but it specifically parses all lines of the input.
pub fn parse_items_lines<T, C>(input: &str) -> Result<C, ParseItemsError<T::Err>>
where
    T: FromStr,
    C: FromIterator<T>,
    T::Err: Error,
{
    parse_items(input.lines())
}

/// Advent of Code commonly gives input as a uniform list of separated items.
/// This parses such a list by takin an iterator of those items and then parsing
/// each individual element with FromStr. This function is generic over any
/// parsable input item T, and any collection C which is FromIterator<T>. Any
/// errors are wrapped, indicating which element failed to parse.
pub fn parse_items<'a, I, T, C>(input: I) -> Result<C, ParseItemsError<T::Err>>
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
            value.parse().map_err(|error| ParseItemsError {
                index,
                error,
                input: value.to_owned(),
            })
        })
        .collect()
}
