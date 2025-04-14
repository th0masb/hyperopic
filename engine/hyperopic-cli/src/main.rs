use std::time::Duration;

fn main() {
    // We need one thread polling stdin, parsing command and writing to channel for processing
    // One coordinator thread processing the commands and writing output
    // All other threads assigned to engine computation
    for input_line in std::io::stdin().lines() {
        match input_line {
            Ok(line) => match line.as_str() {
                "exit" => break,
                _ => println!("{}", line),
            },
            Err(e) => {
                eprintln!("{}", e);
                break;
            }
        }
    }
}

#[derive(Debug, Clone)]
enum EngineState {
    Idle,
    Pondering,
    Searching,
}

// See https://gist.github.com/DOBRO/2592c6dad754ba67e6dcaec8c90165bf for a description of
// the UCI interface.
#[derive(Debug, Clone)]
enum UciCommand {
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

#[derive(Debug, Clone)]
enum EngineOpt {}
