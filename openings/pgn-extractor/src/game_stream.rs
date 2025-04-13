use itertools::Itertools;
use regex::Regex;
use std::fs::File;
use std::io;
use std::io::{BufRead, BufReader, Lines};
use std::str::FromStr;

pub struct GameStream {
    inner: Lines<BufReader<File>>,
}

impl GameStream {
    pub fn new(f: File) -> GameStream {
        GameStream { inner: BufReader::new(f).lines() }
    }
}

impl Iterator for GameStream {
    type Item = io::Result<String>;

    fn next(&mut self) -> Option<Self::Item> {
        let mut line = self.inner.next();

        // Fast forward to next game start/eof/read error
        while match &line {
            None => false,
            Some(line_result) => match line_result {
                Err(_) => false,
                Ok(line) => !game_start().is_match(line.as_str()),
            },
        } {
            line = self.inner.next();
        }

        match line {
            // If no new game start line then terminate stream
            None => None,
            Some(game_start_result) => {
                match game_start_result {
                    // If there was a read error then forward it
                    Err(_) => Some(game_start_result),
                    // Otherwise we found the start of a game, now we collect the
                    // continuation lines together
                    Ok(game_start) => {
                        let mut game_components: Vec<String> = vec![game_start];
                        while let Some(game_component_result) = self.inner.next() {
                            match game_component_result {
                                // Again if there is a read error just forward it
                                Err(_) => return Some(game_component_result),
                                Ok(game_component) => {
                                    if game_continuation().is_match(game_component.as_str()) {
                                        game_components.push(game_component)
                                    } else {
                                        break;
                                    }
                                }
                            }
                        }
                        Some(Ok(game_components.iter().join(" ")))
                    }
                }
            }
        }
    }
}

pub fn game_start() -> &'static Regex {
    lazy_static! {
        static ref RE: Regex = Regex::from_str(r"^1[.].*$").unwrap();
    }
    &RE
}

pub fn game_continuation() -> &'static Regex {
    lazy_static! {
        static ref RE: Regex = Regex::from_str(r"^((1[0-9]+)|([2-9][0-9]*))[.].*$").unwrap();
    }
    &RE
}

#[cfg(test)]
mod test {
    use crate::game_stream::GameStream;
    use std::env;
    use std::fs::File;

    const RELATIVE_RESOURCE_PATH: &'static str = "resources/test";

    #[test]
    fn game_start_regex() {
        let under_test = super::game_start();
        assert!(under_test.is_match("1.c4 Nf6 2.d4 g6 3.Nc3 Bg7 4.e4 d6 5.f4 O-O 6.Nf3 e5 7.fxe5"));
        assert!(!under_test.is_match("16.a3 f5 17.b4 Na6 18.c5 Qf6 19.Qxf6"));
        assert!(!under_test.is_match("[Event \"New York\"]"));
    }

    #[test]
    fn game_continuation_regex() {
        let under_test = super::game_continuation();
        assert!(under_test.is_match("9.Bd3 Nc5 10.Bc2 a5 11.O-O Qd6 12.Qe1 Bd7 13.Qh4 Rae8"));
        assert!(under_test.is_match("16.a3 f5 17.b4 Na6 18.c5 Qf6 19.Qxf6"));
        assert!(
            !under_test.is_match("1.c4 Nf6 2.d4 g6 3.Nc3 Bg7 4.e4 d6 5.f4 O-O 6.Nf3 e5 7.fxe5")
        );
        assert!(!under_test.is_match("[Event \"New York\"]"));
    }

    fn file_path(name: &str) -> String {
        format!("{}/{}/{}", env::var("CARGO_MANIFEST_DIR").unwrap(), RELATIVE_RESOURCE_PATH, name)
    }

    #[test]
    fn single_game_pgn() {
        let games: Vec<String> = GameStream::new(File::open(file_path("single-game.pgn")).unwrap())
            .map(|result| result.unwrap())
            .collect();

        assert_eq!(
            vec!["1.e4 e5 2.Nf3 Nc6 3.Bb5 Nf6 4.O-O Bc5 9.Qf3 Nxf2 10.Qxf2 Qd6 11.Bf4 Qd5"],
            games
        )
    }

    #[test]
    fn multi_game_pgn() {
        let games: Vec<String> = GameStream::new(File::open(file_path("multi-game.pgn")).unwrap())
            .map(|result| result.unwrap())
            .collect();

        assert_eq!(
            vec![
                "1.e4 e5 2.Nf3 Nc6 3.Bb5 Nf6 4.d3 Bd6 5.c3 h6 9.Bc4 Bc5 10.b4 Ba7 11.Nh4 d5 12.exd5 Nxd5",
                "1.e4 e5 2.Nf3 Nc6 3.Bb5 Nf6 4.d4 Nxd4 5.Nxd4 exd4 9.Be2 d5 10.exd6 Bxd6 11.Qxd4 Bf5 12.Nc3 O-O-O"
            ],
            games
        )
    }
}
