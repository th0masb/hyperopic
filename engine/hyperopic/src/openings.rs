use crate::LookupMoveService;
use crate::moves::Move;
use crate::position::Position;
use anyhow::{Error, Result, anyhow};
use itertools::Itertools;
use std::str::FromStr;

const MOVE_FREQ_SEPARATOR: &'static str = ":";

#[derive(Debug, Clone, PartialOrd, PartialEq)]
pub struct OpeningMoveRecord {
    mv: String,
    freq: u64,
}

pub trait OpeningMoveFetcher {
    fn lookup(&self, position: &Position) -> Result<Vec<OpeningMoveRecord>>;
}

pub struct OpeningService<F: OpeningMoveFetcher> {
    fetcher: F,
}

impl <F: OpeningMoveFetcher> OpeningService<F> {
    pub fn new(fetcher: F) -> Self {
        OpeningService { fetcher }
    }
}

impl<F: OpeningMoveFetcher> LookupMoveService for OpeningService<F> {
    fn lookup(&self, position: Position) -> Result<Option<Move>> {
        let options = self.fetcher.lookup(&position)?;
        if options.len() == 0 {
            return Ok(None);
        }
        let chosen_move = choose_move(&options, rand::random)?;
        let parsed = position.clone().play(chosen_move.mv)?;
        let m =
            parsed.first().cloned().ok_or(anyhow!("{:?} not parsed on {}", options, position))?;
        Ok(Some(m))
    }
}

impl FromStr for OpeningMoveRecord {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let split = s.split(MOVE_FREQ_SEPARATOR).map(|s| s.to_string()).collect::<Vec<_>>();
        Ok(OpeningMoveRecord {
            mv: split.get(0).ok_or(anyhow!("Cannot parse move from {}", s))?.clone(),
            freq: split.get(1).ok_or(anyhow!("Cannot parse freq from {}", s))?.parse()?,
        })
    }
}

fn choose_move(
    available: &Vec<OpeningMoveRecord>,
    f: impl Fn() -> u64,
) -> Result<OpeningMoveRecord> {
    let records = available.iter().sorted_by_key(|r| r.freq).collect::<Vec<_>>();

    let frequency_sum = records.iter().map(|r| r.freq).sum::<u64>();

    if frequency_sum == 0 {
        Err(anyhow!("Freq is 0 for {:?}", available))
    } else {
        let record_choice = f() % frequency_sum;
        let mut sum = 0u64;
        for &record in records.iter() {
            if sum <= record_choice && record_choice < sum + record.freq {
                return Ok(record.clone());
            }
            sum += record.freq;
        }
        panic!("Failed to choose move {:?}", available)
    }
}

#[cfg(test)]
mod test {
    use super::{OpeningMoveRecord, choose_move};

    fn mv(input: &str) -> OpeningMoveRecord {
        input.parse().unwrap()
    }

    #[test]
    fn test_choose_move() {
        let choices = vec![mv("a2a3:1"), mv("b2b4:1"), mv("g8f6:3"), mv("e1g1:20")];

        assert_eq!(mv("a2a3:1"), choose_move(&choices, || { 0 }).unwrap());
        assert_eq!(mv("b2b4:1"), choose_move(&choices, || { 1 }).unwrap());

        for i in 2..5 {
            assert_eq!(mv("g8f6:3"), choose_move(&choices, || { i }).unwrap());
        }

        for i in 5..25 {
            assert_eq!(mv("e1g1:20"), choose_move(&choices, || { i }).unwrap());
        }

        assert_eq!(mv("a2a3:1"), choose_move(&choices, || { 25 }).unwrap());
    }
}
