use std::{
    collections::{HashMap, HashSet},
    fmt::Display,
};

use anyhow::Context;
use cascade::cascade;
use itertools::Itertools;
use joinery::prelude::*;
use nom::{
    branch::alt,
    character::complete::{alpha1, char, multispace1, space0, space1},
    IResult, Parser,
};
use nom_supreme::{
    error::ErrorTree,
    final_parser::{final_parser, Location},
    multi::parse_separated_terminated,
    parser_ext::ParserExt,
    tag::complete::tag,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct Ingredient {
    name: &'static str,
}

impl Display for Ingredient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.name.fmt(f)
    }
}

fn parse_ingredient(
    input: &'static str,
) -> IResult<&'static str, Ingredient, ErrorTree<&'static str>> {
    alpha1
        .map(|name| Ingredient { name })
        .context("ingredient")
        .parse(input)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
struct Allergen {
    name: &'static str,
}

fn parse_allergen(input: &'static str) -> IResult<&'static str, Allergen, ErrorTree<&'static str>> {
    alpha1
        .map(|name| Allergen { name })
        .context("allergen")
        .parse(input)
}

struct Recipe {
    ingredients: HashSet<Ingredient>,
    known_allergens: HashSet<Allergen>,
}

fn parse_recipe(input: &'static str) -> IResult<&'static str, Recipe, ErrorTree<&'static str>> {
    let parse_allergen_list = parse_separated_terminated(
        parse_allergen,
        tag(", "),
        char(')').peek(),
        HashSet::new,
        |vec, item| cascade! {vec; ..insert(item);},
    )
    .preceded_by(tag("(contains "))
    .terminated(char(')'))
    .context("allergens list");

    parse_separated_terminated(
        parse_ingredient,
        space1,
        space1.terminated(alt((char('('), char('\n')))).peek(),
        HashSet::new,
        |vec, item| cascade! {vec; ..insert(item);},
    )
    .context("ingredients list")
    .terminated(space0)
    .and(parse_allergen_list.opt())
    .map(|(ingredients, allergens)| Recipe {
        ingredients,
        known_allergens: allergens.unwrap_or_default(),
    })
    .context("recipe")
    .parse(input)
}

fn parse_all_recipes(input: &'static str) -> Result<Vec<Recipe>, ErrorTree<Location>> {
    final_parser(
        parse_separated_terminated(
            parse_recipe,
            multispace1,
            multispace1.all_consuming(),
            Vec::new,
            |vec, item| cascade! {vec; ..push(item);},
        )
        .context("recipes list"),
    )(input)
}

fn compute_allergens(recipes: &[Recipe]) -> anyhow::Result<HashMap<Ingredient, Allergen>> {
    let all_allergens: HashSet<Allergen> = recipes
        .iter()
        .flat_map(|r| r.known_allergens.iter())
        .copied()
        .collect();

    let mut allergen_candidate_map: HashMap<Allergen, HashSet<Ingredient>> = HashMap::new();

    for &allergen in &all_allergens {
        let mut relevant_recipes = recipes
            .iter()
            .filter(|r| r.known_allergens.contains(&allergen));

        let candidate_ingredients: HashSet<Ingredient> =
            relevant_recipes.next().unwrap().ingredients.clone();

        let candidate_ingredients = relevant_recipes
            .fold(candidate_ingredients, |ingredients, recipe| {
                &ingredients & &recipe.ingredients
            });

        allergen_candidate_map.insert(allergen, candidate_ingredients);
    }

    let mut allergen_map: HashMap<Ingredient, Allergen> =
        HashMap::with_capacity(allergen_candidate_map.len());

    for _ in 0..allergen_candidate_map.len() {
        let (allergen, ingredient) = allergen_candidate_map
            .iter()
            .find_map(|(&allergen, candidates)| {
                candidates
                    .iter()
                    .exactly_one()
                    .ok()
                    .map(|&ingredient| (allergen, ingredient))
            })
            .context("No unique solution")?;

        // This ingredient is known, remove it as a candidate
        allergen_candidate_map.values_mut().for_each(|candidates| {
            candidates.remove(&ingredient);
        });

        allergen_map.insert(ingredient, allergen);
    }

    Ok(allergen_map)
}

pub fn part1(input: &'static str) -> anyhow::Result<usize> {
    let recipes = parse_all_recipes(input).context("Failed to parse all recipes")?;
    let allergen_map = compute_allergens(&recipes).context("Failed to compute allergens")?;

    let instances_of_safe_ingredient = recipes
        .iter()
        .flat_map(|r| r.ingredients.iter())
        .filter(|&ingredient| !allergen_map.contains_key(ingredient))
        .count();

    Ok(instances_of_safe_ingredient)
}

pub fn part2(input: &'static str) -> anyhow::Result<impl Display> {
    let recipes = parse_all_recipes(input).context("Failed to parse all recipes")?;
    let allergen_map = compute_allergens(&recipes).context("Failed to compute allergens")?;

    let mut ingredients: Vec<Ingredient> = allergen_map.keys().copied().collect();
    ingredients.sort_unstable_by_key(|ingredient| allergen_map.get(ingredient));

    Ok(ingredients.join_with(","))
}
