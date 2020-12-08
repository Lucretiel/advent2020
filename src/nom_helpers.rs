use std::{
    error::Error,
    fmt::{self, Display, Formatter, Write},
};

use cascade::cascade;
use indent_write::fmt::IndentWriter;
use joinery::prelude::*;
use nom::{
    combinator::{all_consuming, complete},
    error::{ContextError, ErrorKind as NomErrorKind, FromExternalError, ParseError},
    Compare, CompareResult, Err as NomErr, IResult, InputLength, InputTake, Offset, Parser,
};

/// Similar to [`ParseError`] and [`ContextError`], this trait allows a parser
/// to create an error representing an unmatched tag. This allows error
/// messages to produce more useful context about what went wrong.
pub trait TagError<T, I>: Sized {
    /// Create an error from an expected tag at a location.
    fn from_tag(input: I, tag: T) -> Self;

    /// As above, but for a case insensitive tag. By default this just
    /// calls from_tag
    fn from_case_insensitive_tag(input: I, tag: T) -> Self {
        Self::from_tag(input, tag)
    }
}

/// Trait for recombining error information with the original input.
///
/// This trait is used to take the context information attached to nom errors-
/// specifically, the tail of the input indicating the location of the input-
/// and recombine it with the *original* input to produce something more useful
/// for error reporting.
///
/// It has two typical use patterns:
///
/// For instance, `&str` implements `RecombineInput<Location>`, which
/// allows extracting the line and column number where an error occurred.
pub trait RecombineInput<T>: Sized {
    fn recombine_input(self, original_input: Self) -> T;
}

impl<T> RecombineInput<T> for T {
    fn recombine_input(self, _: Self) -> T {
        self
    }
}

/// A byte offset into the input where an error may have occurred
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ByteOffset(pub usize);

impl<I: Offset> RecombineInput<ByteOffset> for I {
    fn recombine_input(self, original_input: Self) -> ByteOffset {
        ByteOffset(original_input.offset(&self))
    }
}

/// A location in a string where an error may have occurred. In keeping with
/// the typical practice from editors and other tools, line and columns are both
/// 1-indexed.
///
/// If the input string had *no* newlines in the input, the 0-indexed character
/// index is instead returned via the "Flat" variant.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Location {
    pub line: usize,
    pub column: usize,
}

impl Location {
    /// Given the *original* input string, as well as the context reported by
    /// nom, compute the location in the original string where the error
    /// occurred.
    ///
    /// This function will report garbage (and may panic) if the context is not
    /// associated with the input
    pub fn from_context<'a>(original_input: &'a str, context: &'a str) -> Self {
        let offset = original_input.len() - context.len();
        let input_bytes = original_input.as_bytes();

        let prefix = &input_bytes[..offset];
        let line_number = prefix.iter().filter(|&&b| b == b'\n').count() + 1;

        let last_line_start = prefix
            .iter()
            .rposition(|&b| b == b'\n')
            .map(|i| i + 1)
            .unwrap_or(0);
        let last_line = &prefix[last_line_start..];
        let column_number = last_line.len() + 1;

        Location {
            line: line_number,
            column: column_number,
        }
    }
}

impl Display for Location {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        if f.alternate() {
            write!(f, "line {}, column {}", self.line, self.column)
        } else {
            write!(f, "{}:{}", self.line, self.column)
        }
    }
}

impl RecombineInput<Location> for &str {
    fn recombine_input(self, original_input: Self) -> Location {
        Location::from_context(original_input, self)
    }
}

/// Trait for recombining error information with the original input.
///
/// This trait is used to take the context information attached to nom errors-
/// specifically, the tail of the input indicating the location of the input-
/// and recombine it with the *original* input to produce an error with
/// something more useful for error reporting.
///
/// Typically, it looks like  `ExtractContext<I, E<T>> for E<I>`. This
/// indicates that some error type `E`, which is generic over the input type,
/// can be converted into another variant of that error, using `T` instead of
/// `I` to hold the result context. Often this context conversion can happen
/// with RecombineInput
pub trait ExtractContext<I, T> {
    /// Given the context attached to a nom error, and given the *original*
    /// input to the nom parser, extract more the useful context information.
    ///
    /// For example, for a string, 1 possible context extraction would be the
    /// Location (line and column number) in the original input where the error
    /// indicated by self occurred.
    fn extract_context(self, original_input: I) -> T;
}

#[derive(Debug)]
pub enum BaseErrorKind {
    Tag(&'static str),
    Char(char),
    Kind(NomErrorKind),
    Context(&'static str),
    External(NomErrorKind, Box<dyn Error + Send + Sync + 'static>),
}

impl Display for BaseErrorKind {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match *self {
            BaseErrorKind::Tag(tag) => write!(f, "expected {:?}", tag),
            BaseErrorKind::Char(character) => write!(f, "expected {:?}", character),
            BaseErrorKind::Context(context) => write!(f, "in section '{}'", context),
            BaseErrorKind::External(kind, ref err) => write!(f, "{:?}: {}", kind, err),
            BaseErrorKind::Kind(kind) => write!(f, "{:?}", kind),
        }
    }
}

#[derive(Debug)]
pub enum NomError<I> {
    Base {
        kind: BaseErrorKind,
        location: I,
    },

    /// A stack indicates a chain of error contexts. The stack should be read
    /// "backwards"; that is, errors *earlier* in the Vec occurred "sooner"
    /// (deeper in the call stack)
    Stack(Vec<Self>),

    /// All of the errors in this set are "siblings"
    Alt(Vec<Self>),
}

impl<I> NomError<I> {
    fn map_locations_ref<T>(self, convert_location: &mut impl FnMut(I) -> T) -> NomError<T> {
        match self {
            NomError::Base { location, kind } => NomError::Base {
                location: convert_location(location),
                kind,
            },
            NomError::Stack(stack) => NomError::Stack(
                stack
                    .into_iter()
                    .map(|err| err.map_locations_ref(convert_location))
                    .collect(),
            ),
            NomError::Alt(siblings) => NomError::Alt(
                siblings
                    .into_iter()
                    .map(|err| err.map_locations_ref(convert_location))
                    .collect(),
            ),
        }
    }

    /// Convert all of the locations in this error using some kind of mapping
    /// function. This is intended to help add additional context that may not
    /// have been available when the nom parsers were running, such as line
    /// and column numbers.
    pub fn map_locations<T>(self, mut convert_location: impl FnMut(I) -> T) -> NomError<T> {
        self.map_locations_ref(&mut convert_location)
    }
}

impl<I: Display> Display for NomError<I> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            NomError::Base { kind, location } => write!(f, "{} at {:#}", kind, location),
            NomError::Stack(stack) => {
                writeln!(f, "trace:")?;
                let mut f = IndentWriter::new("  ", f);
                let trace = stack.iter().join_with(",\n");
                write!(f, "{}", trace)
            }
            NomError::Alt(siblings) => {
                writeln!(f, "one of:")?;
                let mut f = IndentWriter::new("  ", f);
                let alts = siblings.iter().join_with(", or\n");
                write!(f, "{}", alts)
            }
        }
    }
}

impl<I> ParseError<I> for NomError<I> {
    /// Create a new error at the given position
    fn from_error_kind(location: I, kind: NomErrorKind) -> Self {
        NomError::Base {
            location,
            kind: BaseErrorKind::Kind(kind),
        }
    }

    /// Combine an existing error with a new one. This is how
    /// error context is accumulated when backtracing. "other" is the orignal
    /// error, and the inputs new error from higher in the call stack.
    fn append(input: I, kind: NomErrorKind, other: Self) -> Self {
        let stack = cascade! {
            match other {
                NomError::Stack(stack) => stack,
                err => cascade! {
                    Vec::with_capacity(2);
                    ..push(err);
                }
            };
            ..push(Self::from_error_kind(input, kind));
        };

        NomError::Stack(stack)
    }

    /// Create an error indicating an expected character at a given position
    fn from_char(location: I, character: char) -> Self {
        NomError::Base {
            location,
            kind: BaseErrorKind::Char(character),
        }
    }

    /// Combine two errors from branches of alt
    fn or(self, other: Self) -> Self {
        let siblings = match (self, other) {
            (NomError::Alt(mut siblings1), NomError::Alt(mut siblings2)) => {
                if siblings1.capacity() >= siblings2.capacity() {
                    siblings1.extend(siblings2);
                    siblings1
                } else {
                    siblings2.extend(siblings1);
                    siblings2
                }
            }
            (NomError::Alt(mut siblings), err) | (err, NomError::Alt(mut siblings)) => {
                siblings.push(err);
                siblings
            }
            (err1, err2) => vec![err1, err2],
        };

        NomError::Alt(siblings)
    }
}

impl<I> ContextError<I> for NomError<I> {
    /// Similar to append: Create a new error with some added context
    fn add_context(location: I, ctx: &'static str, other: Self) -> Self {
        let stack = cascade! {
            match other {
                NomError::Stack(stack) => stack,
                err => cascade! {
                    Vec::with_capacity(2);
                    ..push(err);
                }
            };
            ..push(NomError::Base {
                location,
                kind: BaseErrorKind::Context(ctx),
            });
        };

        NomError::Stack(stack)
    }
}

impl<I, E: Error + Send + Sync + 'static> FromExternalError<I, E> for NomError<I> {
    /// Create an error from a given external error, such as from FromStr
    fn from_external_error(location: I, kind: NomErrorKind, e: E) -> Self {
        NomError::Base {
            location,
            kind: BaseErrorKind::External(kind, Box::new(e)),
        }
    }
}

impl<I> TagError<&'static str, I> for NomError<I> {
    fn from_tag(location: I, tag: &'static str) -> Self {
        NomError::Base {
            location,
            kind: BaseErrorKind::Tag(tag),
        }
    }
}

impl<I, T> ExtractContext<I, NomError<T>> for NomError<I>
where
    I: Clone + RecombineInput<T>,
{
    fn extract_context(self, original_input: I) -> NomError<T> {
        self.map_locations(move |location| location.recombine_input(original_input.clone()))
    }
}

pub fn tag<T, I, E>(tag: T) -> impl Clone + Fn(I) -> IResult<I, I, E>
where
    T: InputLength + Clone,
    I: InputTake + Compare<T>,
    E: TagError<T, I>,
{
    move |input: I| match input.compare(tag.clone()) {
        CompareResult::Ok => Ok(input.take_split(tag.input_len())),
        _ => Err(NomErr::Error(E::from_tag(input, tag.clone()))),
    }
}

#[allow(dead_code)]
pub fn tag_case_insensitive<T, I, E>(tag: T) -> impl Clone + Fn(I) -> IResult<I, I, E>
where
    T: InputLength + Clone,
    I: InputTake + Compare<T>,
    E: TagError<T, I>,
{
    move |input: I| match input.compare_no_case(tag.clone()) {
        CompareResult::Ok => Ok(input.take_split(tag.input_len())),
        _ => Err(NomErr::Error(E::from_case_insensitive_tag(
            input,
            tag.clone(),
        ))),
    }
}

/// Bootstrapping layer for a nom parser.
///
/// This function is intended to be the entry point into a nom parser; it
/// represents in some sense the "end of composability". It creates a function
/// which applies a parser to a string. The parser is configured such that it
/// must parse the *entire* input string, and any "Incomplete" responses are
/// reported as errors. Additionally, if the parser returns an error, the
/// context information in the error is recombined with the original input
/// string via `ExtractContext` to create a more useful error.
pub fn final_parser<I, O, E, E2>(parser: impl Parser<I, O, E>) -> impl FnMut(I) -> Result<O, E2>
where
    E: ParseError<I> + ExtractContext<I, E2>,
    I: InputLength + Clone,
{
    let mut parser = all_consuming(complete(parser));

    move |input| match parser.parse(input.clone()) {
        Ok((_, parsed)) => Ok(parsed),
        Err(NomErr::Error(err)) | Err(NomErr::Failure(err)) => Err(err.extract_context(input)),
        Err(NomErr::Incomplete(..)) => {
            unreachable!("Complete combinator should make this impossible")
        }
    }
}

/// To make our lives easier, this function is the same as final_parser, but
/// more specific types
pub fn final_str_parser<'a, O>(
    parser: impl Parser<&'a str, O, NomError<&'a str>>,
) -> impl FnMut(&'a str) -> Result<O, NomError<Location>> {
    final_parser(parser)
}
