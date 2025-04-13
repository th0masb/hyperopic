use std::time::{Duration, Instant};

use anyhow::{Result, anyhow};
use async_trait::async_trait;
use reqwest::Response;
use tokio_util::sync::CancellationToken;

pub use cancel::{CancellationHook, EmptyCancellationHook};
pub use compute::MoveChooser;
use response_stream::{LoopAction, StreamHandler};

use crate::game::{Game, GameConfig, GameExecutionState};

mod cancel;
mod compute;
mod events;
mod game;
mod lichess;
mod messages;

const GAME_STREAM_ENDPOINT: &'static str = "https://lichess.org/api/bot/game/stream";

#[derive(Debug, Clone)]
pub struct Metadata {
    pub game_id: String,
    pub our_bot_id: String,
    pub auth_token: String,
}

pub async fn play<M, C>(
    cancel_after: Duration,
    moves: M,
    metadata: Metadata,
    on_cancellation: C,
) -> Result<String>
where
    M: MoveChooser + Send + Sync,
    C: CancellationHook,
{
    let token = CancellationToken::new();
    let cloned_token = token.clone();

    tokio::spawn(async move {
        log::info!("Cancelling in {}s", cancel_after.as_secs());
        tokio::time::sleep(cancel_after).await;
        log::info!("Cancelling current lambda invocation");
        cloned_token.cancel();
    });

    let game = init_game(
        moves,
        metadata.game_id.clone(),
        metadata.our_bot_id.clone(),
        metadata.auth_token.clone(),
        token.child_token(),
    )?;

    game.post_introduction().await;

    log::info!("{}: Initializing game loop", metadata.game_id);
    let mut handler = GameStreamHandler {
        game,
        start: Instant::now(),
        max_wait: Duration::from_secs(30),
        cancel: token.child_token(),
    };
    let game_stream = open_game_stream(&metadata.game_id, &metadata.auth_token).await?;
    match response_stream::handle(game_stream, &mut handler).await? {
        None => Err(anyhow!("{}: Game stream ended unexpectedly!", metadata.game_id)),
        Some(CompletionType::GameFinished) => Ok(format!("{}: Game completed", metadata.game_id)),
        Some(CompletionType::Cancelled) => on_cancellation.run().await,
    }
}

struct GameStreamHandler<M: MoveChooser> {
    game: Game<M>,
    start: Instant,
    max_wait: Duration,
    cancel: CancellationToken,
}

enum CompletionType {
    Cancelled,
    GameFinished,
}

#[async_trait]
impl<M: MoveChooser + Send + Sync> StreamHandler<CompletionType> for GameStreamHandler<M> {
    async fn handle(&mut self, line: String) -> Result<LoopAction<CompletionType>> {
        log::debug!("Stream heartbeat");
        if self.cancel.is_cancelled() {
            log::info!("Cancellation detected! Breaking from game stream");
            return Ok(LoopAction::Break(CompletionType::Cancelled));
        }
        if line.trim().is_empty() {
            if self.game.halfmove_count() < 2 && self.start.elapsed() > self.max_wait {
                let abort_status = self.game.abort().await?;
                if abort_status.is_success() {
                    log::info!("Successfully aborted game due to inactivity!");
                    Ok(LoopAction::Break(CompletionType::GameFinished))
                } else {
                    Err(anyhow!("Failed to abort game, lichess status: {}", abort_status))
                }
            } else {
                Ok(LoopAction::Continue)
            }
        } else {
            log::debug!("Received event: {}", line);
            Ok(match self.game.process_event(line.as_str()).await? {
                GameExecutionState::Running => LoopAction::Continue,
                GameExecutionState::Finished => LoopAction::Break(CompletionType::GameFinished),
                GameExecutionState::Cancelled => LoopAction::Break(CompletionType::Cancelled),
            })
        }
    }
}

fn init_game<M: MoveChooser>(
    moves: M,
    game_id: String,
    bot_id: String,
    auth_token: String,
    cancel_token: CancellationToken,
) -> Result<Game<M>> {
    Ok(GameConfig { game_id, bot_id, auth_token, moves, cancel_token }.into())
}

async fn open_game_stream(game_id: &String, auth_token: &String) -> Result<Response> {
    reqwest::Client::new()
        .get(format!("{}/{}", GAME_STREAM_ENDPOINT, game_id).as_str())
        .bearer_auth(auth_token)
        .send()
        .await
        .map_err(|e| anyhow!(e))
}
