//! Extensions to the nom Parser trait which add postfix versions of the
//! common combinators

use std::{marker::PhantomData, ops::RangeTo};

use nom::{
    combinator::{all_consuming, complete, cut, recognize, verify},
    error::ParseError,
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

    fn cut(self) -> Cut<Self>
    where
        E: ParseError<I>,
    {
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

    fn verify<F>(self, verifier: F)
    where
        F: Fn(&O) -> bool,
        I: Clone;

    fn context(self, context: &'static str);

    fn fill<T>(self, target: &mut [O]);

    fn terminated<F, O2>(self, terminator: F)
    where
        F: Parser<I, O2, E>;

    fn precedes<F, O2>(self, successor: F)
    where
        F: Parser<I, O2, E>;

    fn preceded_by<F, O2>(self, proceeder: F)
    where
        F: Parser<I, O2, E>;
}

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
        all_consuming(move |i| self.parser.parse(i)).parse(input)
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
        complete(move |i| self.parser.parse(i)).parse(input)
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
    E: ParseError<I>,
{
    fn parse(&mut self, input: I) -> nom::IResult<I, O, E> {
        cut(move |i| self.parser.parse(i)).parse(input)
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
            Ok((tail, value)) => Ok(tail, Some(value)),
            Err(NomErr::Error(_)) => Ok(input, None),
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
    E: nom::error::ParseError<I>,
    I: Clone + Slice<RangeTo<usize>> + Offset,
{
    fn parse(&mut self, input: I) -> nom::IResult<I, I, E> {
        recognize(move |i| self.parser.parse(i)).parse(input)
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
    E: ParseError<I>,
    T: Clone,
{
    fn parse(&mut self, input: I) -> nom::IResult<I, T, E> {
        self.parser
            .parse(input)
            .map(move |(tail, _)| (tail, self.value.clone()))
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
    E: nom::error::ParseError<I>,
    F: Fn(&O) -> bool,
    I: Clone,
{
    fn parse(&mut self, input: I) -> nom::IResult<I, O, E> {
        let Verify {
            ref mut parser,
            ref verifier,
        } = *self;

        verify(move |i| parser.parse(i), verifier).parse(input)
    }
}

#[derive(Debug, Clone, Copy)]
struct Context<P> {
    context: &'static str,
    parser: P,
}
