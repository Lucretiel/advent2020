use std::cmp::Ordering;

use anyhow::Context;
use itertools::Itertools;
use num::integer::lcm;

use crate::library::parse_items;

pub fn part1(input: &str) -> anyhow::Result<i64> {
    let mut parts = input.split_whitespace();

    let earliest_departure_time: i64 = parts
        .next()
        .context("No earliest departure time")?
        .parse()
        .context("Failed to parse departure time")?;

    let schedule = parts
        .next()
        .context("No bus schedules")?
        .split(',')
        .filter(|&bus_id| bus_id.chars().all(|c| c.is_numeric()));

    let schedule: Vec<i64> = parse_items(schedule).context("Failed to parse bus schedule")?;

    let (target_departure, bus_id) = schedule
        .iter()
        .map(|&bus_id| {
            let cycles = earliest_departure_time / bus_id;
            let extra_time = earliest_departure_time % bus_id;

            let departure = match extra_time {
                0 => earliest_departure_time,
                _ => (cycles + 1) * bus_id,
            };

            (departure, bus_id)
        })
        .min()
        .context("No busses in the schedule")?;

    Ok((target_departure - earliest_departure_time) * bus_id)
}

#[derive(Debug)]
struct BusDesc {
    period: i64,
    offset: i64,
}

fn advance_candidate(target: i64, candidate: i64, period: i64) -> i64 {
    let difference = target - candidate;

    let steps = difference.div_euclid(period);
    let extra = (difference.rem_euclid(period) != 0) as i64;

    candidate + ((steps + extra) * period)
}

pub fn part2(input: &str) -> anyhow::Result<i64> {
    let solution = input
        .split_whitespace()
        .nth(1)
        .context("No bus schedule found")?
        .split(',')
        .enumerate()
        .filter_map(|(index, bus_id)| {
            bus_id.parse().ok().map(|bus_id| BusDesc {
                period: bus_id,
                offset: index as i64,
            })
        })
        .fold1(|bus1, bus2| {
            let combined_period = lcm(bus1.period, bus2.period);

            let mut candidate1 = -bus1.offset;
            let mut candidate2 = -bus2.offset;

            loop {
                match candidate1.cmp(&candidate2) {
                    Ordering::Equal => {
                        break BusDesc {
                            period: combined_period,
                            offset: combined_period - candidate1,
                        }
                    }
                    Ordering::Less => {
                        candidate1 = advance_candidate(candidate2, candidate1, bus1.period)
                    }
                    Ordering::Greater => {
                        candidate2 = advance_candidate(candidate1, candidate2, bus2.period)
                    }
                }
            }
        })
        .context("No busses in schedule")?;

    Ok(solution.period - solution.offset)
}

/*
7, 13, 17

N |
  N % 7 == 0
  N % 13 == -1

  N' = 71

  N % 91 == -71
  N % 17 == -2

  P-N % P == N''''''

  
*/