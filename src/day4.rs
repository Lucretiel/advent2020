mod better_part2;

use nom::{
    branch::alt,
    bytes::complete::is_not,
    bytes::complete::tag,
    // bytes::complete::take_while_m_n,
    character::complete::char,
    character::complete::one_of,
    combinator::all_consuming,
    combinator::opt,
    error::ParseError,
    multi::fold_many0,
    sequence::{pair, preceded, terminated},
    IResult,
    Parser,
};

// use crate::common::BoolExt;

fn passport_field<'a, E>(label: &'static str) -> impl Parser<&'a str, &'a str, E>
where
    E: ParseError<&'a str>,
{
    preceded(pair(tag(label), char(':')), is_not(" \t\n\r"))
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Field<'a> {
    BirthYear(&'a str),
    IssueYear(&'a str),
    ExpirationYear(&'a str),
    Height(&'a str),
    HairColor(&'a str),
    EyeColor(&'a str),
    PassportId(&'a str),
    CountryId(&'a str),
}

fn parse_field(input: &str) -> IResult<&str, Field> {
    alt((
        passport_field("byr").map(Field::BirthYear),
        passport_field("iyr").map(Field::IssueYear),
        passport_field("eyr").map(Field::ExpirationYear),
        passport_field("hgt").map(Field::Height),
        passport_field("hcl").map(Field::HairColor),
        passport_field("ecl").map(Field::EyeColor),
        passport_field("pid").map(Field::PassportId),
        passport_field("cid").map(Field::CountryId),
    ))(input)
}

/*
fn parse_color(input: &str) -> IResult<&str, &str> {
    preceded(
        char('#'),
        take_while_m_n(6, 6, |c: char| {
            c.is_ascii_hexdigit() && !c.is_ascii_uppercase()
        }),
    )(input)
}

fn parse_eye_color(input: &str) -> IResult<&str, &str> {
    alt((
        tag("amb"),
        tag("blu"),
        tag("brn"),
        tag("gry"),
        tag("grn"),
        tag("hzl"),
        tag("oth"),
    ))(input)
}
*/

#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct PartialDocument<'a> {
    birth_year: Option<&'a str>,
    issue_year: Option<&'a str>,
    expiration_year: Option<&'a str>,
    height: Option<&'a str>,
    hair_color: Option<&'a str>,
    eye_color: Option<&'a str>,
    passport_id: Option<&'a str>,
    country_id: Option<&'a str>,
}

/*
fn check_year(year: &str, min: u32, max: u32) -> Option<()> {
    let year: u32 = year.parse().ok()?;
    (min <= year && year <= max).then_some(())
}

fn check_height(height: &str) -> Option<()> {
    match (
        height.get(..height.len() - 2)?,
        height.get(height.len() - 2..)?,
    ) {
        (h, "in") => {
            let height: u32 = h.parse().ok()?;
            (59 <= height && height <= 76).then_some(())
        }
        (h, "cm") => {
            let height: u32 = h.parse().ok()?;
            (150 <= height && height <= 193).then_some(())
        }
        _ => None,
    }
}
*/

impl PartialDocument<'_> {
    fn is_mostly_valid(&self) -> bool {
        matches!(
            self,
            PartialDocument {
                birth_year: Some(..),
                issue_year: Some(..),
                expiration_year: Some(..),
                height: Some(..),
                hair_color: Some(..),
                eye_color: Some(..),
                passport_id: Some(..),
                ..
            }
        )
    }

    /*
    fn is_even_more_mostly_valid(&self) -> Option<()> {
        check_year(self.birth_year?, 1920, 2002)?;
        check_year(self.issue_year?, 2010, 2020)?;
        check_year(self.expiration_year?, 2020, 2030)?;
        check_height(self.height?)?;
        parse_color(self.hair_color?).ok()?;
        parse_eye_color(self.eye_color?).ok()?;

        let pid = self.passport_id?;
        (pid.len() == 9).then_some(())?;
        pid.chars().all(|c| c.is_ascii_digit()).then_some(())?;

        Some(())
    }
    */
}

fn parse_document(input: &str) -> IResult<&str, PartialDocument> {
    let parse_field = terminated(parse_field, opt(one_of(" \n")));

    fold_many0(
        parse_field,
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
                Field::CountryId(id) => document.country_id = Some(id),
            }

            document
        },
    )(input)
}

pub fn part1(input: &str) -> anyhow::Result<usize> {
    let count = input
        .split("\n\n")
        .enumerate()
        .filter(|&(idx, line)| {
            let (_, document) = all_consuming(parse_document)(line)
                .unwrap_or_else(|err| panic!("Error parsing document index {}: {}", idx, err));

            document.is_mostly_valid()
        })
        .count();

    Ok(count)
}

/*
pub fn part2(input: &str) -> anyhow::Result<usize> {
    let count = input
        .split("\n\n")
        .enumerate()
        .filter_map(|(idx, line)| {
            let (_, document) = all_consuming(parse_document)(line)
                .unwrap_or_else(|err| panic!("Error parsing document index {}: {}", idx, err));

            document.is_even_more_mostly_valid()
        })
        .count();

    Ok(count)
}
*/

pub use better_part2::part2;
