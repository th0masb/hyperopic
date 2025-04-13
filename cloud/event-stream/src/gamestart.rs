use std::str::FromStr;

use anyhow::{Error, Result, anyhow};
use lichess_api::LichessClient;
use lichess_events::events::GameStart;
use rusoto_core::Region;
use rusoto_lambda::{InvokeAsyncRequest, Lambda, LambdaClient};

use crate::challenge_table::ChallengeTableClient;
use crate::config::AppConfig;

pub struct GameStartService {
    client: LichessClient,
    invoker: LambdaInvoker,
    challenge_table: ChallengeTableClient,
    our_id: String,
}

impl GameStartService {
    pub fn new(parameters: &AppConfig) -> GameStartService {
        GameStartService {
            client: LichessClient::new(parameters.lichess_bot.auth_token.clone()),
            invoker: LambdaInvoker::new(parameters.clone()),
            challenge_table: ChallengeTableClient::new(&parameters.rate_limits.challenge_table),
            our_id: parameters.lichess_bot.bot_id.to_lowercase(),
        }
    }

    pub async fn process_event(&self, event: GameStart) -> Result<String> {
        let (game_id, mut challenger_id) = (event.id.as_str(), event.opponent.id.as_str());
        log::info!("Processing GameStart {} against {}", game_id, challenger_id);
        let table_client = &self.challenge_table;

        if table_client.get_entry(challenger_id, game_id).await?.is_none() {
            challenger_id = self.our_id.as_str();
        }

        if table_client.get_entry(challenger_id, game_id).await?.is_none() {
            return Err(anyhow!("No challenge entry found for {}", game_id));
        }

        if table_client.set_started(challenger_id, game_id).await? {
            log::info!("Lambda for {}/{} should be invoked", challenger_id, game_id);
            match self.invoker.trigger_lambda(game_id).await {
                Err(e) => Err(anyhow!(
                    "Unable to trigger lambda: {}, abort status: {:?}",
                    e,
                    self.client.abort_game(game_id).await
                )),
                Ok(status) => match status {
                    None => Err(anyhow!(
                        "No status for lambda invocation for {}, abort status: {:?}",
                        game_id,
                        self.client.abort_game(game_id).await
                    )),
                    Some(n) => {
                        if n == 202 {
                            Ok(format!("Lambda successfully queued for game {}", game_id))
                        } else {
                            Err(anyhow!(
                                "{} status for lambda invocation for {}, abort status: {:?}",
                                n,
                                game_id,
                                self.client.abort_game(game_id).await
                            ))
                        }
                    }
                },
            }
        } else {
            Ok(format!("Lambda for {}/{} was already invoked", challenger_id, game_id))
        }
    }
}

struct LambdaInvoker {
    params: AppConfig,
    client: LambdaClient,
}

impl LambdaInvoker {
    fn new(params: AppConfig) -> LambdaInvoker {
        let region = Region::from_str(params.game_function.id.region.as_str()).unwrap();
        LambdaInvoker { client: LambdaClient::new(region), params }
    }

    async fn trigger_lambda(&self, game_id: &str) -> Result<Option<i64>> {
        let payload = crate::config::extract_game_lambda_payload(&self.params, game_id);
        let request = InvokeAsyncRequest {
            function_name: self.params.game_function.id.name.clone(),
            invoke_args: bytes::Bytes::from(payload),
        };
        self.client.invoke_async(request).await.map(|response| response.status).map_err(Error::from)
    }
}
