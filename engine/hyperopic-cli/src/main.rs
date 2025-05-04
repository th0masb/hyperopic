mod command;
mod latch;
mod openings;

use crate::command::{Command, SearchParams};
use crate::openings::OpeningsDatabase;
use crate::state::{IDLE, SEARCHING, STOPPING};
use anyhow::Result;
use anyhow::anyhow;
use clap::Parser;
use hyperopic::constants::side;
use hyperopic::openings::OpeningService;
use hyperopic::position::Position;
use hyperopic::search::end::SearchEndSignal;
use hyperopic::timing::TimeAllocator;
use hyperopic::{ComputeMoveInput, ComputeMoveOutput, Engine, LookupMoveService};
use latch::CountDownLatch;
use log::{debug, error, info};
use state::PONDERING;
use std::cmp::max;
use std::sync::Arc;
use std::sync::atomic::Ordering::SeqCst;
use std::sync::atomic::{AtomicU8, Ordering};
use std::time::{Duration, Instant};

const DEFAULT_TABLE_SIZE: usize = 1_000_000;
const ONE_YEAR_IN_SECS: u64 = 60 * 60 * 24 * 365;

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
    #[clap(long, default_value = None)]
    log_config: Option<String>,
}

fn main() -> Result<()> {
    let args = Args::parse();
    if let Some(log_config) = args.log_config.as_ref() {
        log4rs::init_file(log_config, Default::default())?;
    }
    info!("Starting hyperopic CLI");
    Hyperopic::new(args).run()
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
    ponderhit_search_duration: Option<Duration>,
}

impl Hyperopic {
    pub fn new(args: Args) -> Self {
        let mut lookups: Vec<Arc<dyn LookupMoveService + Send + Sync>> = vec![];
        if let Some(openings_db) = args.openings_db {
            match OpeningsDatabase::new(std::path::PathBuf::from(openings_db.clone())) {
                Err(err) => {
                    error!("Could not open Openings database at {}: {}", openings_db, err)
                }
                Ok(db) => {
                    info!("Loaded openings from {}", openings_db);
                    lookups.push(Arc::new(OpeningService {
                        fetcher: db,
                        max_depth: args.max_openings_depth,
                    }))
                }
            }
        }
        Hyperopic {
            search_control: None,
            engine: Engine::new(args.table_size.unwrap_or(DEFAULT_TABLE_SIZE), lookups),
            state: Arc::new(AtomicU8::new(IDLE)),
            position: Position::default(),
            ponderhit_search_duration: None,
        }
    }

    pub fn run(&mut self) -> Result<()> {
        for input_line in std::io::stdin().lines() {
            match input_line {
                Err(e) => {
                    return Err(anyhow!("Error reading stdin {}", e));
                }
                Ok(line) => {
                    info!("Received command input: \"{}\"", line);
                    match line.as_str().parse::<Command>() {
                        Err(e) => error!("Error parsing \"{}\": {}", line, e),
                        Ok(command) => {
                            let curr_state = self.state.load(SeqCst);
                            debug!("In state {} processing command {:?}", curr_state, command);
                            match command {
                                Command::Uci => {
                                    println!("id name Hyperopic");
                                    println!("id author th0masb");
                                    println!("uciok");
                                }
                                Command::IsReady => println!("readyok"),
                                Command::Debug(_) => {}
                                Command::Quit => {
                                    match curr_state {
                                        SEARCHING | PONDERING | STOPPING => {
                                            let control = self.search_control.as_ref().unwrap();
                                            control.stop_search.count_down();
                                            control.wait_search.register_join().recv()?;
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
                                Command::PonderHit => {
                                    if curr_state == PONDERING {
                                        debug!("Received ponderhit command while pondering");
                                        let search_duration =
                                            self.ponderhit_search_duration.unwrap();
                                        let control = self.search_control.as_ref().unwrap().clone();
                                        std::thread::spawn(move || {
                                            debug!(
                                                "PonderHit wait started for {:?}",
                                                search_duration
                                            );
                                            std::thread::sleep(search_duration);
                                            debug!("Stopping search after PonderHit");
                                            control.stop_search.count_down()
                                        });
                                        self.ponderhit_search_duration = None;
                                        self.state.store(SEARCHING, SeqCst);
                                    }
                                }
                                // Need to handle position string during pondering
                                Command::Position(position) => self.position = position,
                                Command::Stop => {
                                    if curr_state == SEARCHING || curr_state == PONDERING {
                                        self.state.store(STOPPING, SeqCst);
                                        self.ponderhit_search_duration = None;
                                        if let Some(control) = self.search_control.as_ref() {
                                            debug!("Stopping search after Stop");
                                            control.stop_search.count_down();
                                        }
                                    }
                                }
                                Command::Search(params) => {
                                    if curr_state == IDLE {
                                        let state_holder = self.state.clone();
                                        state_holder.store(
                                            if params.ponder { PONDERING } else { SEARCHING },
                                            SeqCst,
                                        );
                                        let next_search_control =
                                            Arc::new(SearchControl::default());
                                        self.search_control = Some(next_search_control.clone());
                                        let mut search_duration =
                                            self.compute_search_duration(&params);
                                        if params.ponder {
                                            self.ponderhit_search_duration = Some(search_duration);
                                            search_duration = Duration::from_secs(ONE_YEAR_IN_SECS)
                                        }
                                        let stop_instant = Instant::now() + search_duration;
                                        self.engine.compute_move_async(
                                            ComputeMoveInput {
                                                position: self.position.clone(),
                                                max_depth: None,
                                                wait_for_end: params.ponder,
                                                search_end: GoSearchEnd {
                                                    stop_latch: next_search_control
                                                        .stop_search
                                                        .clone(),
                                                    stop_instant,
                                                },
                                            },
                                            move |result| {
                                                state_holder.store(IDLE, SeqCst);
                                                next_search_control.wait_search.count_down();
                                                match result {
                                                    Err(e) => {
                                                        eprintln!("Error computing move: {}", e)
                                                    }
                                                    Ok(output) => format_output(output),
                                                }
                                            },
                                        );
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        Ok(())
    }

    fn compute_search_duration(&self, params: &SearchParams) -> Duration {
        let is_white = self.position.active == side::W;
        TimeAllocator::default().allocate(
            self.position.history.len(),
            if is_white { params.w_time } else { params.b_time }
                .unwrap_or(Duration::from_millis(5000)),
            if is_white { params.w_inc } else { params.b_inc }.unwrap_or(Duration::ZERO),
        )
    }
}

fn format_output(output: ComputeMoveOutput) {
    if let Some(details) = output.search_details.as_ref() {
        // TODO Handle score output better
        let score_cp = (details.relative_eval as f64 / 2.3).round() as i32;
        let search_info = format!(
            "info depth {} time {} score cp {}",
            details.depth,
            details.time.as_millis(),
            score_cp
        );
        info!("{}", search_info);
        println!("{}", search_info);
    }
    println!(
        "bestmove {}{}",
        output.best_move,
        output
            .search_details
            .as_ref()
            .and_then(|details| details.optimal_path.get(1).cloned())
            .map(|m| format!(" ponder {}", m))
            .unwrap_or("".to_string())
    );
}

#[derive(Clone)]
struct GoSearchEnd {
    stop_instant: Instant,
    stop_latch: Arc<CountDownLatch>,
}

impl SearchEndSignal for GoSearchEnd {
    fn should_end_now(&self) -> bool {
        self.stop_instant.should_end_now()
            || self.stop_latch.get_current_count(Ordering::Relaxed) == 0
    }

    fn join(&self) -> () {
        let duration_until_stop = max(Duration::ZERO, self.stop_instant - Instant::now());
        self.stop_latch.register_join().recv_timeout(duration_until_stop).ok();
    }
}

struct SearchControl {
    /// Stop the current search by counting down once
    stop_search: Arc<CountDownLatch>,
    /// Join this latch to wait for search completion
    wait_search: Arc<CountDownLatch>,
}

impl Default for SearchControl {
    fn default() -> Self {
        SearchControl {
            stop_search: Arc::new(CountDownLatch::new(1)),
            wait_search: Arc::new(CountDownLatch::new(1)),
        }
    }
}
