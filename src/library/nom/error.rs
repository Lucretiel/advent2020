use std::{
    error::Error,
    fmt::{self, Debug, Display, Formatter, Write},
};

use cascade::cascade;
use indent_write::fmt::IndentWriter;
use joinery::JoinableIterator;
use nom::error::{ContextError, ErrorKind as NomErrorKind, FromExternalError, ParseError};

use super::{ExtractContext, RecombineInput, TagError};

/// These are the different specific things that can go wrong at a particular
/// location during a nom parse. Many of these are collected into a NomError.
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
            BaseErrorKind::External(kind, ref err) => write!(f, "while parsing{:?}: {}", kind, err),
            BaseErrorKind::Kind(kind) => write!(f, "while parsing {:?}", kind),
        }
    }
}

/// The nom error to end all nom errors
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
                write!(f, "{}", stack.iter().join_with(",\n"))
            }
            NomError::Alt(siblings) => {
                writeln!(f, "one of:")?;
                let mut f = IndentWriter::new("  ", f);
                write!(f, "{}", siblings.iter().join_with(", or\n"))
            }
        }
    }
}

impl<I: Display + Debug> Error for NomError<I> {}

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
