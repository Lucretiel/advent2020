//! Enhanced tag parser for nom

use nom::{Compare, CompareResult, Err as NomErr, IResult, InputLength, InputTake};

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

/// Enhanced tag parser that records the tag in the error in the event of
/// a parse failure via TagError
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

/// Enhanced tag parser that records the tag in the error in the event of
/// a parse failure via TagError
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
