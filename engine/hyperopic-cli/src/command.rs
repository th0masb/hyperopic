use std::str::FromStr;
use std::time::Duration;
use crate::EngineOpt;

// See https://gist.github.com/DOBRO/2592c6dad754ba67e6dcaec8c90165bf for a description of
// the UCI interface.
#[derive(Debug, Clone)]
pub enum Command {
    Start,
    IsReady,
    NewGame,
    Ponder,
    PonderHit,
    Stop,
    Quit,
    Debug(bool),
    SetOption(EngineOpt),
    Position(String),
    Search { w_time: Duration, w_inc: Duration, b_time: Duration, b_inc: Duration },
}

impl FromStr for Command {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        todo!()
    }
}