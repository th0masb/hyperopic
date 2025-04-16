mod openings;
mod command;

use anyhow::anyhow;
use hyperopic::Engine;
use clap::Parser;
use crate::command::Command;

const DEFAULT_TABLE_SIZE: usize = 1_000_000;

#[derive(Parser, Debug, Clone)]
struct Args {
    /// Path to the openings database file to use
    #[clap(long, default_value = None)]
    openings_db: Option<String>,
    /// Table row capacity for the transposition table
    #[clap(long, default_value = None)]
    table_size: Option<usize>
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    let table_size = args.table_size.unwrap_or(DEFAULT_TABLE_SIZE);
    let lookups = vec![];
    let engine = Engine::new(table_size, lookups);

    for input_line in std::io::stdin().lines() {
        match input_line {
            Err(e) => {
                return Err(anyhow!("Error reading stdin {}", e));
            }
            Ok(line) => match line.as_str().parse::<Command>() {
                Err(e) => eprintln!("Error parsing command \"{}\": {}", line, e),
                Ok(command) => {
                    match command {
                        Command::Quit => {
                            // If currently searching stop, join and then break
                        },
                        Command::IsReady => println!("readyok"),
                        Command::Start => {},
                        Command::NewGame => {},
                        Command::Debug(debug) => {},
                        Command::SetOption(option) => {},
                        Command::Stop => {},
                        Command::Ponder => {},
                        Command::PonderHit => {},
                        Command::Position(position) => {},
                        Command::Search { w_time, w_inc, b_time, b_inc} => {},
                    }
                }
            },
        }
    }
    Ok(())
}

#[derive(Debug, Clone)]
enum EngineState {
    Idle,
    Pondering,
    Searching,
}

#[derive(Debug, Clone)]
enum EngineOpt {}
