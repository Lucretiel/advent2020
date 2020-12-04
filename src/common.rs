//! Common functionality that different puzzles may need

use std::{error::Error, iter::FromIterator, str::FromStr};

use anyhow::Context;
use nom::{combinator::map_res, error::FromExternalError, Parser};
/// Even more generic version of `parse_items`; takes an iterator of &str for
/// parsing.
pub fn parse_items_iter<'a, I, T, C>(input: I) -> anyhow::Result<C>
where
    I: IntoIterator<Item = &'a str>,
    T: FromStr,
    T::Err: Send + Sync + Error + 'static,
    C: FromIterator<T>,
{
    input
        .into_iter()
        .enumerate()
        .map(|(i, value)| {
            value
                .parse()
                .with_context(|| format!("Failed to parse item index {}", i))
        })
        .collect()
}

/// Advent of Code commonly gives input as a uniform list of newline or
/// whitespace separated items. This parses such a list by splitting on
/// whitespace and then parsing each individual element. This function is
/// generic over any parsable input item T, and any collection C which is
/// FromIterator<T>.
pub fn parse_items<T, C>(input: &str) -> anyhow::Result<C>
where
    T: FromStr,
    T::Err: Send + Sync + Error + 'static,
    C: FromIterator<T>,
{
    parse_items_iter(input.split_whitespace())
}

/// Helper trait for converting from `bool` to `Option`.
pub trait BoolExt: Sized {
    /// If the bool is true, return the result of `func`, wrapped in `Some`;
    /// otherwise return `None`.
    fn then<T, F: FnOnce() -> T>(self, func: F) -> Option<T>;

    fn then_some<T>(self, value: T) -> Option<T> {
        self.then(move || value)
    }
}

impl BoolExt for bool {
    fn then<T, F: FnOnce() -> T>(self, func: F) -> Option<T> {
        if self {
            Some(func())
        } else {
            None
        }
    }
}

/// A nom parser that parses any FromStr type. It uses a recognizer to parse
/// the prefix string that should be parsed via FromStr
pub fn parse_from_str<'a, F, T, E>(recognizer: F) -> impl Parser<&'a str, T, E>
where
    F: Parser<&'a str, &'a str, E> + Sized,
    T: FromStr,
    E: FromExternalError<&'a str, T::Err>,
{
    map_res(recognizer, |value| value.parse())
}
