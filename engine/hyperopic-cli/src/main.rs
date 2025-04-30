mod command;
mod latch;
mod openings;

use crate::command::Command;
use crate::openings::OpeningsDatabase;
use crate::state::{IDLE, SEARCHING, STOPPING};
use anyhow::anyhow;
use anyhow::Result;
use clap::Parser;
use hyperopic::constants::side;
use hyperopic::openings::OpeningService;
use hyperopic::position::Position;
use hyperopic::{ComputeMoveInput, Engine, LookupMoveService};
use latch::CountDownLatch;
use state::PONDERING;
use std::sync::atomic::Ordering::SeqCst;
use std::sync::atomic::{AtomicBool, AtomicU8};
use std::sync::Arc;
use std::time::Duration;
use hyperopic::moves::Move;

const DEFAULT_TABLE_SIZE: usize = 1_000_000;

#[derive(Parser, Debug, Clone)]
struct Args {
    /// Path to the openings database file to use
    #[clap(long, default_value = None)]
    openings_db: Option<String>,
    #[clap(long, default_value = "10")]
    max_openings_depth: usize,
    /// Table row capacity for the transposition table
    #[clap(long, default_value = None)]
    table_size: Option<usize>,
}

fn main() -> Result<()> {
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
    ponder_move: Option<Move>
}

impl Hyperopic {
    pub fn new(args: Args) -> Self {
        let mut lookups: Vec<Arc<dyn LookupMoveService + Send + Sync>> = vec![];
        if let Some(openings_db) = args.openings_db {
            match OpeningsDatabase::new(std::path::PathBuf::from(openings_db.clone())) {
                Err(err) => {
                    eprintln!("Could not open Openings database at {}: {}", openings_db, err)
                }
                Ok(db) => lookups.push(Arc::new(OpeningService {
                    fetcher: db,
                    max_depth: args.max_openings_depth,
                })),
            }
        }
        Hyperopic {
            search_control: None,
            engine: Engine::new(args.table_size.unwrap_or(DEFAULT_TABLE_SIZE), lookups),
            state: Arc::new(AtomicU8::new(IDLE)),
            position: Position::default(),
            ponder_move: None
        }
    }

    pub fn run(&mut self) -> anyhow::Result<()> {
        for input_line in std::io::stdin().lines() {
            match input_line {
                Err(e) => {
                    return Err(anyhow!("Error reading stdin {}", e));
                }
                Ok(line) => match line.as_str().parse::<Command>() {
                    Err(e) => eprintln!("Error parsing \"{}\": {}", line, e),
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
                                    }
                                    _ => {}
                                }
                                break;
                            }
                            Command::NewGame => {
                                if curr_state == IDLE {
                                    self.engine.reset();
                                }
                            }
                            Command::Ponder => {}
                            Command::PonderHit => {}
                            Command::Position(position) => self.position = position,
                            Command::Stop => {
                                if curr_state == SEARCHING || curr_state == PONDERING {
                                    self.state.store(STOPPING, SeqCst);
                                    if let Some(control) = self.search_control.as_ref() {
                                        control.stop_search.store(true, SeqCst);
                                    }
                                }
                            }
                            Command::Search { w_time, w_inc, b_time, b_inc, move_time } => {
                                if curr_state == IDLE {
                                    let state_holder = self.state.clone();
                                    state_holder.store(SEARCHING, SeqCst);
                                    let next_search_control = Arc::new(SearchControl::new());
                                    self.search_control = Some(next_search_control.clone());
                                    let is_white = self.position.active == side::W;
                                    self.engine.compute_move_async(
                                        ComputeMoveInput {
                                            position: self.position.clone(),
                                            remaining: if is_white { w_time } else { b_time }
                                                .unwrap_or(Duration::from_millis(5000)),
                                            increment: if is_white { w_inc } else { b_inc }
                                                .unwrap_or(Duration::ZERO),
                                            max_time: move_time,
                                            stop_flag: Some(
                                                next_search_control.stop_search.clone(),
                                            ),
                                        },
                                        move |result| {
                                            next_search_control.wait_search.count_down();
                                            state_holder.store(IDLE, SeqCst);
                                            match result {
                                                Err(e) => {
                                                    eprintln!("Error computing move: {}", e);
                                                    // Quit with error
                                                }
                                                Ok(output) => {
                                                    //if let Some(ponder_move) = output.search_details.and_then(|details| details.optimal_path.get(1).cloned()) {
                                                    //    println!("")
                                                    //    
                                                    //} else {
                                                    //    println!("bestmove {}", output.best_move);
                                                    //}
                                                    println!("bestmove {}", output.best_move);
                                                }
                                            }
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

#[derive(Debug, Clone, PartialEq)]
enum EngineOpt {}
