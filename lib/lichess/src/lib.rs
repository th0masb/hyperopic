mod endings;
pub mod ratings;

use crate::ratings::{
    ChallengeRequest, OnlineBot, TimeLimitType, UserDetails, UserDetailsGamePerf,
};
use anyhow::{Error, Result, anyhow};
pub use endings::LichessEndgameClient;
use reqwest::StatusCode;
use serde_derive::Deserialize;
use std::collections::HashMap;

const GAME_ENDPOINT: &'static str = "https://lichess.org/api/bot/game";
const CHALLENGE_ENDPOINT: &'static str = "https://lichess.org/api/challenge";
const ACCOUNT_ENDPOINT: &'static str = "https://lichess.org/api/account";

pub struct LichessClient {
    auth_token: String,
    client: reqwest::Client,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum LichessChatRoom {
    Player,
    Spectator,
}

impl LichessClient {
    pub fn new(auth_token: String) -> LichessClient {
        LichessClient { auth_token, client: reqwest::Client::new() }
    }

    pub async fn get_our_profile(&self) -> Result<Account> {
        let response = self
            .client
            .get(ACCOUNT_ENDPOINT)
            .bearer_auth(self.auth_token.as_str())
            .send()
            .await
            .map_err(Error::from)?;
        response.json().await.map_err(Error::from)
    }

    pub async fn post_challenge_response(
        &self,
        challenge_id: &str,
        decision: &str,
    ) -> Result<StatusCode> {
        self.client
            .post(format!("{}/{}/{}", CHALLENGE_ENDPOINT, challenge_id, decision).as_str())
            .bearer_auth(&self.auth_token)
            .send()
            .await
            .map(|response| response.status())
            .map_err(Error::from)
    }

    pub async fn abort_game(&self, game_id: &str) -> Result<StatusCode> {
        self.client
            .post(format!("{}/{}/abort", GAME_ENDPOINT, game_id).as_str())
            .bearer_auth(&self.auth_token)
            .send()
            .await
            .map_err(|error| anyhow!("Error aborting game {}: {}", game_id, error))
            .map(|response| response.status())
    }

    pub async fn post_move(&self, game_id: &str, mv: &str) -> Result<StatusCode> {
        // Add timeout and retry logic
        let response = self
            .client
            .post(format!("{}/{}/move/{}", GAME_ENDPOINT, game_id, mv).as_str())
            .bearer_auth(&self.auth_token)
            .send()
            .await
            .map_err(|error| anyhow!("Error posting move: {}", error))?;

        let status = response.status();
        if status.is_success() {
            Ok(status)
        } else {
            let body =
                response.text().await.map_err(|e| anyhow!("Failed to get error body {}", e))?;
            Err(anyhow!("Error posting move {} in {}: {} -> {}", mv, game_id, status, body))
        }
    }

    pub async fn post_chatline(
        &self,
        game_id: &str,
        text: &str,
        room: LichessChatRoom,
    ) -> Result<StatusCode> {
        let mut params = HashMap::new();
        params.insert(
            "room",
            match room {
                LichessChatRoom::Player => "player",
                LichessChatRoom::Spectator => "spectator",
            },
        );
        params.insert("text", text);
        self.client
            .post(format!("{}/{}/chat", GAME_ENDPOINT, game_id).as_str())
            .bearer_auth(&self.auth_token)
            .form(&params)
            .send()
            .await
            .map_err(|error| anyhow!("Error posting chatline: {}", error))
            .map(|response| response.status())
    }

    pub async fn create_challenge(
        &self,
        request: ChallengeRequest,
    ) -> Result<(StatusCode, String)> {
        let mut params: HashMap<&str, String> = HashMap::new();
        params.insert("rated", request.rated.to_string());
        params.insert("clock.limit", request.time_limit.limit.to_string());
        params.insert("clock.increment", request.time_limit.increment.to_string());
        let response = self
            .client
            .post(format!("https://lichess.org/api/challenge/{}", request.target_user_id))
            .bearer_auth(self.auth_token.as_str())
            .form(&params)
            .send()
            .await
            .map_err(|e| anyhow!("Error challenging {}: {}", request.target_user_id, e))?;
        let status = response.status();
        let text =
            response.text().await.map_err(|e| anyhow!("Could not get response text: {}", e))?;
        Ok((status, text))
    }

    pub async fn fetch_rating(
        &self,
        user_id: &str,
        time_limit_type: TimeLimitType,
    ) -> Result<Option<UserDetailsGamePerf>, Error> {
        Ok(self
            .client
            .get(format!("https://lichess.org/api/user/{}", user_id))
            .send()
            .await?
            .json::<UserDetails>()
            .await?
            .perfs
            .rating_for(time_limit_type))
    }

    pub async fn fetch_online_bots(&self) -> Result<Vec<OnlineBot>> {
        self.client
            .get(format!("https://lichess.org/api/bot/online"))
            .send()
            .await?
            .text()
            .await?
            .split('\n')
            .map(|line| line.trim())
            .filter(|line| !line.is_empty())
            .map(|s| serde_json::from_str::<OnlineBot>(s).map_err(|e| anyhow!(e)))
            .collect()
    }

    pub async fn get_our_live_games(&self) -> Result<OngoingGames> {
        let response = self
            .client
            .get("https://lichess.org/api/account/playing")
            .bearer_auth(self.auth_token.as_str())
            .send()
            .await
            .map_err(Error::from)?;
        response.json().await.map_err(Error::from)
    }
}

#[derive(Deserialize, Debug, Clone, Eq, PartialEq)]
pub struct Account {
    pub id: String,
}

#[derive(Deserialize, Debug, Clone, Eq, PartialEq)]
struct GameFull {
    clock: Clock,
}

#[derive(Deserialize, Debug, Clone, Eq, PartialEq)]
struct Clock {
    #[serde(rename = "initial")]
    initial_millis: u32,
    #[serde(rename = "increment")]
    increment_millis: u32,
}

#[derive(Deserialize, Debug, Clone, Eq, PartialEq)]
pub struct OngoingGames {
    #[serde(rename = "nowPlaying")]
    pub now_playing: Vec<OngoingGame>,
}

#[derive(Deserialize, Debug, Clone, Eq, PartialEq)]
pub struct OngoingGame {
    #[serde(rename = "gameId")]
    pub game_id: String,
}
