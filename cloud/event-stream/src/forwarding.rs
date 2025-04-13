use std::convert::Infallible;

use anyhow::Result;
use warp::http::StatusCode;

const CHALLENGE_ENDPOINT: &'static str = "https://lichess.org/api/challenge";

#[derive(Deserialize, Serialize)]
pub struct ChallengeRequest {
    rated: bool,
    #[serde(rename = "clock.limit")]
    clock_limit: usize,
    #[serde(rename = "clock.increment")]
    clock_increment: usize,
    #[serde(default)]
    color: ColourOption,
}

#[derive(Deserialize, Serialize)]
pub enum ColourOption {
    #[serde(rename = "random")]
    Random,
    #[serde(rename = "white")]
    White,
    #[serde(rename = "black")]
    Black,
}
impl Default for ColourOption {
    fn default() -> Self {
        ColourOption::Random
    }
}

pub async fn challenge(
    client: &reqwest::Client,
    auth_token: &str,
    user: String,
    params: ChallengeRequest,
) -> Result<impl warp::Reply + use<>, Infallible> {
    log::info!("Challenging {} with game params {}", user, serde_json::to_string(&params).unwrap());
    //let forward_response = client.post_challenge(user.as_str(), &params).await;
    Ok(match create_challenge(client, auth_token, user, params).await {
        Ok((status, body)) => {
            log::info!("Received Lichess response code:{}, body:{}", status, body);
            warp::reply::with_status(body, status)
        }
        Err(e) => {
            log::error!("Error trying to contact Lichess: {}", e);
            warp::reply::with_status(
                format!("{{\"error\"}}:\"{}\"", e),
                StatusCode::INTERNAL_SERVER_ERROR,
            )
        }
    })
}

async fn create_challenge(
    client: &reqwest::Client,
    auth_token: &str,
    user: String,
    params: ChallengeRequest,
) -> Result<(StatusCode, String)> {
    let response = client
        .post(format!("{}/{}", CHALLENGE_ENDPOINT, user))
        .bearer_auth(auth_token)
        .form(&params)
        .send()
        .await?;
    Ok((response.status(), response.text().await?))
}

//pub async fn post_challenge(
//    &self,
//    username: &str,
//    challenge_params: &ChallengeRequest,
//) -> Result<(StatusCode, String)> {
//    Ok((status, body))
//}
