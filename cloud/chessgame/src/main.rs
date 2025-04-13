use std::ops::Sub;
use std::str::FromStr;
use std::time::{Duration, Instant, SystemTime};

use async_trait::async_trait;
use bytes::Bytes;
use lambda_runtime::{Context, Error, LambdaEvent, service_fn};
use rusoto_core::Region;
use rusoto_lambda::{InvocationRequest, InvokeAsyncRequest, Lambda, LambdaClient};
use simple_logger::SimpleLogger;

use anyhow::{Result, anyhow};
use hyperopic::moves::Move;
use hyperopic::position::Position;
use lambda_payloads::chessgame::*;
use lambda_payloads::chessmove::{ChooseMoveEvent, ChooseMoveEventClock, ChooseMoveOutput};
use lichess_game::{CancellationHook, MoveChooser};

const CANCEL_PERIOD_SECS: u64 = 60;

#[tokio::main]
async fn main() -> Result<(), Error> {
    SimpleLogger::new().with_level(log::LevelFilter::Info).without_timestamps().init()?;
    lambda_runtime::run(service_fn(game_handler)).await?;
    Ok(())
}

async fn game_handler(event: LambdaEvent<PlayGameEvent>) -> Result<PlayGameOutput, Error> {
    let e = event.payload;
    let region = Region::from_str(e.move_function_region.as_str())?;
    lichess_game::play(
        compute_wait_until_cancel(&event.context)?,
        MoveLambdaClient::from((region.clone(), e.move_function_name.clone())),
        lichess_game::Metadata {
            game_id: e.lichess_game_id.clone(),
            our_bot_id: e.lichess_bot_id.clone(),
            auth_token: e.lichess_auth_token.clone(),
        },
        RecursionHook {
            client: LambdaClient::new(region),
            payload: e.clone(),
            function_arn: event.context.invoked_function_arn,
        },
    )
    .await
    .map_err(Error::from)
    .map(|m| PlayGameOutput { message: m })
}

fn compute_wait_until_cancel(ctx: &Context) -> Result<Duration, Error> {
    Ok(ctx
        .deadline()
        .sub(Duration::from_secs(CANCEL_PERIOD_SECS))
        .duration_since(SystemTime::now())?)
}

struct RecursionHook {
    client: LambdaClient,
    payload: PlayGameEvent,
    function_arn: String,
}

#[async_trait]
impl CancellationHook for RecursionHook {
    async fn run(&self) -> Result<String> {
        log::info!("Recursively calling this function");
        let mut payload = self.payload.clone();
        payload.current_depth += 1;
        if payload.current_depth >= payload.max_depth {
            Err(anyhow!("Can not recurse any further!"))
        } else {
            let response = self
                .client
                .invoke_async(InvokeAsyncRequest {
                    function_name: self.function_arn.clone(),
                    invoke_args: Bytes::from(serde_json::to_string(&payload)?),
                })
                .await?;

            if let Some(202) = response.status {
                Ok(format!("Successfully continued {}", self.payload.lichess_game_id))
            } else {
                Err(anyhow!(
                    "Recursion status {:?} for game {}",
                    response.status,
                    self.payload.lichess_game_id
                ))
            }
        }
    }
}

pub struct MoveLambdaClient {
    client: LambdaClient,
    function_name: String,
}

impl From<(Region, String)> for MoveLambdaClient {
    fn from((region, name): (Region, String)) -> Self {
        MoveLambdaClient { function_name: name, client: LambdaClient::new(region) }
    }
}

#[async_trait]
impl MoveChooser for MoveLambdaClient {
    async fn choose(
        &mut self,
        moves_played: &str,
        remaining: Duration,
        increment: Duration,
    ) -> Result<Move> {
        let timer = Instant::now();
        let request = ChooseMoveEvent {
            moves_played: moves_played.to_owned(),
            features: vec![],
            clock_millis: ChooseMoveEventClock {
                increment: increment.as_millis() as u64,
                remaining: remaining.as_millis() as u64,
            },
        };
        log::info!("Request payload {:?}", request);
        let response = self
            .client
            .invoke(InvocationRequest {
                function_name: self.function_name.clone(),
                payload: Some(Bytes::from(serde_json::to_string(&request)?)),
                client_context: None,
                invocation_type: None,
                log_type: None,
                qualifier: None,
            })
            .await?;
        log::info!("Response status: {:?}", response.status_code);
        log::info!("Invocation took {}ms", timer.elapsed().as_millis());
        match response.payload {
            None => Err(anyhow!("Missing response payload!")),
            Some(raw_bytes) => {
                let decoded = String::from_utf8(raw_bytes.to_vec())?;
                log::info!("Response payload: {}", decoded);
                let response = serde_json::from_str::<ChooseMoveOutput>(decoded.as_str())?;
                let mut position = moves_played.parse::<Position>()?;
                position
                    .play(&response.best_move)?
                    .first()
                    .cloned()
                    .ok_or(anyhow!("Could not parse {}", response.best_move))
            }
        }
    }
}
