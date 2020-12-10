//! The library is a collection of common types, traits, and functions that
//! have come in handy in the past and will probably continue to do so in the
//! future

mod boolext;
pub mod dynamic;
pub mod nom;
mod parse_items;

pub use boolext::BoolExt;
pub use parse_items::{parse_items, parse_items_lines, parse_items_ws};
