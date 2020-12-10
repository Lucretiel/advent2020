//! Extensions to the nom Parser trait which add postfix versions of the
//! common combinators

use std::{marker::PhantomData, ops::RangeTo};

use nom::{
    error::{ContextError, ErrorKind as NomErrorKind, ParseError},
    Err as NomErr, InputLength, Offset, Parser, Slice,
};

pub trait ParserExt<I, O, E>: Parser<I, O, E> + Sized {
    fn all_consuming(self) -> AllConsuming<Self>
    where
        I: InputLength,
        E: ParseError<I>,
    {
        AllConsuming { parser: self }
    }

    fn complete(self) -> Complete<Self>
    where
        I: Clone,
        E: ParseError<I>,
    {
        Complete { parser: self }
    }

    fn cut(self) -> Cut<Self> {
        Cut { parser: self }
    }

    fn opt(self) -> Optional<Self>
    where
        I: Clone,
    {
        Optional { parser: self }
    }

    fn recognize(self) -> Recognize<Self, O>
    where
        I: Clone + Slice<RangeTo<usize>> + Offset,
    {
        Recognize {
            parser: self,
            phantom: PhantomData,
        }
    }

    fn value<T: Clone>(self, value: T) -> Value<T, Self, O> {
        Value {
            parser: self,
            value,
            phantom: PhantomData,
        }
    }

    fn verify<F>(self, verifier: F) -> Verify<Self, F>
    where
        F: Fn(&O) -> bool,
        I: Clone,
        E: ParseError<I>,
    {
        Verify {
            parser: self,
            verifier,
        }
    }

    fn context(self, context: &'static str) -> Context<Self>
    where
        E: ContextError<I>,
        I: Clone,
    {
        Context {
            context,
            parser: self,
        }
    }

    fn terminated<F, O2>(self, terminator: F) -> Terminated<Self, F, O2>
    where
        F: Parser<I, O2, E>,
    {
        Terminated {
            parser: self,
            terminator,
            phantom: PhantomData,
        }
    }

    fn precedes<F, O2>(self, successor: F) -> Preceded<F, Self, O>
    where
        F: Parser<I, O2, E>,
    {
        successor.preceded_by(self)
    }

    fn preceded_by<F, O2>(self, prefix: F) -> Preceded<Self, F, O2>
    where
        F: Parser<I, O2, E>,
    {
        Preceded {
            parser: self,
            prefix,
            phantom: PhantomData,
        }
    }

    fn delimited_by<L, R, O1, O2>(self, prefix: L, suffix: R) -> Delimited<L, Self, R, O1, O2>
    where
        L: Parser<I, O1, E>,
        R: Parser<I, O2, E>,
    {
        Delimited {
            prefix,
            suffix,
            parser: self,
            phantom: PhantomData,
        }
    }
}

impl<I, O, E, P> ParserExt<I, O, E> for P where P: Parser<I, O, E> {}

/// Parser which fails if the whole input isn't consumed
#[derive(Debug, Clone, Copy)]
pub struct AllConsuming<P> {
    parser: P,
}

impl<I, O, E, P> Parser<I, O, E> for AllConsuming<P>
where
    P: Parser<I, O, E>,
    E: ParseError<I>,
    I: InputLength,
{
    fn parse(&mut self, input: I) -> nom::IResult<I, O, E> {
        let (tail, value) = self.parser.parse(input)?;

        if tail.input_len() > 0 {
            Err(NomErr::Error(E::from_error_kind(tail, NomErrorKind::Eof)))
        } else {
            Ok((tail, value))
        }
    }
}

/// Parser which transforms incomplete into an error
#[derive(Debug, Clone, Copy)]
pub struct Complete<P> {
    parser: P,
}

impl<I, O, E, P> Parser<I, O, E> for Complete<P>
where
    P: Parser<I, O, E>,
    E: ParseError<I>,
    I: Clone,
{
    fn parse(&mut self, input: I) -> nom::IResult<I, O, E> {
        self.parser
            .parse(input.clone())
            .map_err(move |err| match err {
                NomErr::Incomplete(..) => {
                    NomErr::Error(E::from_error_kind(input, NomErrorKind::Complete))
                }
                err => err,
            })
    }
}

/// Parser which transforms Error into Failure, preventing other branches
#[derive(Debug, Clone, Copy)]
pub struct Cut<P> {
    parser: P,
}

impl<I, O, E, P> Parser<I, O, E> for Cut<P>
where
    P: Parser<I, O, E>,
{
    fn parse(&mut self, input: I) -> nom::IResult<I, O, E> {
        self.parser.parse(input).map_err(|err| match err {
            NomErr::Error(err) => NomErr::Failure(err),
            err => err,
        })
    }
}

/// Parser which is optional, and returns None if it fails
#[derive(Debug, Clone, Copy)]
pub struct Optional<P> {
    parser: P,
}

impl<I, O, E, P> Parser<I, Option<O>, E> for Optional<P>
where
    P: Parser<I, O, E>,
    I: Clone,
{
    fn parse(&mut self, input: I) -> nom::IResult<I, Option<O>, E> {
        match self.parser.parse(input.clone()) {
            Ok((tail, value)) => Ok((tail, Some(value))),
            Err(NomErr::Error(_)) => Ok((input, None)),
            Err(e) => Err(e),
        }
    }
}

/// Parser which discards its output and instead returns the consumed input
#[derive(Debug, Clone, Copy)]
pub struct Recognize<P, O> {
    parser: P,
    phantom: PhantomData<O>,
}

impl<I, O, E, P> Parser<I, I, E> for Recognize<P, O>
where
    P: Parser<I, O, E>,
    I: Clone + Slice<RangeTo<usize>> + Offset,
{
    fn parse(&mut self, input: I) -> nom::IResult<I, I, E> {
        let (tail, _) = self.parser.parse(input.clone())?;
        let index = input.offset(&tail);
        Ok((tail, input.slice(..index)))
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Value<T, P, O> {
    parser: P,
    value: T,
    phantom: PhantomData<O>,
}

impl<I, O, E, T, P> Parser<I, T, E> for Value<T, P, O>
where
    P: Parser<I, O, E>,
    T: Clone,
{
    fn parse(&mut self, input: I) -> nom::IResult<I, T, E> {
        let (input, _) = self.parser.parse(input)?;
        Ok((input, self.value.clone()))
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Verify<P, F> {
    parser: P,
    verifier: F,
}

impl<I, O, E, P, F> Parser<I, O, E> for Verify<P, F>
where
    P: Parser<I, O, E>,
    E: ParseError<I>,
    F: Fn(&O) -> bool,
    I: Clone,
{
    fn parse(&mut self, input: I) -> nom::IResult<I, O, E> {
        let (input, value) = self.parser.parse(input.clone())?;

        match (self.verifier)(&value) {
            true => Ok((input, value)),
            false => Err(NomErr::Error(E::from_error_kind(
                input,
                NomErrorKind::Verify,
            ))),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Context<P> {
    context: &'static str,
    parser: P,
}

impl<I, O, E, P> Parser<I, O, E> for Context<P>
where
    P: Parser<I, O, E>,
    E: ContextError<I>,
    I: Clone,
{
    fn parse(&mut self, input: I) -> nom::IResult<I, O, E> {
        self.parser
            .parse(input.clone())
            .map_err(move |err| err.map(move |err| E::add_context(input, self.context, err)))
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Terminated<P1, P2, O2> {
    parser: P1,
    terminator: P2,
    phantom: PhantomData<O2>,
}

impl<I, O1, O2, E, P1, P2> Parser<I, O1, E> for Terminated<P1, P2, O2>
where
    P1: Parser<I, O1, E>,
    P2: Parser<I, O2, E>,
{
    fn parse(&mut self, input: I) -> nom::IResult<I, O1, E> {
        let (input, value) = self.parser.parse(input)?;
        let (input, _) = self.terminator.parse(input)?;

        Ok((input, value))
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Preceded<P1, P2, O2> {
    parser: P1,
    prefix: P2,
    phantom: PhantomData<O2>,
}

impl<I, O1, O2, E, P1, P2> Parser<I, O1, E> for Preceded<P1, P2, O2>
where
    P1: Parser<I, O1, E>,
    P2: Parser<I, O2, E>,
{
    fn parse(&mut self, input: I) -> nom::IResult<I, O1, E> {
        let (input, _) = self.prefix.parse(input)?;
        self.parser.parse(input)
    }
}

pub struct Delimited<L, P, R, O1, O2> {
    prefix: L,
    parser: P,
    suffix: R,

    phantom: PhantomData<(O1, O2)>,
}

impl<I, O, O1, O2, E, L, P, R> Parser<I, O, E> for Delimited<L, P, R, O1, O2>
where
    L: Parser<I, O1, E>,
    P: Parser<I, O, E>,
    R: Parser<I, O2, E>,
{
    fn parse(&mut self, input: I) -> nom::IResult<I, O, E> {
        let (input, _) = self.prefix.parse(input)?;
        let (input, value) = self.parser.parse(input)?;
        let (input, _) = self.suffix.parse(input)?;

        Ok((input, value))
    }
}
