use anyhow::anyhow;
use hyperopic::position::Position;
use lazy_static::lazy_static;
use regex::{Captures, Regex};
use std::fmt::{Display, Formatter};
use std::str::FromStr;
use std::time::Duration;

// See https://gist.github.com/DOBRO/2592c6dad754ba67e6dcaec8c90165bf for a description of
// the UCI interface.
#[derive(Debug, Clone, PartialEq)]
pub enum Command {
    Uci,
    IsReady,
    NewGame,
    PonderHit,
    Stop,
    Quit,
    Debug(bool),
    Position(Position),
    Search(SearchParams),
}

impl Display for Command {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        if let Command::Position(pos) = self {
            write!(f, "Position({})", pos)
        } else {
            write!(f, "{:?}", self)
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct SearchParams {
    pub w_time: Option<Duration>,
    pub w_inc: Option<Duration>,
    pub b_time: Option<Duration>,
    pub b_inc: Option<Duration>,
    pub move_time: Option<Duration>,
    pub ponder: bool,
}

lazy_static! {
    static ref UCI: Regex = r"^\s*uci\s*$".parse().unwrap();
    static ref DEBUG: Regex = r"^\s*debug\s+(?<value>on|off)\s*$".parse().unwrap();
    static ref ISREADY: Regex = r"^\s*isready\s*$".parse().unwrap();
    static ref NEW_GAME: Regex = r"^\s*ucinewgame\s*$".parse().unwrap();
    static ref STOP: Regex = r"^\s*stop\s*$".parse().unwrap();
    static ref QUIT: Regex = r"^\s*quit\s*$".parse().unwrap();
    static ref SEARCH: Regex = r"\s*go\s+(?<params>.+)".parse().unwrap();
    static ref WTIME: Regex = r"wtime\s+(?<val>\d+)".parse().unwrap();
    static ref BTIME: Regex = r"btime\s+(?<val>\d+)".parse().unwrap();
    static ref WINC: Regex = r"winc\s+(?<val>\d+)".parse().unwrap();
    static ref BINC: Regex = r"binc\s+(?<val>\d+)".parse().unwrap();
    static ref PONDER: Regex = r"ponder".parse().unwrap();
    static ref PONDERHIT: Regex = r"\s*ponderhit\s*".parse().unwrap();
    static ref MOVETIME: Regex = r"movetime\s+(?<val>\d+)".parse().unwrap();
    static ref POSITION: Regex =
        r"^\s*position\s+((fen\s+(?<fen>[^m]+))|(startpos))\s*(moves\s+(?<moves>.+))?$"
            .parse()
            .unwrap();
}

impl FromStr for Command {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if let Some(_) = UCI.captures(s) {
            Ok(Command::Uci)
        } else if let Some(caps) = DEBUG.captures(s) {
            Ok(Command::Debug(&caps["value"] == "on"))
        } else if let Some(_) = ISREADY.captures(s) {
            Ok(Command::IsReady)
        } else if let Some(_) = NEW_GAME.captures(s) {
            Ok(Command::NewGame)
        } else if let Some(_) = STOP.captures(s) {
            Ok(Command::Stop)
        } else if let Some(_) = QUIT.captures(s) {
            Ok(Command::Quit)
        } else if let Some(_) = PONDERHIT.captures(s) {
            Ok(Command::PonderHit)
        } else if let Some(caps) = POSITION.captures(s) {
            let mut pos = if let Some(fen) = caps.name("fen") {
                fen.as_str().parse::<Position>()?
            } else {
                Position::default()
            };
            if let Some(moves) = caps.name("moves") {
                pos.play(moves.as_str())?;
            }
            Ok(Command::Position(pos))
        } else if let Some(caps) = SEARCH.captures(s) {
            let params = caps.name("params").unwrap().as_str();
            Ok(Command::Search(SearchParams {
                w_time: WTIME.captures(params).extract_duration("val"),
                w_inc: WINC.captures(params).extract_duration("val"),
                b_time: BTIME.captures(params).extract_duration("val"),
                b_inc: BINC.captures(params).extract_duration("val"),
                move_time: MOVETIME.captures(params).extract_duration("val"),
                ponder: PONDER.captures(params).is_some(),
            }))
        } else {
            Err(anyhow!("Unrecognized command"))
        }
    }
}

trait UciCaptures {
    fn extract_duration(&self, name: &str) -> Option<Duration>;
}

impl UciCaptures for Captures<'_> {
    fn extract_duration(&self, name: &str) -> Option<Duration> {
        self.name(name).map(|m| Duration::from_millis(m.as_str().parse::<u64>().unwrap()))
    }
}

impl UciCaptures for Option<Captures<'_>> {
    fn extract_duration(&self, name: &str) -> Option<Duration> {
        self.as_ref().and_then(|caps| caps.extract_duration(name))
    }
}

#[cfg(test)]
mod test {
    use super::*;

    fn assert_command_position(expected_fen: &str, input: &str) {
        let mut expected = expected_fen.parse::<Position>().unwrap();
        let actual = input.parse::<Command>().unwrap();
        if let Command::Position(pos) = &actual {
            expected.history = pos.history.clone();
        }
        assert_eq!(Command::Position(expected), actual);
    }

    #[test]
    fn uci() {
        assert_eq!(Command::Uci, " uci\t".parse().unwrap());
    }

    #[test]
    fn debug_on() {
        assert_eq!(Command::Debug(true), " debug  on".parse().unwrap());
    }

    #[test]
    fn debug_off() {
        assert_eq!(Command::Debug(false), " debug\toff".parse().unwrap());
    }

    #[test]
    fn new_game() {
        assert_eq!(Command::NewGame, "  ucinewgame\t ".parse().unwrap());
    }

    #[test]
    fn start_pos_1() {
        assert_eq!(Command::Position(Position::default()), "position startpos".parse().unwrap());
    }

    #[test]
    fn start_pos_2() {
        assert_eq!(Command::Position(Position::default()), " position startpos\t".parse().unwrap());
    }

    #[test]
    fn start_pos_3() {
        assert_command_position(
            "r1bqkbnr/pppppppp/2n5/8/4P3/5N2/PPPP1PPP/RNBQKB1R b KQkq - 2 2",
            " position  startpos  moves \t e2e4 b8c6\tg1f3 ",
        )
    }

    #[test]
    fn fen_start_1() {
        assert_command_position(
            "r1bqk1nr/pppp1ppp/2n5/4p3/1b2P3/1P3N2/P1PP1PPP/RNBQKB1R w KQkq - 1 4",
            " position fen r1bqkbnr/pppppppp/2n5/8/4P3/5N2/PPPP1PPP/RNBQKB1R b KQkq - 2 2  moves e7e5 b2b3 f8b4",
        )
    }

    #[test]
    fn search_1() {
        assert_eq!(
            Command::Search(SearchParams {
                w_time: Some(Duration::from_millis(2319)),
                w_inc: Some(Duration::from_millis(32)),
                b_time: Some(Duration::from_millis(2212)),
                b_inc: Some(Duration::from_millis(890)),
                move_time: None,
                ponder: false,
            }),
            " go\t btime  2212 wtime 2319 winc 32  binc 890 \t".parse().unwrap()
        );
    }

    #[test]
    fn search_2() {
        assert_eq!(
            Command::Search(SearchParams {
                w_time: Some(Duration::from_millis(2319)),
                w_inc: Some(Duration::from_millis(32)),
                b_time: None,
                b_inc: Some(Duration::from_millis(890)),
                move_time: None,
                ponder: false,
            }),
            " go\t wtime 2319 winc 32  binc 890 \t".parse().unwrap()
        );
    }

    #[test]
    fn search_3() {
        assert_eq!(
            Command::Search(SearchParams {
                w_time: Some(Duration::from_millis(2319)),
                w_inc: Some(Duration::from_millis(32)),
                b_time: None,
                b_inc: Some(Duration::from_millis(890)),
                move_time: None,
                ponder: true,
            }),
            " go\t wtime 2319 winc 32  ponder binc 890 \t".parse().unwrap()
        );
    }

    #[test]
    fn ponderhit() {
        assert_eq!(Command::PonderHit, " ponderhit\t".parse().unwrap());
    }
}
