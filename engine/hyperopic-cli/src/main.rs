mod command;
mod openings;
mod latch;

use crate::command::Command;
use crate::state::{IDLE, SEARCHING, STOPPING};
use anyhow::anyhow;
use clap::Parser;
use hyperopic::constants::side;
use hyperopic::position::Position;
use hyperopic::{ComputeMoveInput, Engine};
use latch::CountDownLatch;
use state::PONDERING;
use std::sync::atomic::Ordering::SeqCst;
use std::sync::atomic::{AtomicBool, AtomicU8};
use std::sync::Arc;

const DEFAULT_TABLE_SIZE: usize = 1_000_000;

#[derive(Parser, Debug, Clone)]
struct Args {
    /// Path to the openings database file to use
    #[clap(long, default_value = None)]
    openings_db: Option<String>,
    /// Table row capacity for the transposition table
    #[clap(long, default_value = None)]
    table_size: Option<usize>,
}

fn main() -> anyhow::Result<()> {
    Hyperopic::new(Args::parse()).run()
}

mod state {
    type EngineState = u8;
    pub const IDLE: EngineState = 0;
    pub const STOPPING: EngineState = 1;
    pub const SEARCHING: EngineState = 2;
    pub const PONDERING: EngineState = 3;
}

struct Hyperopic {
    engine: Engine,
    search_control: Option<Arc<SearchControl>>,
    state: Arc<AtomicU8>,
    position: Position,
}

impl Hyperopic {
    pub fn new(args: Args) -> Self {
        let table_size = args.table_size.unwrap_or(DEFAULT_TABLE_SIZE);
        let lookups = vec![];
        Hyperopic {
            search_control: None,
            engine: Engine::new(table_size, lookups),
            state: Arc::new(AtomicU8::new(IDLE)),
            position: Position::default(),
        }
    }

    pub fn run(&mut self) -> anyhow::Result<()> {
        for input_line in std::io::stdin().lines() {
            match input_line {
                Err(e) => {
                    return Err(anyhow!("Error reading stdin {}", e));
                }
                Ok(line) => match line.as_str().parse::<Command>() {
                    Err(e) => eprintln!("Error parsing command \"{}\": {}", line, e),
                    Ok(command) => {
                        let curr_state = self.state.load(SeqCst);
                        match command {
                            Command::Uci => {
                                println!("id name Hyperopic");
                                println!("id author th0masb");
                                println!("uciok");
                            }
                            Command::IsReady => println!("readyok"),
                            Command::Debug(debug) => {}
                            Command::SetOption(option) => {}
                            Command::Quit => {
                                match curr_state {
                                    SEARCHING | PONDERING | STOPPING => {
                                        let control = self.search_control.as_ref().unwrap();
                                        control.stop_search.store(true, SeqCst);
                                        control.wait_search.join();
                                    },
                                    _ => {},
                                }
                                break
                            }
                            Command::NewGame => {
                                if curr_state == IDLE {
                                    self.engine.reset();
                                }
                            }
                            Command::Ponder => {}
                            Command::PonderHit => {}
                            Command::Position(position) => {
                                self.position = position
                            },
                            Command::Stop => {
                                if curr_state == SEARCHING || curr_state == PONDERING {
                                    self.state.store(STOPPING, SeqCst);
                                    if let Some(control) = self.search_control.as_ref() {
                                        control.stop_search.store(true, SeqCst);
                                    }
                                }
                            }
                            Command::Search { w_time, w_inc, b_time, b_inc } => {
                                if curr_state == IDLE {
                                    let state_holder = self.state.clone();
                                    state_holder.store(SEARCHING, SeqCst);
                                    let next_search_control = Arc::new(SearchControl::new());
                                    self.search_control = Some(next_search_control.clone());
                                    let is_white = self.position.active == side::W;
                                    self.engine.compute_move_async(
                                        ComputeMoveInput {
                                            position: self.position.clone(),
                                            remaining: if is_white { w_time } else { b_time },
                                            increment: if is_white { w_inc } else { b_inc },
                                            stop_flag: Some(next_search_control.stop_search.clone())
                                        },
                                        move |result| {
                                            next_search_control.wait_search.count_down();
                                            state_holder.store(IDLE, SeqCst);
                                            // Print output
                                        },
                                    );
                                }
                            }
                        }
                    }
                },
            }
        }
        Ok(())
    }
}

struct SearchControl {
    stop_search: Arc<AtomicBool>,
    wait_search: Arc<CountDownLatch>,
}

impl SearchControl {
    fn new() -> SearchControl {
        SearchControl {
            stop_search: Arc::new(AtomicBool::new(false)),
            wait_search: Arc::new(CountDownLatch::new(1)),
        }
    }
}

// #[derive(Debug, Clone)]
// enum EngineState {
//     Idle,
//     Pondering,
//     Searching,
// }

#[derive(Debug, Clone)]
enum EngineOpt {}
