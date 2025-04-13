use std::cmp::max;
use std::collections::HashSet;
use std::time::Duration;

use lichess_api::LichessChatRoom;
use reqwest::StatusCode;
use tokio_util::sync::CancellationToken;

use anyhow::{Result, anyhow};
use hyperopic::Side;
use hyperopic::constants::side;
use hyperopic::position::Position;

use crate::compute::MoveChooser;
use crate::events::{Clock, GameEvent, GameFull, GameState};
use crate::lichess::LichessService;
use crate::messages;

const STARTED_STATUS: &'static str = "started";
const CREATED_STATUS: &'static str = "created";
const MOVE_LATENCY_MS: u64 = 200;
const MIN_COMPUTE_TIME_MS: u64 = 200;

#[derive(Debug, Clone, Eq, PartialEq)]
struct InferredGameMetadata {
    lambda_side: Side,
    clock: Clock,
}

#[derive(Debug, Clone)]
pub struct GameConfig<M: MoveChooser> {
    pub game_id: String,
    pub bot_id: String,
    pub auth_token: String,
    pub moves: M,
    pub cancel_token: CancellationToken,
}

pub struct Game<M: MoveChooser> {
    bot_id: String,
    inferred_metadata: Option<InferredGameMetadata>,
    lichess: LichessService,
    moves: M,
    position_count: usize,
    cancel_token: CancellationToken,
    states_processed: HashSet<String>,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum GameExecutionState {
    Running,
    Finished,
    Cancelled,
}

impl<M: MoveChooser> From<GameConfig<M>> for Game<M> {
    fn from(conf: GameConfig<M>) -> Self {
        Game {
            lichess: LichessService::new(conf.auth_token, conf.game_id),
            moves: conf.moves,
            bot_id: conf.bot_id,
            inferred_metadata: None,
            position_count: 0,
            cancel_token: conf.cancel_token,
            states_processed: HashSet::default(),
        }
    }
}

impl<M: MoveChooser> Game<M> {
    pub fn halfmove_count(&self) -> usize {
        self.position_count
    }

    pub async fn abort(&self) -> Result<StatusCode> {
        self.lichess.client.abort_game(self.lichess.game_id.as_str()).await
    }

    pub async fn post_introduction(&self) {
        self.post_chat(messages::INTRO, LichessChatRoom::Player).await;
        self.post_chat(messages::INTRO, LichessChatRoom::Spectator).await;
    }

    async fn post_chat(&self, text: &str, room: LichessChatRoom) {
        match self.lichess.client.post_chatline(self.lichess.game_id.as_str(), text, room).await {
            Err(err) => {
                log::warn!("Failed to post chatline {} in {:?}: {}", text, room, err)
            }
            Ok(status) => {
                log::info!("Response status {} for chatline {} in room {:?}", status, text, room)
            }
        }
    }

    pub async fn process_event(&mut self, event_json: &str) -> Result<GameExecutionState> {
        match serde_json::from_str(event_json) {
            Err(error) => {
                log::warn!("Error parsing event {}", error);
                Err(anyhow!("{}", error))
            }
            Ok(event) => {
                log::debug!("{}: {}", self.lichess.game_id, event_json);
                match event {
                    GameEvent::GameFull { content } => self.process_game(content).await,
                    GameEvent::State { content } => self.process_state(content).await,
                    GameEvent::ChatLine { .. } | GameEvent::OpponentGone { .. } => {
                        Ok(GameExecutionState::Running)
                    }
                }
            }
        }
    }

    async fn process_game(&mut self, game: GameFull) -> Result<GameExecutionState> {
        if game.initial_fen.as_str() != "startpos" {
            return Err(anyhow!("Custom start positions not currently supported"));
        }
        // Track info required for playing future gamestates
        self.inferred_metadata = Some(InferredGameMetadata {
            clock: game.clock,
            lambda_side: if self.bot_id == game.white.id {
                log::info!("Detected lambda is playing as white");
                side::W
            } else if self.bot_id == game.black.id {
                log::info!("Detected lambda is playing as black");
                side::B
            } else {
                return Err(anyhow!(
                    "Name not matched, us: {} w: {} b: {}",
                    self.bot_id,
                    game.white.id,
                    game.black.id
                ));
            },
        });
        self.process_state(game.state).await
    }

    async fn process_state(&mut self, state: GameState) -> Result<GameExecutionState> {
        if !self.states_processed.insert(state.moves.clone()) {
            log::warn!("{}: Duplicate game state {}", self.lichess.game_id, state.moves.as_str());
            return Ok(GameExecutionState::Running);
        }
        log::debug!("Parsing previous game moves: {}", state.moves);
        let position = state.moves.parse::<Position>()?;
        let active = position.active;
        let position_count = position.history.len();
        self.position_count = position_count;
        match state.status.as_str() {
            STARTED_STATUS | CREATED_STATUS => {
                let metadata = self.get_latest_metadata()?.clone();
                if active != metadata.lambda_side {
                    log::debug!("It is not our turn, waiting for opponents move");
                    Ok(GameExecutionState::Running)
                } else {
                    let (remaining, increment) = if metadata.lambda_side == side::W {
                        (state.wtime, state.winc)
                    } else {
                        (state.btime, state.binc)
                    };
                    tokio::select! {
                        _ = self.cancel_token.cancelled() => {
                            log::info!("Move selection cancelled!");
                            Ok(GameExecutionState::Cancelled)
                        },
                        computed_move_result = self.moves.choose(
                            state.moves.as_str(),
                            Duration::from_millis(max(MIN_COMPUTE_TIME_MS, remaining - MOVE_LATENCY_MS)),
                            Duration::from_millis(increment)
                        ) => {
                            let m = computed_move_result?;
                            let game_id = self.lichess.game_id.as_str();
                            log::info!("{}: Posting {}", game_id, m);
                            self.lichess.client.post_move(game_id, m.to_string().as_str()).await?;
                            Ok(GameExecutionState::Running)
                        }
                    }
                }
            }
            // All other possibilities indicate the game is over
            status => {
                log::info!("Game finished with status: {}!", status);
                Ok(GameExecutionState::Finished)
            }
        }
    }

    fn get_latest_metadata(&self) -> Result<&InferredGameMetadata> {
        self.inferred_metadata.as_ref().ok_or(anyhow!("Metadata not initialized"))
    }
}
