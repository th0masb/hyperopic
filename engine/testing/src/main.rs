use anyhow::{Result, anyhow};
use async_trait::async_trait;
use chrono::{DateTime, Datelike, Timelike, Utc};
use clap::Parser;
use hyperopic::Engine;
use hyperopic::openings::OpeningService;
use lazy_static::lazy_static;
use lichess_api::ratings::{ChallengeRequest, OnlineBot, TimeLimitType, TimeLimits};
use lichess_api::{LichessClient, LichessEndgameClient};
use lichess_events::events::{Challenge, GameStart};
use lichess_events::{EventProcessor, LichessEvent, StreamParams};
use lichess_game::{EmptyCancellationHook, Metadata};
use log::LevelFilter;
use openings::{DynamoOpeningClient, OpeningTable};
use rand::prelude::IndexedRandom;
use simple_logger::SimpleLogger;
use std::collections::{HashMap, HashSet};
use std::ops::Range;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc::{Receiver, Sender};
use tokio::time::sleep;

const TABLE_SIZE: usize = 5_000_000;

lazy_static! {
    // Every 10 days we do 2 blitz days, 1 rapid and 7 bullet
    static ref TIME_LIMITS: [TimeLimits; 10] = std::array::from_fn(|i| {
        if i % 5 == 0 {
            TimeLimits { limit: 180, increment: 2 }
        } else if i % 7 == 0 {
            TimeLimits { limit: 300, increment: 5 }
        } else {
            TimeLimits { limit: 120, increment: 1 }
        }
    });
}

#[derive(Parser, Debug)]
struct Args {
    #[arg(long)]
    auth_token: String,
    #[arg(long)]
    rated: bool,
    #[arg(long, default_value_t = 19)]
    start_hour: u32,
    #[arg(long, default_value_t = 7)]
    end_hour: u32,
    #[arg(long, default_value_t = LevelFilter::Info)]
    log_level: LevelFilter,
    #[arg(long, default_value_t = 2)]
    max_concurrent_games: usize,
    #[arg(long, default_value_t = 150)]
    rating_offset_above: u32,
    #[arg(long, default_value_t = 200)]
    rating_offset_below: u32,
    #[arg(long, default_value_t = 5400)]
    flush_interval_secs: u64,
    #[arg(long)]
    time_limit: Option<u32>,
    #[arg(long)]
    time_increment: Option<u32>,
}

#[derive(Debug, Clone, Eq, PartialEq)]
struct GameStarted {
    id: String,
    opponent_id: String,
}

#[tokio::main]
async fn main() {
    let args = Args::parse();
    SimpleLogger::new().with_level(args.log_level).init().unwrap();
    let client = LichessClient::new(args.auth_token.clone());
    let bot_id = client.get_our_profile().await.expect("").id;
    log::info!("Our id is \"{}\"", bot_id.as_str());
    let cloned_id = bot_id.clone();
    let cloned_token = args.auth_token.clone();
    let (tx, rx) = tokio::sync::mpsc::channel::<GameStarted>(32);
    tokio::spawn(async move { run_event_stream(cloned_token, cloned_id, tx).await });
    search_for_game(&args, bot_id.clone(), rx).await;
}

#[derive(Debug, Clone, Default)]
struct BotTracker {
    activity: HashMap<String, i32>,
}

#[derive(Debug, Clone, Default)]
struct RatingRange {
    offset_below: u32,
    offset_above: u32,
}

async fn search_for_game(args: &Args, bot_id: String, mut rx: Receiver<GameStarted>) {
    let client = LichessClient::new(args.auth_token.clone());
    let mut poll_interval = tokio::time::interval(Duration::from_secs(20));
    let mut flush_interval = tokio::time::interval(Duration::from_secs(args.flush_interval_secs));
    let mut tracker = BotTracker::default();
    let mut backoff_index = 0u32;
    loop {
        tokio::select! {
            _ = flush_interval.tick() => {
                log::info!("Flushing bot tracker");
                tracker.activity.clear()
            }
            Some(game_id) = rx.recv() => {
                *tracker.activity.entry(game_id.opponent_id).or_insert(1) -= 1;
            }
            _ = poll_interval.tick() => {
                match execute_challenge_poll(
                    args,
                    &mut tracker,
                    bot_id.as_str(),
                    &client,
                    RatingRange {
                        offset_below: args.rating_offset_below,
                        offset_above: args.rating_offset_above
                    },
                ).await { Err(e) => {
                    log::error!("Error in challenge poll: {}", e);
                    backoff_index += 1;
                    backoff(backoff_index).await;
                } _ => {
                    backoff_index = 0;
                }};
            }
        }
    }
}

async fn backoff(index: u32) {
    let base_wait = Duration::from_secs(120);
    let max_wait = Duration::from_secs(600);
    sleep(std::cmp::min(max_wait, index * base_wait)).await;
}

fn get_active_time_range(args: &Args) -> Vec<Range<DateTime<Utc>>> {
    let (lo, hi) = (args.start_hour, args.end_hour);
    let now: DateTime<Utc> = Utc::now();
    if hi > lo {
        vec![change_time(now, lo, 0, 0)..change_time(now, hi, 0, 0)]
    } else {
        vec![
            change_time(now, 0, 0, 0)..change_time(now, hi, 0, 0),
            change_time(now, lo, 0, 0)..change_time(now, 23, 59, 59),
        ]
    }
}

fn change_time(date_time: DateTime<Utc>, hour: u32, min: u32, sec: u32) -> DateTime<Utc> {
    date_time.with_hour(hour).unwrap().with_minute(min).unwrap().with_second(sec).unwrap()
}

async fn execute_challenge_poll(
    args: &Args,
    tracker: &mut BotTracker,
    bot_id: &str,
    client: &LichessClient,
    rating_range: RatingRange,
) -> Result<()> {
    let now = Utc::now();
    if !get_active_time_range(args).into_iter().any(|r| r.contains(&now)) {
        log::debug!("{} not in active range", now);
        return Ok(());
    }
    let time_limit = choose_time_limits(args);
    let exclusions = vec!["hyperopic", "myopic-bot"];
    let time_limit_type = time_limit.get_type();
    let BotState { rating, online_bots, games_in_progress } =
        fetch_bot_state(bot_id, time_limit_type, client)
            .await
            .map_err(|e| anyhow!("Failed to fetch bot state: {}", e))?;

    if games_in_progress >= args.max_concurrent_games {
        return Ok(());
    } else if !online_bots.iter().any(|b| b.id.as_str() == bot_id) {
        log::warn!("It does not appear that we are online!");
        return Ok(());
    }

    let min_rating = rating - rating_range.offset_below;
    let max_rating = rating + rating_range.offset_above;
    let ratings = min_rating..=max_rating;
    // Only take bots within the acceptable rating range, whose rating is not provisional and who
    // have not violated tos as these bots will be more likely to accept challenges.
    let candidate_bots: Vec<_> = online_bots
        .into_iter()
        .filter(|b| !exclusions.contains(&b.id.as_str()))
        .filter(|b| !b.tos_violation.unwrap_or(false))
        .filter(|b| b.perfs.rating_for(time_limit_type).is_some())
        .filter(|b| ratings.contains(&b.perfs.rating_for(time_limit_type).unwrap().rating))
        .filter(|b| !b.perfs.rating_for(time_limit_type).unwrap().prov.unwrap_or(false))
        .collect();

    log::info!("{} candidate opponents", candidate_bots.len());
    let (tested, untested): (Vec<_>, Vec<_>) =
        candidate_bots.into_iter().partition(|b| tracker.activity.contains_key(&b.id));
    log::info!("{} tested, {} untested", tested.len(), untested.len());
    let (active, inactive): (Vec<_>, Vec<_>) =
        tested.clone().into_iter().partition(|b| tracker.activity[&b.id] == 0);
    log::info!("{} active, {} inactive", active.len(), inactive.len());

    let chosen = if !untested.is_empty() {
        untested
            .iter()
            .max_by_key(|b| b.perfs.rating_for(time_limit_type).unwrap().rating)
            .unwrap()
            .clone()
    } else if !active.is_empty() {
        active.choose(&mut rand::rng()).unwrap().clone()
    } else {
        inactive.into_iter().min_by_key(|b| tracker.activity[&b.id]).unwrap()
    };

    log::info!("Chose opponent: {}", chosen.id.as_str());

    let request =
        ChallengeRequest { rated: args.rated, time_limit, target_user_id: chosen.id.clone() };

    let _ = client
        .create_challenge(request)
        .await
        .map_err(|e| anyhow!("Failed to create challenge {}", e))
        .and_then(|(status, message)| match status.as_u16() {
            200 => Ok(()),
            400 => {
                log::warn!("Failed to create challenge with 400 response {}", message);
                Ok(())
            }
            429 => Err(anyhow!("Failed to create challenge with 429!")),
            _ => Err(anyhow!("Error status {} for challenge creation: {}", status, message)),
        })?;

    *tracker.activity.entry(chosen.id).or_insert(0) += 1;
    Ok(())
}

fn choose_time_limits(args: &Args) -> TimeLimits {
    if args.time_limit.is_some() && args.time_increment.is_some() {
        TimeLimits { limit: args.time_limit.unwrap(), increment: args.time_increment.unwrap() }
    } else {
        let day_of_month = Utc::now().day0() as usize;
        TIME_LIMITS[day_of_month % TIME_LIMITS.len()].clone()
    }
}

async fn fetch_bot_state(
    bot_id: &str,
    time_limit_type: TimeLimitType,
    client: &LichessClient,
) -> Result<BotState> {
    Ok(BotState {
        rating: client
            .fetch_rating(bot_id, time_limit_type)
            .await?
            .map(|r| r.rating)
            .unwrap_or(1500),
        online_bots: client.fetch_online_bots().await?,
        games_in_progress: client.get_our_live_games().await?.now_playing.len(),
    })
}

struct BotState {
    pub rating: u32,
    pub online_bots: Vec<OnlineBot>,
    pub games_in_progress: usize,
}

async fn run_event_stream(auth_token: String, bot_id: String, tx: Sender<GameStarted>) {
    lichess_events::stream(
        StreamParams {
            status_poll_frequency: Duration::from_secs(300),
            max_lifespan: Duration::from_secs(120 * 60 * 60),
            retry_wait: Duration::from_secs(10),
            our_bot_id: bot_id.clone(),
            auth_token: auth_token.clone(),
        },
        EventProcessorImpl {
            our_bot_id: bot_id.clone(),
            auth_token: auth_token.clone(),
            lichess: LichessClient::new(auth_token.clone()),
            games_started: Default::default(),
            table_size: TABLE_SIZE,
            tx,
        },
    )
    .await;
}

fn opening_table() -> OpeningService<DynamoOpeningClient> {
    OpeningTable {
        name: "MyopicOpenings".to_string(),
        region: "eu-west-2".to_string(),
        position_key: "PositionFEN".to_string(),
        move_key: "Moves".to_string(),
        max_depth: 10,
    }
    .try_into()
    .map(|client| OpeningService::new(client))
    .expect("Bad opening table config")
}

struct EventProcessorImpl {
    our_bot_id: String,
    auth_token: String,
    lichess: LichessClient,
    games_started: HashSet<String>,
    table_size: usize,
    tx: Sender<GameStarted>,
}

#[async_trait]
impl EventProcessor for EventProcessorImpl {
    async fn process(&mut self, event: LichessEvent) {
        match event {
            // Decline incoming challenges for now
            LichessEvent::Challenge { challenge: Challenge { id, challenger, .. } } => {
                if challenger.id != self.our_bot_id {
                    log::info!("Declining challenge from {}", challenger.id);
                    self.lichess.post_challenge_response(id.as_str(), "decline").await.ok();
                }
            }
            // Span a new task to play the game if we haven't already done so
            LichessEvent::GameStart { game: GameStart { id, opponent } } => {
                if self.games_started.insert(id.clone()) {
                    let metadata = Metadata {
                        game_id: id,
                        our_bot_id: self.our_bot_id.clone(),
                        auth_token: self.auth_token.clone(),
                    };
                    let engine = Engine::new(
                        self.table_size,
                        vec![Arc::new(opening_table()), Arc::new(LichessEndgameClient::default())],
                    );
                    self.tx
                        .send(GameStarted {
                            id: metadata.game_id.clone(),
                            opponent_id: opponent.id.clone(),
                        })
                        .await
                        .ok();
                    tokio::spawn(async move {
                        let game_id = metadata.game_id.clone();
                        log::info!("Starting game {}", game_id);
                        lichess_game::play(Duration::MAX, engine, metadata, EmptyCancellationHook)
                            .await
                            .map_err(|e| {
                                log::error!("Game id {} failed: {}", game_id, e);
                            })
                            .ok();
                    });
                }
            }
        }
    }
}
