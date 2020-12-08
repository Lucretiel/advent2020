//! Originally my day4 part2 solution was a simple variation on part1 (just
//! a stricter validator function). That's icky though so even though I already
//! got the right answer, I'm still reimplementing it here with a strict parser
//! instead of a validator.

use nom::{
    branch::alt,
    bytes::complete::{is_not, take_while_m_n},
    character::complete::{char, digit1, multispace1},
    combinator::{all_consuming, eof, map_opt, map_res, value, verify},
    error::ParseError,
    multi::fold_many0,
    sequence::{pair, preceded, terminated, tuple},
    IResult, Parser,
};

use crate::common::parse_from_str;
use crate::nom_helpers::{final_parser, tag, Location, NomError, TagError};

fn passport_field<'a, E>(label: &'static str) -> impl Parser<&'a str, &'a str, E>
where
    E: ParseError<&'a str> + TagError<&'static str, &'a str>,
{
    preceded(pair(tag(label), char(':')), is_not(" \t\n\r"))
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
enum Height {
    Cm(u32),
    In(u32),
}

fn parse_height(input: &str) -> IResult<&str, Height, NomError<&str>> {
    alt((
        terminated(parse_from_str(digit1), tag("cm")).map(Height::Cm),
        terminated(parse_from_str(digit1), tag("in")).map(Height::In),
    ))(input)
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Default)]
struct Color {
    red: u8,
    green: u8,
    blue: u8,
}

fn parse_hex_u8(input: &str) -> IResult<&str, u8, NomError<&str>> {
    map_res(
        take_while_m_n(2, 2, |c: char| {
            c.is_ascii_hexdigit() && !c.is_ascii_uppercase()
        }),
        |s| u8::from_str_radix(s, 16),
    )(input)
}

fn parse_color(input: &str) -> IResult<&str, Color, NomError<&str>> {
    preceded(char('#'), tuple((parse_hex_u8, parse_hex_u8, parse_hex_u8)))
        .map(|(red, green, blue)| Color { red, green, blue })
        .parse(input)
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
enum EyeColor {
    Amber,
    Blue,
    Brown,
    Green,
    Grey,
    Hazel,
    Other,
}

fn parse_eye_color(input: &str) -> IResult<&str, EyeColor, NomError<&str>> {
    alt((
        value(EyeColor::Amber, tag("amb")),
        value(EyeColor::Blue, tag("blu")),
        value(EyeColor::Brown, tag("brn")),
        value(EyeColor::Grey, tag("gry")),
        value(EyeColor::Green, tag("grn")),
        value(EyeColor::Hazel, tag("hzl")),
        value(EyeColor::Other, tag("oth")),
    ))(input)
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
struct PassportId(u32);

fn parse_passport_id(input: &str) -> IResult<&str, PassportId, NomError<&str>> {
    parse_from_str(verify(digit1, |d: &str| d.len() == 9))
        .map(PassportId)
        .parse(input)
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Field {
    BirthYear(u32),
    IssueYear(u32),
    ExpirationYear(u32),
    Height(Height),
    HairColor(Color),
    EyeColor(EyeColor),
    PassportId(PassportId),
    CountryId(()),
}

fn parse_field(input: &str) -> IResult<&str, Field, NomError<&str>> {
    alt((
        parse_from_str(verify(passport_field("byr"), |s: &str| s.len() == 4)).map(Field::BirthYear),
        parse_from_str(verify(passport_field("iyr"), |s: &str| s.len() == 4)).map(Field::IssueYear),
        parse_from_str(verify(passport_field("eyr"), |s: &str| s.len() == 4))
            .map(Field::ExpirationYear),
        passport_field("hgt")
            .and_then(all_consuming(parse_height))
            .map(Field::Height),
        passport_field("hcl")
            .and_then(all_consuming(parse_color))
            .map(Field::HairColor),
        passport_field("ecl")
            .and_then(all_consuming(parse_eye_color))
            .map(Field::EyeColor),
        passport_field("pid")
            .and_then(all_consuming(parse_passport_id))
            .map(Field::PassportId),
        value((), passport_field("cid")).map(Field::CountryId),
    ))
    .parse(input)
}

fn bounds_check<T: Eq + Ord + Copy>(min: T, value: T, max: T) -> bool {
    min <= value && value <= max
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct PartialDocument {
    birth_year: Option<u32>,
    issue_year: Option<u32>,
    expiration_year: Option<u32>,

    height: Option<Height>,
    hair_color: Option<Color>,
    eye_color: Option<EyeColor>,

    passport_id: Option<PassportId>,
}

impl PartialDocument {
    fn validate(self) -> Option<Document> {
        Some(Document {
            birth_year: self
                .birth_year
                .filter(|&year| bounds_check(1920, year, 2008))?,
            issue_year: self
                .issue_year
                .filter(|&year| bounds_check(2010, year, 2020))?,
            expiration_year: self
                .expiration_year
                .filter(|&year| bounds_check(2020, year, 2030))?,
            height: self.height.filter(|&height| match height {
                Height::Cm(h) => bounds_check(150, h, 193),
                Height::In(h) => bounds_check(59, h, 76),
            })?,
            hair_color: self.hair_color?,
            eye_color: self.eye_color?,
            passport_id: self.passport_id?,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Document {
    birth_year: u32,
    issue_year: u32,
    expiration_year: u32,

    height: Height,
    hair_color: Color,
    eye_color: EyeColor,

    passport_id: PassportId,
}

fn parse_document(input: &str) -> Result<Document, NomError<Location>> {
    final_parser(map_opt(
        fold_many0(
            terminated(parse_field, alt((multispace1, eof))),
            PartialDocument::default(),
            |mut document, field| {
                match field {
                    Field::BirthYear(year) => document.birth_year = Some(year),
                    Field::IssueYear(year) => document.issue_year = Some(year),
                    Field::ExpirationYear(year) => document.expiration_year = Some(year),
                    Field::Height(height) => document.height = Some(height),
                    Field::HairColor(color) => document.hair_color = Some(color),
                    Field::EyeColor(color) => document.eye_color = Some(color),
                    Field::PassportId(id) => document.passport_id = Some(id),
                    Field::CountryId(()) => {}
                };
                document
            },
        ),
        |partial| partial.validate(),
    ))(input)
}

pub fn part2(input: &str) -> anyhow::Result<usize> {
    Ok(input
        .split("\n\n")
        .filter_map(|batch| parse_document(batch).ok())
        .count())
}
