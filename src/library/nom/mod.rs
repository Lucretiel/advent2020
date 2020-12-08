//! Helpers for doing nom stuff

mod error;
mod final_parser;
mod tag;

use std::str::FromStr;

use nom::{combinator::map_res, error::FromExternalError, Parser};

pub use self::{
    error::NomError,
    final_parser::{
        final_parser, final_str_parser, ByteOffset, ExtractContext, Location, RecombineInput,
    },
    tag::{tag, tag_case_insensitive, TagError},
};

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
