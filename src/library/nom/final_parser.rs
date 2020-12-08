use std::fmt::{self, Display, Formatter};

use nom::{
    combinator::{all_consuming, complete},
    error::ParseError,
    Err as NomErr, InputLength, Offset, Parser,
};

use super::NomError;

/// Trait for recombining error information with the original input.
///
/// This trait is used to take the context information attached to nom errors-
/// specifically, the tail of the input indicating the location of the input-
/// and recombine it with the *original* input to produce something more useful
/// for error reporting.
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
