use std::collections::{HashSet, VecDeque};

use anyhow::Context;
use cascade::cascade;
use nom::{
    branch::alt,
    character::complete::{char, digit1, multispace0, multispace1, space1},
    IResult, Parser,
};
use nom_supreme::{
    error::ErrorTree,
    final_parser::{final_parser, Location},
    multi::parse_separated_terminated,
    parser_ext::ParserExt,
    tag::complete::tag,
};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
struct Card {
    rank: usize,
}

fn parse_card(input: &str) -> IResult<&str, Card, ErrorTree<&str>> {
    digit1
        .parse_from_str()
        .map(|rank| Card { rank })
        .context("card")
        .parse(input)
}

#[derive(Debug, Clone, Default, Hash, PartialEq, Eq)]
struct Deck {
    cards: VecDeque<Card>,
}

impl Deck {
    fn peek(&mut self) -> Option<CardPreview> {
        if self.cards.is_empty() {
            None
        } else {
            Some(CardPreview { deck: self })
        }
    }

    fn len(&self) -> usize {
        self.cards.len()
    }

    fn is_empty(&self) -> bool {
        self.cards.is_empty()
    }

    fn add_pair(&mut self, card1: Card, card2: Card) {
        self.cards.push_back(card1);
        self.cards.push_back(card2)
    }

    fn try_clone(&self, count: usize) -> Option<Deck> {
        if self.cards.len() >= count {
            Some(Deck {
                cards: self.cards.iter().take(count).cloned().collect(),
            })
        } else {
            None
        }
    }

    fn score(&self) -> usize {
        self.cards
            .iter()
            .rev()
            .zip(1..)
            .map(|(card, multiply)| card.rank * multiply)
            .sum()
    }
}

#[derive(Debug)]
struct CardPreview<'a> {
    deck: &'a mut Deck,
}

impl CardPreview<'_> {
    fn draw(self) -> Card {
        self.deck.cards.pop_front().unwrap()
    }
}

fn parse_deck(input: &str) -> IResult<&str, Deck, ErrorTree<&str>> {
    parse_separated_terminated(
        parse_card,
        multispace1,
        alt((tag("\n\n"), multispace0.all_consuming())),
        VecDeque::new,
        |cards, card| cascade! {cards; ..push_back(card);},
    )
    .map(|cards| Deck { cards })
    .preceded_by(
        tag("Player")
            .terminated(space1)
            .terminated(digit1)
            .terminated(char(':'))
            .terminated(multispace1),
    )
    .context("deck")
    .parse(input)
}

fn parse_deck_pair(input: &str) -> Result<(Deck, Deck), ErrorTree<Location>> {
    final_parser(
        parse_deck
            .context("player 1")
            .and(parse_deck.context("player 2")),
    )(input)
}

pub fn part1(input: &str) -> anyhow::Result<usize> {
    let (mut deck1, mut deck2) = parse_deck_pair(input).context("Error parsing decks")?;

    while let (Some(card1), Some(card2)) = (deck1.peek(), deck2.peek()) {
        let card1 = card1.draw();
        let card2 = card2.draw();

        if card1 > card2 {
            deck1.add_pair(card1, card2);
        } else {
            deck2.add_pair(card2, card1);
        }
    }

    if deck1.is_empty() {
        Ok(deck2.score())
    } else {
        Ok(deck1.score())
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
enum Player {
    Player1,
    Player2,
}

use Player::*;

/// Returns true if deck 1 wins
fn play_recursive_combat(mut deck1: Deck, mut deck2: Deck) -> (Player, usize) {
    let mut game_states: HashSet<(Deck, Deck)> = HashSet::new();

    game_states.insert((deck1.clone(), deck2.clone()));

    while let (Some(card1), Some(card2)) = (deck1.peek(), deck2.peek()) {
        let card1 = card1.draw();
        let card2 = card2.draw();

        let winner = if deck1.len() >= card1.rank && deck2.len() >= card2.rank {
            let subdeck1 = deck1.try_clone(card1.rank).unwrap();
            let subdeck2 = deck2.try_clone(card2.rank).unwrap();
            play_recursive_combat(subdeck1, subdeck2).0
        } else if card1 > card2 {
            Player1
        } else {
            Player2
        };

        match winner {
            Player1 => deck1.add_pair(card1, card2),
            Player2 => deck2.add_pair(card2, card1),
        }

        if !game_states.insert((deck1.clone(), deck2.clone())) {
            return (Player1, deck1.score());
        }
    }

    if deck1.is_empty() {
        (Player2, deck2.score())
    } else {
        (Player1, deck1.score())
    }
}

pub fn part2(input: &str) -> anyhow::Result<usize> {
    let (deck1, deck2) = parse_deck_pair(input).context("Error parsing decks")?;

    let (_, score) = play_recursive_combat(deck1, deck2);

    Ok(score)
}
