//! Helpers for doing nom stuff

mod error;
mod final_parser;
// mod parser_ext;
mod tag;

use std::str::FromStr;

use nom::{
    combinator::map_res,
    error::{ErrorKind, FromExternalError, ParseError},
    Err as NomErr, Parser,
};

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

/// The perfected folding parser. Parses a series of 1 more more things,
/// separated by some separator, terminated by some terminator. None of these
/// things are optional (though you can of course pass an empty or no-op parser
/// to any of them). If you want 0 or more things to be parsed, wrap this in
/// opt.
///
/// When parsing begins, an accumulator value is created with init(). Then,
/// each parsed item will be folded into the accumulator via the fold function.
///
/// After parsing each item, `parse_separated_terminated` will attempt to
/// parse a terminator. If it succeeds, it will return the accumulator;
/// otherwise, it will attempt to parse a separator. If it fails to parse
/// either a separator or a terminator, it will return an error; otherwise,
/// it will continue on to parse and fold the next item.
///
/// This parser exists to provide meaningful parse errors. By requiring a
/// terminator, we can ensure that it doesn't suffer from the normal folding
/// parser problem of unconditionally returning success because all parse
/// failures simply end the fold without knowing if there's a larger problem.
///
/// Unlike nom multi parsers, we allow 0-length matches. Take care not to
/// end up in an infinite loop.
pub fn parse_separated_terminated<I, PO, SO, TO, E, P, S, T, R, F>(
    mut parser: P,
    mut separator: S,
    mut terminator: T,

    mut init: impl FnMut() -> R,
    mut fold: F,
) -> impl Parser<I, R, E>
where
    P: Parser<I, PO, E>,
    S: Parser<I, SO, E>,
    T: Parser<I, TO, E>,
    F: FnMut(R, PO) -> R,
    I: Clone,
    R: Clone,
    E: ParseError<I>,
{
    move |mut input: I| {
        let mut accum = init();

        loop {
            // Try to find a value. To fail to do so at this point is an
            // error, since we either just started or successfully parsed a
            // terminator.
            let (tail, value) = match parser.parse(input.clone()) {
                Ok((tail, value)) => (tail, value),
                Err(err) => {
                    break Err(err.map(|err| E::append(input.clone(), ErrorKind::Many1, err)))
                }
            };
            input = tail;

            // Try to find a terminator; if we found it, we're done.
            let terminator_err = match terminator.parse(input.clone()) {
                // We found a terminator, so we're done
                Ok((tail, _)) => break Ok((tail, accum)),

                // No terminator. Keep track of the error in case we also fail
                // to find a separator.
                Err(NomErr::Error(err)) => err,

                // Other kinds of errors should be returned immediately.
                Err(NomErr::Failure(err)) => {
                    break Err(NomErr::Failure(E::append(
                        input.clone(),
                        ErrorKind::Many1,
                        err,
                    )))
                }
                Err(NomErr::Incomplete(n)) => break Err(NomErr::Incomplete(n)),
            };

            // No terminator, so instead try to find a separator
            let tail = match separator.parse(input.clone()) {
                // Found a separator; advance the input.
                Ok((tail, _)) => tail,

                // We found neither the terminator nor the separator we were
                // expecting. This is therefore an alternation error
                Err(NomErr::Error(err)) => {
                    break Err(NomErr::Failure(E::append(
                        input.clone(),
                        ErrorKind::Many1,
                        E::or(err, terminator_err),
                    )))
                }

                // Other kinds of errors should be returned immediately
                Err(NomErr::Failure(err)) => {
                    break Err(NomErr::Failure(E::append(
                        input.clone(),
                        ErrorKind::Many1,
                        err,
                    )))
                }
                Err(NomErr::Incomplete(n)) => break Err(NomErr::Incomplete(n)),
            };

            input = tail;
            accum = fold(accum, value);
        }
    }
}
